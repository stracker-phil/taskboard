//! Subtasks in a view (ADR 0037): which issues a mode filters, and in Hierarchy each parent's subtasks right
//! after it, in its group.

use std::collections::HashMap;

use rusqlite::Connection;

use super::fetch::{Source, fetch};
use super::groups::{Facts, Group};
use super::model::SubtaskMode;
use super::rows::Row;
use crate::error::AppResult;
use crate::filter::{TOP_LEVEL, TRACKED_ISSUES};

/// What the mode adds to the view's condition: Hidden and Hierarchy filter top-level issues only.
pub fn top_level_only(mode: SubtaskMode) -> Option<&'static str> {
    (mode != SubtaskMode::List).then_some(TOP_LEVEL)
}

/// Every subtask of the parents bound as one JSON array of ids.
const OF_PARENTS: &str =
    "i.type_is_subtask = 1 AND i.parent_id IN (SELECT value FROM json_each(?))";

/// Puts each parent's subtasks, in their order, right after it. A group's count stays its parents'.
pub fn nest(groups: &mut [Group], subtasks: &[Row]) {
    let mut by_parent: HashMap<&str, Vec<&str>> = HashMap::new();
    for row in subtasks {
        if let Some(parent) = &row.parent_id {
            by_parent.entry(parent).or_default().push(&row.id);
        }
    }
    for group in groups {
        group.row_ids = group
            .row_ids
            .iter()
            .flat_map(|id| {
                let children = by_parent.get(id.as_str()).into_iter().flatten();
                std::iter::once(id.clone()).chain(children.map(|child| child.to_string()))
            })
            .collect();
    }
}

/// Every subtask of the parents, unfiltered.
pub fn subtasks_of(
    conn: &Connection,
    source: &Source,
    parents: &[(Row, Facts)],
) -> AppResult<Vec<(Row, Facts)>> {
    if parents.iter().all(|(row, _)| row.subtask_count == 0) {
        return Ok(Vec::new());
    }
    let ids: Vec<&str> = parents.iter().map(|(row, _)| row.id.as_str()).collect();
    let ids = serde_json::to_string(&ids)?;
    fetch(
        conn,
        source,
        &format!("{TRACKED_ISSUES} AND {OF_PARENTS}"),
        vec![ids.into()],
    )
}
