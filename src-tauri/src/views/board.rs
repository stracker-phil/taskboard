//! The Board layout: one column per status of the issues shown, in the Board artboard's style order.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use super::query::{Fetched, Narrowing, fetch_view};
use super::rows::Row;
use crate::db::Db;
use crate::error::AppResult;
use crate::lists::Viewer;
use crate::status_style::Style;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct BoardData {
    pub columns: Vec<BoardColumn>,
    pub rows: HashMap<String, Row>,
    /// What a card shows besides its key and summary, in order.
    pub card_fields: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct BoardColumn {
    pub status_id: String,
    pub name: String,
    pub style: Style,
    /// Shown as a 48px strip.
    pub collapsed: bool,
    /// Row ids in the view's sort order.
    pub ids: Vec<String>,
}

/// To do, In progress, Blocked, In review, Done: the Board artboard's order (the List groups differently).
fn style_rank(style: Style) -> u8 {
    match style {
        Style::Todo => 0,
        Style::Progress => 1,
        Style::Blocked => 2,
        Style::Review => 3,
        Style::Done => 4,
    }
}

/// The order Jira listed its statuses in when the sync cached them.
fn status_positions(conn: &Connection) -> AppResult<HashMap<String, i64>> {
    let mut query = conn.prepare("SELECT id, rowid FROM cache.status")?;
    let rows = query.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub async fn view_board(
    db: &Db,
    viewer: Viewer,
    list_id: i64,
    view_id: i64,
    narrowing: Narrowing,
) -> AppResult<BoardData> {
    db.read(move |conn| {
        let Fetched { view, items, .. } = fetch_view(conn, &viewer, list_id, view_id, &narrowing)?;
        let positions = status_positions(conn)?;
        let mut columns: Vec<BoardColumn> = Vec::new();
        for (row, facts) in &items {
            match columns.iter_mut().find(|c| c.status_id == facts.status_id) {
                Some(column) => column.ids.push(row.id.clone()),
                None => columns.push(BoardColumn {
                    status_id: facts.status_id.clone(),
                    name: row.status_name.clone(),
                    style: row.style,
                    collapsed: view
                        .collapsed
                        .get(&facts.status_id)
                        .copied()
                        .unwrap_or(row.style == Style::Done),
                    ids: vec![row.id.clone()],
                }),
            }
        }
        columns.sort_by_key(|c| {
            (
                style_rank(c.style),
                positions.get(&c.status_id).copied().unwrap_or(i64::MAX),
            )
        });
        Ok(BoardData {
            columns,
            rows: items
                .into_iter()
                .map(|(row, _)| (row.id.clone(), row))
                .collect(),
            card_fields: view.columns,
        })
    })
    .await
}
