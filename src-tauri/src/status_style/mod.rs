//! One status language: every Jira status shows in one of six styles (status types), detected from its category and
//! name unless the owner picked one app-wide (`status_style`) or in a Space (`space_status_style`).

pub mod commands;

mod store;

use std::collections::HashMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;
use crate::views::model::from_text;

pub use store::{
    AppStatusRow, StatusStyleRow, status_rows, status_style_set, status_style_set_app,
    status_styles_all, status_styles_list,
};

/// Stored as its name; the `--status-{style}-*` tokens draw it. Ordered as the list's status groups.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Style {
    Progress,
    Review,
    Blocked,
    Todo,
    Backlog,
    Done,
}

impl Style {
    /// The status type's name, as the Status type group and column show it.
    pub fn name(self) -> &'static str {
        match self {
            Style::Backlog => "Backlog",
            Style::Todo => "To do",
            Style::Progress => "In progress",
            Style::Review => "In review",
            Style::Blocked => "Blocked",
            Style::Done => "Done",
        }
    }
}

/// Jira's category decides To do and Done; the name tells backlog apart from to do, and review and blocked
/// apart from in progress.
pub fn detect(category: &str, name: &str) -> Style {
    let name = name.to_lowercase();
    match category {
        "new" if name.contains("backlog") => Style::Backlog,
        "new" => Style::Todo,
        "done" => Style::Done,
        _ if name.contains("review") => Style::Review,
        _ if name.contains("block") => Style::Blocked,
        _ => Style::Progress,
    }
}

/// The owner's style choices: app-wide and per Space. A status without one shows its detected style.
pub struct Styles {
    space: HashMap<(String, String), Style>,
    app: HashMap<String, Style>,
}

impl Styles {
    pub fn load(conn: &Connection) -> AppResult<Self> {
        let space = choices(
            conn,
            "SELECT project_id, status_id, style FROM space_status_style",
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let app = choices(conn, "SELECT status_id, style FROM status_style", |row| {
            row.get(0)
        })?;
        Ok(Self { space, app })
    }

    /// A status's style in a Space: the owner's choice there, else app-wide, else detected.
    pub fn of(&self, project_id: &str, status_id: &str, category: &str, name: &str) -> Style {
        self.space
            .get(&(project_id.to_string(), status_id.to_string()))
            .copied()
            .unwrap_or_else(|| self.app_of(status_id, category, name))
    }

    /// A status's style outside any Space: the owner's app-wide choice, else detected.
    pub fn app_of(&self, status_id: &str, category: &str, name: &str) -> Style {
        self.app
            .get(status_id)
            .copied()
            .unwrap_or_else(|| detect(category, name))
    }
}

/// The rows of a choice table by key; the style is the last column, and an unknown one counts as no choice.
fn choices<K: Eq + std::hash::Hash>(
    conn: &Connection,
    sql: &str,
    key: impl Fn(&rusqlite::Row) -> rusqlite::Result<K>,
) -> AppResult<HashMap<K, Style>> {
    let mut query = conn.prepare(sql)?;
    let last = query.column_count() - 1;
    let rows = query.query_map([], |row| Ok((key(row)?, row.get::<_, String>(last)?)))?;
    let mut chosen = HashMap::new();
    for row in rows {
        let (key, style) = row?;
        if let Some(style) = from_text(&style) {
            chosen.insert(key, style);
        }
    }
    Ok(chosen)
}

#[cfg(test)]
mod tests;
