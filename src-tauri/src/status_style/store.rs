//! The owner's status style choices: app-wide for every status the tracked projects use, and per Space with
//! the Space's issue counts.

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
    /// The owner's choice in this Space; none = the app-wide style.
    pub chosen: Option<Style>,
    /// The owner's app-wide choice; none = detected.
    pub app: Option<Style>,
    /// The Space's issues in this status.
    pub issues: i64,
}

impl StatusStyleRow {
    pub fn style(&self) -> Style {
        self.chosen.or(self.app).unwrap_or(self.detected)
    }
}

/// The statuses of the Space's tracked issues; other workflows on the site are ignored.
pub fn status_rows(conn: &Connection, project_id: &str) -> AppResult<Vec<StatusStyleRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT s.id, s.name, s.category, o.style, a.style, used.issues
         FROM cache.status s
         JOIN (SELECT i.status_id, COUNT(*) AS issues FROM cache.issue i
               WHERE {TRACKED_ISSUES} AND i.project_id = ?1
               GROUP BY i.status_id) used ON used.status_id = s.id
         LEFT JOIN space_status_style o ON o.status_id = s.id AND o.project_id = ?1
         LEFT JOIN status_style a ON a.status_id = s.id
         ORDER BY CASE s.category WHEN 'new' THEN 0 WHEN 'indeterminate' THEN 1 ELSE 2 END,
                  s.name COLLATE NOCASE, s.id"
    ))?;
    let rows = query.query_map([project_id], |row| {
        let name: String = row.get(1)?;
        let category: String = row.get(2)?;
        Ok(StatusStyleRow {
            id: row.get(0)?,
            detected: detect(&category, &name),
            chosen: style_at(row, 3)?,
            app: style_at(row, 4)?,
            issues: row.get(5)?,
            name,
            category,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub async fn status_styles_list(db: &Db, project_id: String) -> AppResult<Vec<StatusStyleRow>> {
    db.read(move |conn| status_rows(conn, &project_id)).await
}

/// `None` goes back to the app-wide style.
pub async fn status_style_set(
    db: &Db,
    project_id: String,
    status_id: String,
    style: Option<Style>,
) -> AppResult<Vec<StatusStyleRow>> {
    db.write(move |tx, changes| {
        ensure_known(tx, &status_id)?;
        match style {
            Some(style) => tx.execute(
                "INSERT INTO space_status_style (project_id, status_id, style) VALUES (?1, ?2, ?3)
                 ON CONFLICT (project_id, status_id) DO UPDATE SET style = excluded.style",
                params![project_id, status_id, to_text(style)],
            )?,
            None => tx.execute(
                "DELETE FROM space_status_style WHERE project_id = ?1 AND status_id = ?2",
                params![project_id, status_id],
            )?,
        };
        changes.local_changed(LocalPart::Styles);
        status_rows(tx, &project_id)
    })
    .await
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppStatusRow {
    pub id: String,
    pub name: String,
    /// Jira's status category: `new`, `indeterminate` or `done`.
    pub category: String,
    pub detected: Style,
    /// The owner's app-wide choice; none = detected.
    pub chosen: Option<Style>,
}

/// Every status a tracked project's issues are in, by name.
pub fn app_status_rows(conn: &Connection) -> AppResult<Vec<AppStatusRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT s.id, s.name, s.category, a.style
         FROM cache.status s
         LEFT JOIN status_style a ON a.status_id = s.id
         WHERE s.id IN (SELECT i.status_id FROM cache.issue i WHERE {TRACKED_ISSUES})
         ORDER BY s.name COLLATE NOCASE, s.id"
    ))?;
    let rows = query.query_map([], |row| {
        let name: String = row.get(1)?;
        let category: String = row.get(2)?;
        Ok(AppStatusRow {
            id: row.get(0)?,
            detected: detect(&category, &name),
            chosen: style_at(row, 3)?,
            name,
            category,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub async fn status_styles_all(db: &Db) -> AppResult<Vec<AppStatusRow>> {
    db.read(app_status_rows).await
}

/// `None` goes back to the detected style; a Space's own choice stays.
pub async fn status_style_set_app(
    db: &Db,
    status_id: String,
    style: Option<Style>,
) -> AppResult<Vec<AppStatusRow>> {
    db.write(move |tx, changes| {
        ensure_known(tx, &status_id)?;
        match style {
            Some(style) => tx.execute(
                "INSERT INTO status_style (status_id, style) VALUES (?1, ?2)
                 ON CONFLICT (status_id) DO UPDATE SET style = excluded.style",
                params![status_id, to_text(style)],
            )?,
            None => tx.execute(
                "DELETE FROM status_style WHERE status_id = ?1",
                params![status_id],
            )?,
        };
        changes.local_changed(LocalPart::Styles);
        app_status_rows(tx)
    })
    .await
}

fn ensure_known(conn: &Connection, status_id: &str) -> AppResult<()> {
    let known: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM cache.status WHERE id = ?1)",
        [status_id],
        |row| row.get(0),
    )?;
    if known {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "Status {status_id} doesn't exist"
        )))
    }
}

fn style_at(row: &rusqlite::Row, index: usize) -> rusqlite::Result<Option<Style>> {
    let text: Option<String> = row.get(index)?;
    Ok(text.as_deref().and_then(from_text))
}
