//! The style choices of a Space's statuses: read with their issue counts, and set.

use rusqlite::{Connection, params};
use serde::Serialize;
use ts_rs::TS;

use super::{Style, detect};
use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::filter::TRACKED_ISSUES;
use crate::views::model::{from_text, to_text};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StatusStyleRow {
    pub id: String,
    pub name: String,
    /// Jira's status category: `new`, `indeterminate` or `done`.
    pub category: String,
    pub detected: Style,
    /// The owner's choice in this Space; none = Automatic.
    pub chosen: Option<Style>,
    /// The Space's issues in this status.
    pub issues: i64,
}

impl StatusStyleRow {
    pub fn style(&self) -> Style {
        self.chosen.unwrap_or(self.detected)
    }
}

/// The statuses a Space's issues are in (the rest of the site's workflows don't matter here), by Jira category
/// (to do, in progress, done), then name.
pub fn status_rows(conn: &Connection, project_id: &str) -> AppResult<Vec<StatusStyleRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT s.id, s.name, s.category, o.style, used.issues
         FROM cache.status s
         JOIN (SELECT i.status_id, COUNT(*) AS issues FROM cache.issue i
               WHERE {TRACKED_ISSUES} AND i.project_id = ?1
               GROUP BY i.status_id) used ON used.status_id = s.id
         LEFT JOIN space_status_style o ON o.status_id = s.id AND o.project_id = ?1
         ORDER BY CASE s.category WHEN 'new' THEN 0 WHEN 'indeterminate' THEN 1 ELSE 2 END,
                  s.name COLLATE NOCASE, s.id"
    ))?;
    let rows = query.query_map([project_id], |row| {
        let name: String = row.get(1)?;
        let category: String = row.get(2)?;
        let chosen: Option<String> = row.get(3)?;
        Ok(StatusStyleRow {
            id: row.get(0)?,
            detected: detect(&category, &name),
            chosen: chosen.as_deref().and_then(from_text),
            issues: row.get(4)?,
            name,
            category,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub async fn status_styles_list(db: &Db, project_id: String) -> AppResult<Vec<StatusStyleRow>> {
    db.read(move |conn| status_rows(conn, &project_id)).await
}

/// `None` goes back to the detected style.
pub async fn status_style_set(
    db: &Db,
    project_id: String,
    status_id: String,
    style: Option<Style>,
) -> AppResult<Vec<StatusStyleRow>> {
    db.write(move |tx, changes| {
        let known: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM cache.status WHERE id = ?1)",
            [&status_id],
            |row| row.get(0),
        )?;
        if !known {
            return Err(AppError::NotFound(format!(
                "Status {status_id} doesn't exist"
            )));
        }
        match style {
            Some(style) => tx.execute(
                "INSERT INTO space_status_style (project_id, status_id, style) VALUES (?1, ?2, ?3)
                 ON CONFLICT (project_id, status_id) DO UPDATE SET style = excluded.style",
                params![project_id, status_id, to_text(style)],
            )?,
            None => tx.execute(
                "DELETE FROM space_status_style WHERE project_id = ?1 AND status_id = ?2",
                [&project_id, &status_id],
            )?,
        };
        changes.local_changed(LocalPart::Styles);
        status_rows(tx, &project_id)
    })
    .await
}
