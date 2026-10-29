//! A view: one tab of a sidebar level. It decides how issues show, and its own conditions narrow which
//! (ADR 0033).

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
    /// The status's style: one group per status type (color), however many statuses share it.
    StatusType,
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

/// What a click on an issue opens: the preview beside the view, or the full Issue view.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum IssueOpening {
    Full,
    /// Also what an unknown value reads as.
    #[default]
    #[serde(other)]
    Preview,
}

/// How a view shows subtasks (ADR 0037); the Board always hides them.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum SubtaskMode {
    /// Top-level issues only; a parent shows how many subtasks it has.
    #[default]
    Hidden,
    /// Only parents go through the filter; each one's subtasks follow it, indented.
    Hierarchy,
    /// Subtasks are rows of their own, filtered and grouped on their own values.
    List,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub id: i64,
    pub list_id: i64,
    pub name: String,
    pub layout: Layout,
    /// The layouts its Space offers; `layout` is always one of them.
    pub layouts: Vec<Layout>,
    /// Column ids in order (`assignee`, `customfield_10071`); a board's card fields.
    pub columns: Vec<String>,
    pub group_by: GroupBy,
    pub sort: Sort,
    /// The Table layout's column widths in points, by column id; missing ids use their default.
    pub widths: BTreeMap<String, i64>,
    /// The List layout wraps long titles onto more lines; off, it truncates them.
    pub wrap_titles: bool,
    /// Labels show as tags after the title while the view has no Labels column.
    pub inline_labels: bool,
    /// A Board's columns the owner collapsed (true) or expanded (false), by status id; the rest use the
    /// default (Done collapsed).
    pub collapsed: BTreeMap<String, bool>,
    /// What a click on an issue opens here; none follows Settings → General.
    pub open_with: Option<IssueOpening>,
    pub subtasks: SubtaskMode,
    /// The tab's icon (`look`); none shows the layout's.
    pub icon: Option<String>,
    /// The icon's tile color; none shows the icon plain.
    pub color: Option<String>,
    pub position: i64,
    /// The tab a level opens with.
    pub is_default: bool,
    /// The view's own conditions as stored JSON (ADR 0033), ANDed with its list's.
    #[serde(skip)]
    #[ts(skip)]
    pub filter: Option<String>,
    /// For the Filter badge; 0 when the stored filter can't be read.
    pub own_conditions: usize,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_issue_opening_reads_its_camel_case_name_and_an_unknown_one_as_preview() {
        for (text, expected) in [
            ("full", IssueOpening::Full),
            ("preview", IssueOpening::Preview),
            ("sidebar", IssueOpening::Preview),
            ("", IssueOpening::Preview),
        ] {
            assert_eq!(from_text::<IssueOpening>(text), Some(expected), "{text:?}");
        }
    }

    #[test]
    fn an_issue_opening_is_stored_as_its_camel_case_name() {
        assert_eq!(to_text(IssueOpening::Full), "full");
        assert_eq!(to_text(IssueOpening::Preview), "preview");
        assert_eq!(IssueOpening::default(), IssueOpening::Preview);
    }
}
