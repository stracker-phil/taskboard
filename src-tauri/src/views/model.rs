//! A view: one tab of a sidebar level. It decides how issues show, never which.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The columns a new view starts with (spec: Columns per view).
pub const DEFAULT_COLUMNS: [&str; 4] = ["assignee", "epic", "release", "priority"];

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Layout {
    List,
    Board,
    Table,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum GroupBy {
    Status,
    Project,
    Client,
    Assignee,
    Epic,
    Release,
    Priority,
    Due,
    None,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum SortBy {
    Updated,
    Created,
    Priority,
    Due,
    Key,
    Summary,
    Status,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Sort {
    pub by: SortBy,
    pub descending: bool,
}

impl Default for Sort {
    fn default() -> Self {
        Self {
            by: SortBy::Updated,
            descending: true,
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub id: i64,
    pub list_id: i64,
    pub name: String,
    pub layout: Layout,
    /// Column ids in order (`assignee`, `customfield_10071`); a board's card fields.
    pub columns: Vec<String>,
    pub group_by: GroupBy,
    pub sort: Sort,
    /// The Table layout's column widths in points, by column id; missing ids use their default.
    pub widths: BTreeMap<String, i64>,
    /// The List layout wraps long titles onto more lines; off, it truncates them.
    pub wrap_titles: bool,
    /// A Board's columns the owner collapsed (true) or expanded (false), by status id; the rest use the
    /// default (Done collapsed).
    pub collapsed: BTreeMap<String, bool>,
    pub position: i64,
    /// The tab a level opens with.
    pub is_default: bool,
}

/// Enum values as stored in their text columns.
pub fn to_text<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

pub fn from_text<T: for<'de> Deserialize<'de>>(text: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(text.to_string())).ok()
}
