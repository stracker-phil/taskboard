//! One status language: every Jira status shows in one of five styles, detected from its category and
//! name unless the owner picked one (`status_style`).

pub mod commands;

use std::collections::HashMap;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::views::model::{from_text, to_text};

/// Stored as its name; the `--status-{style}-*` tokens draw it. Ordered as the list's status groups.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Style {
    Progress,
    Review,
    Blocked,
    Todo,
    Done,
}

/// Jira's category decides To do and Done; the name tells review and blocked apart from in progress.
pub fn detect(category: &str, name: &str) -> Style {
    let name = name.to_lowercase();
    match category {
        "new" => Style::Todo,
        "done" => Style::Done,
        _ if name.contains("review") => Style::Review,
        _ if name.contains("block") => Style::Blocked,
        _ => Style::Progress,
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StatusStyleRow {
    pub id: String,
    pub name: String,
    /// Jira's status category: `new`, `indeterminate` or `done`.
    pub category: String,
    pub detected: Style,
    /// The owner's choice; none = Automatic.
    pub chosen: Option<Style>,
}

impl StatusStyleRow {
    pub fn style(&self) -> Style {
        self.chosen.unwrap_or(self.detected)
    }
}

/// Every cached status, by name.
pub fn status_rows(conn: &Connection) -> AppResult<Vec<StatusStyleRow>> {
    let mut query = conn.prepare(
        "SELECT s.id, s.name, s.category, o.style
         FROM cache.status s LEFT JOIN status_style o ON o.status_id = s.id
         ORDER BY s.name COLLATE NOCASE, s.id",
    )?;
    let rows = query.query_map([], |row| {
        let name: String = row.get(1)?;
        let category: String = row.get(2)?;
        let chosen: Option<String> = row.get(3)?;
        Ok(StatusStyleRow {
            id: row.get(0)?,
            detected: detect(&category, &name),
            chosen: chosen.as_deref().and_then(from_text),
            name,
            category,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Each cached status's style.
pub fn styles(conn: &Connection) -> AppResult<HashMap<String, Style>> {
    Ok(status_rows(conn)?
        .into_iter()
        .map(|row| (row.id.clone(), row.style()))
        .collect())
}

pub async fn status_styles_list(db: &Db) -> AppResult<Vec<StatusStyleRow>> {
    db.read(status_rows).await
}

/// `None` goes back to the detected style.
pub async fn status_style_set(
    db: &Db,
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
                "INSERT INTO status_style (status_id, style) VALUES (?1, ?2)
                 ON CONFLICT (status_id) DO UPDATE SET style = excluded.style",
                params![status_id, to_text(style)],
            )?,
            None => tx.execute(
                "DELETE FROM status_style WHERE status_id = ?1",
                [&status_id],
            )?,
        };
        changes.local_changed(LocalPart::Styles);
        status_rows(tx)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_follows_the_category_and_then_the_name() {
        for (category, name, expected) in [
            ("new", "To Do", Style::Todo),
            ("new", "Blocked", Style::Todo),
            ("done", "Done", Style::Done),
            ("done", "Done review", Style::Done),
            ("indeterminate", "In Review", Style::Review),
            ("indeterminate", "Code review", Style::Review),
            ("indeterminate", "Blocked", Style::Blocked),
            ("indeterminate", "Blocker found", Style::Blocked),
            ("indeterminate", "In Progress", Style::Progress),
            ("indeterminate", "Testing", Style::Progress),
        ] {
            assert_eq!(detect(category, name), expected, "{category} / {name}");
        }
    }

    #[test]
    fn styles_rank_in_the_lists_group_order() {
        let mut styles = [
            Style::Done,
            Style::Todo,
            Style::Blocked,
            Style::Review,
            Style::Progress,
        ];
        styles.sort();
        assert_eq!(
            styles,
            [
                Style::Progress,
                Style::Review,
                Style::Blocked,
                Style::Todo,
                Style::Done
            ]
        );
    }

    #[test]
    fn a_chosen_style_wins_over_the_detected_one() {
        let row = |chosen| StatusStyleRow {
            id: "10001".into(),
            name: "In Review".into(),
            category: "indeterminate".into(),
            detected: Style::Review,
            chosen,
        };
        assert_eq!(row(Some(Style::Blocked)).style(), Style::Blocked);
        assert_eq!(row(None).style(), Style::Review);
    }
}
