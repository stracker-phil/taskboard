//! The palette's places: Spaces, filter lists and their views, found by every typed word in their path.

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppResult;
use crate::views::{Layout, fold, list_views};

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

/// One view of a level, opened at that view.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ViewHit {
    pub list: ListHit,
    pub view_id: i64,
    pub name: String,
    pub layout: Layout,
}

/// A level with the name of the Space it belongs to, for its path.
struct Level {
    hit: ListHit,
    space_name: Option<String>,
}

impl Level {
    /// "Space list" for a Space's list, the name alone for a Space or a Home list.
    fn path(&self) -> String {
        match (&self.space_name, self.hit.is_space) {
            (Some(space), false) => format!("{space} {}", self.hit.name),
            _ => self.hit.name.clone(),
        }
    }
}

/// Every level of a tracked Space or outside the Spaces: Spaces first, the others under their parent.
fn levels(conn: &Connection) -> AppResult<Vec<Level>> {
    let mut query = conn.prepare(
        "SELECT l.id, COALESCE(s.name, p.name, l.name), COALESCE(l.project_id, parent.project_id), l.kind = 'space',
                COALESCE(ps.name, pp.name)
         FROM filter_list l
         LEFT JOIN filter_list parent ON parent.id = l.parent_id
         LEFT JOIN space s ON s.project_id = l.project_id
         LEFT JOIN cache.space p ON p.id = l.project_id
         LEFT JOIN space ps ON ps.project_id = COALESCE(l.project_id, parent.project_id)
         LEFT JOIN cache.space pp ON pp.id = COALESCE(l.project_id, parent.project_id)
         WHERE COALESCE(l.project_id, parent.project_id) IS NULL
            OR COALESCE(l.project_id, parent.project_id) IN (SELECT project_id FROM space WHERE tracked = 1)
         ORDER BY l.kind = 'space' DESC, parent.position IS NOT NULL, parent.position, l.parent_id, l.position, l.id",
    )?;
    let rows = query.query_map([], |r| {
        Ok(Level {
            hit: ListHit {
                id: r.get(0)?,
                name: r.get(1)?,
                project_id: r.get(2)?,
                is_space: r.get(3)?,
            },
            space_name: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Every word of the typed text, folded, appears in `path`.
fn holds_every_word(path: &str, words: &[String]) -> bool {
    let path = fold(path);
    words.iter().all(|word| path.contains(word.as_str()))
}

fn words(text: &str) -> Vec<String> {
    fold(text).split_whitespace().map(str::to_string).collect()
}

/// Up to `limit` levels whose path holds every typed word; with nothing typed, every level when `all_when_empty`.
pub fn list_matches(
    conn: &Connection,
    text: &str,
    limit: usize,
    all_when_empty: bool,
) -> AppResult<Vec<ListHit>> {
    let words = words(text);
    if limit == 0 || (words.is_empty() && !all_when_empty) {
        return Ok(Vec::new());
    }
    Ok(levels(conn)?
        .into_iter()
        .filter(|level| holds_every_word(&level.path(), &words))
        .take(limit)
        .map(|level| level.hit)
        .collect())
}

/// Up to `limit` views whose path (Space, list, view) holds every typed word and that name the view itself;
/// nothing without typed text. Bookmarks has no views to choose.
pub fn view_matches(conn: &Connection, text: &str, limit: usize) -> AppResult<Vec<ViewHit>> {
    let words = words(text);
    if limit == 0 || words.is_empty() {
        return Ok(Vec::new());
    }
    let mut hits = Vec::new();
    for level in levels(conn)? {
        let path = level.path();
        let pinned: bool = conn.query_row(
            "SELECT kind = 'pinned' FROM filter_list WHERE id = ?1",
            [level.hit.id],
            |r| r.get(0),
        )?;
        if pinned {
            continue;
        }
        for view in list_views(conn, level.hit.id)? {
            let name = fold(&view.name);
            // A level's name alone finds the level, not each of its views.
            let names_view = words.iter().any(|word| name.contains(word.as_str()));
            if names_view && holds_every_word(&format!("{path} {}", view.name), &words) {
                hits.push(ViewHit {
                    list: level.hit.clone(),
                    view_id: view.id,
                    name: view.name,
                    layout: view.layout,
                });
                if hits.len() == limit {
                    return Ok(hits);
                }
            }
        }
    }
    Ok(hits)
}
