//! The Board layout: one column per group of the view's grouping. By status (the default, also for an
//! ungrouped view) or status type the columns follow the Board artboard's style order; any other grouping
//! orders them as the List does.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use super::groups::{Facts, GroupMark, Lookup, group};
use super::lookup::{spaces, versions};
use super::model::{GroupBy, View};
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
    /// The group's key: a status id when grouped by status.
    pub key: String,
    pub name: String,
    /// What the header draws before the name; a status shows as its pill.
    pub mark: Option<GroupMark>,
    /// The column's tint: its status's style (or status type's), else To do's neutral one.
    pub style: Style,
    /// Shown as a 48px strip.
    pub collapsed: bool,
    /// Row ids in the view's sort order.
    pub ids: Vec<String>,
}

/// Backlog, To do, In progress, Blocked, In review, Done: the Board artboard's order (the List groups
/// differently).
fn style_rank(style: Style) -> u8 {
    match style {
        Style::Backlog => 0,
        Style::Todo => 1,
        Style::Progress => 2,
        Style::Blocked => 3,
        Style::Review => 4,
        Style::Done => 5,
    }
}

/// The order Jira listed its statuses in when the sync cached them.
fn status_positions(conn: &Connection) -> AppResult<HashMap<String, i64>> {
    let mut query = conn.prepare("SELECT id, rowid FROM cache.status")?;
    let rows = query.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn status_columns(
    conn: &Connection,
    view: &View,
    items: &[(Row, Facts)],
) -> AppResult<Vec<BoardColumn>> {
    let positions = status_positions(conn)?;
    let mut columns: Vec<BoardColumn> = Vec::new();
    for (row, facts) in items {
        match columns.iter_mut().find(|c| c.key == facts.status_id) {
            Some(column) => column.ids.push(row.id.clone()),
            None => columns.push(BoardColumn {
                key: facts.status_id.clone(),
                name: row.status_name.clone(),
                mark: Some(GroupMark::Status { style: row.style }),
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
            positions.get(&c.key).copied().unwrap_or(i64::MAX),
        )
    });
    Ok(columns)
}

pub async fn view_board(
    db: &Db,
    viewer: Viewer,
    list_id: i64,
    view_id: i64,
    narrowing: Narrowing,
) -> AppResult<BoardData> {
    db.read(move |conn| {
        let Fetched { view, items, ctx } = fetch_view(conn, &viewer, list_id, view_id, &narrowing)?;
        let columns = match view.group_by {
            GroupBy::Status | GroupBy::None => status_columns(conn, &view, &items)?,
            by => {
                let versions = versions(conn)?;
                let lookup = Lookup {
                    me: ctx.me.as_deref(),
                    today: ctx.today,
                    spaces: &spaces(conn)?,
                    versions: &versions,
                };
                let mut columns: Vec<BoardColumn> = group(&items, by, &lookup)
                    .into_iter()
                    .map(|g| BoardColumn {
                        collapsed: view
                            .collapsed
                            .get(&g.key)
                            .copied()
                            .unwrap_or(g.collapsed_by_default),
                        style: match g.mark {
                            Some(GroupMark::Status { style }) => style,
                            _ => Style::Todo,
                        },
                        key: g.key,
                        name: g.label,
                        mark: g.mark,
                        ids: g.row_ids,
                    })
                    .collect();
                if by == GroupBy::StatusType {
                    columns.sort_by_key(|c| style_rank(c.style));
                }
                columns
            }
        };
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
