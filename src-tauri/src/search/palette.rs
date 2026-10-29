//! What the command palette finds for the typed text: issues by key and full text, and filter lists by name.

use std::collections::HashMap;

use rusqlite::{Connection, params};
use serde::Serialize;
use ts_rs::TS;

use super::fts::{KeyHint, key_hint, match_expression};
use crate::db::Db;
use crate::error::AppResult;
use crate::status_style::{Style, detect, styles};
use crate::views::fold;

const ISSUE_LIMIT: usize = 8;
const LIST_LIMIT: usize = 6;
/// Marks a match in `highlight()` and `snippet()` output; control characters never occur in Jira text.
const OPEN: char = '\u{1}';
const CLOSE: char = '\u{2}';

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct PaletteResults {
    pub issues: Vec<IssueHit>,
    pub lists: Vec<ListHit>,
}

/// Text with its matches marked; the page bolds the `strong` parts.
#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TextPart {
    pub text: String,
    pub strong: bool,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum MatchedIn {
    Key,
    Summary,
    Description,
    Comment,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct IssueHit {
    pub id: String,
    pub key: String,
    /// Opening the hit shows it in this project's Space.
    pub project_id: String,
    pub summary: Vec<TextPart>,
    pub style: Style,
    pub status_name: String,
    pub matched_in: MatchedIn,
    /// The words around the match when only a comment matched.
    pub comment: Option<Vec<TextPart>>,
    /// Deleted in Jira or no longer visible.
    pub gone: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ListHit {
    pub id: i64,
    pub name: String,
    /// The Space's project for a Space or one of its lists; none for Everything and Home lists.
    pub project_id: Option<String>,
    /// The Space itself rather than one of its lists.
    pub is_space: bool,
}

pub async fn palette_query(db: &Db, text: String) -> AppResult<PaletteResults> {
    db.read(move |conn| query(conn, &text)).await
}

pub fn query(conn: &Connection, text: &str) -> AppResult<PaletteResults> {
    let styles = styles(conn)?;
    let mut issues: Vec<IssueHit> = Vec::new();
    for id in key_matches(conn, text)? {
        if let Some(hit) = plain_hit(conn, &styles, &id)? {
            issues.push(hit);
        }
    }
    if let Some(expression) = match_expression(text) {
        for hit in text_matches(conn, &styles, &expression)? {
            if issues.len() >= ISSUE_LIMIT {
                break;
            }
            if !issues.iter().any(|known| known.id == hit.id) {
                issues.push(hit);
            }
        }
    }
    issues.truncate(ISSUE_LIMIT);
    Ok(PaletteResults {
        issues,
        lists: list_matches(conn, text)?,
    })
}

const TRACKED: &str = "i.project_id IN (SELECT project_id FROM space WHERE tracked = 1)";

/// A whole key first; a bare number matches that number in every tracked project.
fn key_matches(conn: &Connection, text: &str) -> AppResult<Vec<String>> {
    let (sql, value) = match key_hint(text) {
        Some(KeyHint::Key(key)) => ("upper(i.key) = ?1", key),
        Some(KeyHint::Number(number)) => ("i.key LIKE '%-' || ?1", number),
        None => return Ok(Vec::new()),
    };
    let mut query = conn.prepare(&format!(
        "SELECT i.id FROM cache.issue i WHERE {sql} AND {TRACKED} ORDER BY i.gone, i.key LIMIT {ISSUE_LIMIT}"
    ))?;
    let ids = query.query_map([value], |r| r.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
}

fn style_of(styles: &HashMap<String, Style>, status_id: &str, name: &str, category: &str) -> Style {
    styles
        .get(status_id)
        .copied()
        .unwrap_or_else(|| detect(category, name))
}

fn plain_hit(
    conn: &Connection,
    styles: &HashMap<String, Style>,
    id: &str,
) -> AppResult<Option<IssueHit>> {
    let mut query = conn.prepare(
        "SELECT i.key, i.summary, i.status_id, COALESCE(st.name, ''), i.status_category, i.gone, i.project_id
         FROM cache.issue i LEFT JOIN cache.status st ON st.id = i.status_id WHERE i.id = ?1",
    )?;
    let mut rows = query.query([id])?;
    let Some(r) = rows.next()? else {
        return Ok(None);
    };
    let status_id: String = r.get(2)?;
    let status_name: String = r.get(3)?;
    let category: String = r.get(4)?;
    Ok(Some(IssueHit {
        id: id.to_string(),
        key: r.get(0)?,
        summary: vec![TextPart {
            text: r.get(1)?,
            strong: false,
        }],
        style: style_of(styles, &status_id, &status_name, &category),
        status_name,
        matched_in: MatchedIn::Key,
        comment: None,
        gone: r.get::<_, i64>(5)? != 0,
        project_id: r.get(6)?,
    }))
}

/// Key and summary weigh most; the best ones first.
fn text_matches(
    conn: &Connection,
    styles: &HashMap<String, Style>,
    expression: &str,
) -> AppResult<Vec<IssueHit>> {
    let mut query = conn.prepare(&format!(
        "SELECT i.id, i.key,
           highlight(issue_fts, 0, ?2, ?3), highlight(issue_fts, 1, ?2, ?3), highlight(issue_fts, 2, ?2, ?3),
           snippet(issue_fts, 3, ?2, ?3, '…', 12),
           i.status_id, COALESCE(st.name, ''), i.status_category, i.gone, i.project_id
         FROM cache.issue_fts
         JOIN cache.issue i ON i.id = CAST(issue_fts.rowid AS TEXT)
         LEFT JOIN cache.status st ON st.id = i.status_id
         WHERE issue_fts MATCH ?1 AND {TRACKED}
         ORDER BY bm25(issue_fts, 10.0, 5.0, 1.0, 1.0)
         LIMIT {ISSUE_LIMIT}"
    ))?;
    let rows = query.query_map(
        params![expression, OPEN.to_string(), CLOSE.to_string()],
        |r| {
            let key: String = r.get(2)?;
            let summary: String = r.get(3)?;
            let description: Option<String> = r.get(4)?;
            let comments: Option<String> = r.get(5)?;
            let status_id: String = r.get(6)?;
            let status_name: String = r.get(7)?;
            let category: String = r.get(8)?;
            let marked = |text: &str| text.contains(OPEN);
            let matched_in = if marked(&key) {
                MatchedIn::Key
            } else if marked(&summary) {
                MatchedIn::Summary
            } else if description.as_deref().is_some_and(marked) {
                MatchedIn::Description
            } else {
                MatchedIn::Comment
            };
            Ok(IssueHit {
                id: r.get(0)?,
                key: r.get(1)?,
                summary: parts(&summary),
                style: style_of(styles, &status_id, &status_name, &category),
                status_name,
                comment: (matched_in == MatchedIn::Comment)
                    .then(|| comments.as_deref().map(parts))
                    .flatten(),
                matched_in,
                gone: r.get::<_, i64>(9)? != 0,
                project_id: r.get(10)?,
            })
        },
    )?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Splits marked text into plain and matched parts.
fn parts(marked: &str) -> Vec<TextPart> {
    let mut out = Vec::new();
    let mut strong = false;
    let mut current = String::new();
    for c in marked.chars() {
        if c == OPEN || c == CLOSE {
            if !current.is_empty() {
                out.push(TextPart {
                    text: std::mem::take(&mut current),
                    strong,
                });
            }
            strong = c == OPEN;
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        out.push(TextPart {
            text: current,
            strong,
        });
    }
    out
}

/// Filter lists and Spaces whose name holds the text, ignoring case and accents.
fn list_matches(conn: &Connection, text: &str) -> AppResult<Vec<ListHit>> {
    let needle = fold(text.trim());
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let mut query = conn.prepare(
        "SELECT l.id, COALESCE(s.name, p.name, l.name), COALESCE(l.project_id, parent.project_id), l.kind = 'space'
         FROM filter_list l
         LEFT JOIN filter_list parent ON parent.id = l.parent_id
         LEFT JOIN space s ON s.project_id = l.project_id
         LEFT JOIN cache.space p ON p.id = l.project_id
         WHERE COALESCE(l.project_id, parent.project_id) IS NULL
            OR COALESCE(l.project_id, parent.project_id) IN (SELECT project_id FROM space WHERE tracked = 1)
         ORDER BY l.kind = 'space' DESC, l.position, l.id",
    )?;
    let rows = query.query_map([], |r| {
        Ok(ListHit {
            id: r.get(0)?,
            name: r.get(1)?,
            project_id: r.get(2)?,
            is_space: r.get(3)?,
        })
    })?;
    let mut hits = Vec::new();
    for hit in rows {
        let hit = hit?;
        if fold(&hit.name).contains(&needle) {
            hits.push(hit);
        }
        if hits.len() == LIST_LIMIT {
            break;
        }
    }
    Ok(hits)
}
