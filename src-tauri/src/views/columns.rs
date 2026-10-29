//! The columns a view can show: the standard fields, then the site's custom fields by their Jira ids.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::db::Db;
use crate::error::AppResult;
use crate::filter::RELEVANT_FIELD;

/// Standard column ids with their names, in the Columns popover's order.
const STANDARD: [(&str, &str); 8] = [
    ("status", "Status"),
    ("assignee", "Assignee"),
    ("epic", "Epic"),
    ("release", "Release"),
    ("priority", "Priority"),
    ("labels", "Labels"),
    ("sprint", "Sprint"),
    ("due", "Due"),
];

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// A standard id or a Jira field id (`customfield_10071`).
    pub id: String,
    pub name: String,
    pub custom: bool,
}

/// A custom field a column can show, with Jira's schema type (`user`, `number`, `option`, …).
#[derive(Clone, Debug, PartialEq)]
pub struct CustomColumn {
    pub id: String,
    pub kind: String,
}

/// Custom fields with a value a cell can show. Jira's `any` fields (rank, epic colour) are internal, and the
/// sprint field is the standard Sprint column.
const SHOWABLE: &str =
    "id LIKE 'customfield\\_%' ESCAPE '\\' AND COALESCE(type, '') <> 'any' AND name <> 'Sprint'";

pub fn catalogue(conn: &Connection) -> AppResult<Vec<Column>> {
    let mut columns: Vec<Column> = STANDARD
        .iter()
        .map(|(id, name)| Column {
            id: id.to_string(),
            name: name.to_string(),
            custom: false,
        })
        .collect();
    let mut query = conn.prepare(&format!(
        "SELECT id, name FROM cache.field WHERE {SHOWABLE} AND {RELEVANT_FIELD}
         ORDER BY name COLLATE NOCASE, id"
    ))?;
    let custom = query.query_map([], |row| {
        Ok(Column {
            id: row.get(0)?,
            name: row.get(1)?,
            custom: true,
        })
    })?;
    columns.extend(custom.collect::<Result<Vec<_>, _>>()?);
    Ok(columns)
}

/// The custom fields among `ids`, in that order; unknown ids are left out.
pub fn custom_columns(conn: &Connection, ids: &[String]) -> AppResult<Vec<CustomColumn>> {
    let mut query = conn.prepare(&format!(
        "SELECT COALESCE(type, '') FROM cache.field WHERE id = ?1 AND {SHOWABLE}"
    ))?;
    let mut found = Vec::new();
    for id in ids {
        if let Some(kind) = query.query_row([id], |row| row.get(0)).optional()? {
            found.push(CustomColumn {
                id: id.clone(),
                kind,
            });
        }
    }
    Ok(found)
}

pub async fn columns_catalogue(db: &Db) -> AppResult<Vec<Column>> {
    db.read(catalogue).await
}
