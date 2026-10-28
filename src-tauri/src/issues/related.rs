//! The issues around one issue: its children (an epic's issues or a task's subtasks) and its links.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use crate::db::Db;
use crate::error::AppResult;
use crate::lists::Viewer;
use crate::status_style::{Style, detect, styles};
use crate::views::rows::{self, Person};

/// One issue as the Child issues and Links sections list it.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct RelatedIssue {
    /// None for a linked issue the cache doesn't hold.
    pub id: Option<String>,
    pub key: String,
    pub summary: String,
    pub style: Style,
    pub issue_type: String,
    pub assignee: Option<Person>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Children {
    /// Not done yet, by status then key.
    pub open: Vec<RelatedIssue>,
    pub done: Vec<RelatedIssue>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct IssueLink {
    /// Jira's words for this side of the link ("blocks", "is blocked by").
    pub relation: String,
    pub issue: RelatedIssue,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Related {
    pub children: Children,
    pub links: Vec<IssueLink>,
}

pub async fn issue_related(db: &Db, viewer: Viewer, id: String) -> AppResult<Related> {
    db.read(move |conn| {
        let me = viewer.me.as_deref();
        Ok(Related {
            children: children(conn, &id, me)?,
            links: links(conn, &id, me)?,
        })
    })
    .await
}

const RELATED: &str =
    "SELECT i.id, i.key, i.summary, i.status_id, COALESCE(st.name, ''), i.status_category,
        i.issue_type, i.assignee_id, u.name
    FROM cache.issue i
    LEFT JOIN cache.status st ON st.id = i.status_id
    LEFT JOIN cache.user u ON u.account_id = i.assignee_id";

fn related_from(
    r: &rusqlite::Row,
    styles: &HashMap<String, Style>,
    me: Option<&str>,
) -> rusqlite::Result<RelatedIssue> {
    let status_id: String = r.get(3)?;
    let status_name: String = r.get(4)?;
    let category: String = r.get(5)?;
    let assignee: Option<String> = r.get(7)?;
    let name: Option<String> = r.get(8)?;
    Ok(RelatedIssue {
        id: Some(r.get(0)?),
        key: r.get(1)?,
        summary: r.get(2)?,
        style: styles
            .get(&status_id)
            .copied()
            .unwrap_or_else(|| detect(&category, &status_name)),
        issue_type: r.get(6)?,
        assignee: assignee.map(|a| rows::person(&a, name.as_deref().unwrap_or(&a), me)),
    })
}

fn children(conn: &Connection, id: &str, me: Option<&str>) -> AppResult<Children> {
    let styles = styles(conn)?;
    let mut query = conn.prepare(&format!("{RELATED} WHERE i.parent_id = ?1 AND i.gone = 0"))?;
    let all = query
        .query_map([id], |r| related_from(r, &styles, me))?
        .collect::<Result<Vec<_>, _>>()?;
    let (mut done, mut open): (Vec<_>, Vec<_>) =
        all.into_iter().partition(|c| c.style == Style::Done);
    open.sort_by(|a, b| {
        a.style
            .cmp(&b.style)
            .then_with(|| key_number(&a.key).cmp(&key_number(&b.key)))
    });
    done.sort_by_key(|c| key_number(&c.key));
    Ok(Children { open, done })
}

fn key_number(key: &str) -> u64 {
    key.rsplit_once('-')
        .and_then(|(_, n)| n.parse().ok())
        .unwrap_or(u64::MAX)
}

/// From the issue's raw `issuelinks`; a linked issue the cache holds shows its cached status.
fn links(conn: &Connection, id: &str, me: Option<&str>) -> AppResult<Vec<IssueLink>> {
    let raw: Option<String> = conn
        .query_row("SELECT raw FROM cache.issue WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    let raw: Value = raw
        .and_then(|r| serde_json::from_str(&r).ok())
        .unwrap_or(Value::Null);
    let styles = styles(conn)?;
    let mut links = Vec::new();
    for link in raw["fields"]["issuelinks"].as_array().into_iter().flatten() {
        let (side, words) = if link["outwardIssue"].is_object() {
            (&link["outwardIssue"], &link["type"]["outward"])
        } else {
            (&link["inwardIssue"], &link["type"]["inward"])
        };
        let (Some(key), Some(relation)) = (side["key"].as_str(), words.as_str()) else {
            continue;
        };
        let cached = related_by_key(conn, key, &styles, me)?;
        let fallback = || RelatedIssue {
            id: None,
            key: key.to_string(),
            summary: side["fields"]["summary"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            style: detect(
                side["fields"]["status"]["statusCategory"]["key"]
                    .as_str()
                    .unwrap_or_default(),
                side["fields"]["status"]["name"]
                    .as_str()
                    .unwrap_or_default(),
            ),
            issue_type: side["fields"]["issuetype"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            assignee: None,
        };
        links.push(IssueLink {
            relation: relation.to_string(),
            issue: cached.unwrap_or_else(fallback),
        });
    }
    Ok(links)
}

fn related_by_key(
    conn: &Connection,
    key: &str,
    styles: &HashMap<String, Style>,
    me: Option<&str>,
) -> AppResult<Option<RelatedIssue>> {
    Ok(conn
        .query_row(
            &format!("{RELATED} WHERE i.key = ?1 AND i.gone = 0"),
            [key],
            |r| related_from(r, styles, me),
        )
        .optional()?)
}
