//! A view's subtasks (ADR 0040): the descendants of its results through `parent_id`, of any type and at any
//! depth. Hidden leaves them out, List adds them as rows of their own, Hierarchy nests them under the results.

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;

use super::fetch::{Source, fetch};
use super::groups::{Facts, Group};
use super::rows::Row;
use crate::error::AppResult;
use crate::filter::TRACKED_ISSUES;

/// How many levels below a result the descendants reach.
pub const MAX_DEPTH: u8 = 5;

type Item = (Row, Facts);

/// Every tracked, still listed issue below the issues bound as one JSON array of ids, down to `MAX_DEPTH`
/// levels. A path never repeats an issue, so a cycle in the parent links ends.
fn below_condition() -> String {
    format!(
        "i.id IN (
           WITH RECURSIVE below(id, depth, path) AS (
             SELECT value, 0, '/' || value || '/' FROM json_each(?)
             UNION ALL
             SELECT c.id, b.depth + 1, b.path || c.id || '/'
             FROM below b JOIN cache.issue c ON c.parent_id = b.id
             WHERE b.depth < {MAX_DEPTH} AND c.gone = 0
               AND c.project_id IN (SELECT project_id FROM space WHERE tracked = 1)
               AND instr(b.path, '/' || c.id || '/') = 0
           )
           SELECT id FROM below WHERE depth > 0)"
    )
}

/// Every descendant of the results, unfiltered, in one query.
pub fn descendants(conn: &Connection, source: &Source, results: &[Item]) -> AppResult<Vec<Item>> {
    if results.iter().all(|(row, _)| row.child_count == 0) {
        return Ok(Vec::new());
    }
    let ids: Vec<&str> = results.iter().map(|(row, _)| row.id.as_str()).collect();
    fetch(
        conn,
        source,
        &format!("{TRACKED_ISSUES} AND {}", below_condition()),
        vec![serde_json::to_string(&ids)?.into()],
    )
}

/// List: the results and every descendant that isn't one of them, marked as a subtask.
pub fn as_items(results: Vec<Item>, descendants: Vec<Item>) -> Vec<Item> {
    let ids: HashSet<String> = results.iter().map(|(row, _)| row.id.clone()).collect();
    let subtasks = descendants
        .into_iter()
        .filter(|(row, _)| !ids.contains(&row.id))
        .map(|(mut row, facts)| {
            row.is_subtask = true;
            (row, facts)
        });
    results.into_iter().chain(subtasks).collect()
}

/// Hierarchy's rows: the roots go through grouping, each followed by its subtree.
pub struct Nested {
    pub roots: Vec<Item>,
    /// Root id → its descendants in depth order, siblings in the order they came in, each with its depth.
    pub below: HashMap<String, Vec<Row>>,
}

/// The results without those below another result, which nest under it instead; every issue shows once.
/// A result a cycle or the depth cap keeps out of every subtree becomes a root of its own.
pub fn nest(results: Vec<Item>, descendants: Vec<Item>) -> Nested {
    let result_ids: HashSet<&str> = results.iter().map(|(row, _)| row.id.as_str()).collect();
    let parent_of: HashMap<&str, &str> = results
        .iter()
        .chain(&descendants)
        .filter_map(|(row, _)| Some((row.id.as_str(), row.parent_id.as_deref()?)))
        .collect();
    let under_a_result = |id: &str| {
        let mut seen = HashSet::from([id]);
        let mut at = parent_of.get(id);
        while let Some(&parent) = at {
            if result_ids.contains(parent) {
                return true;
            }
            if !seen.insert(parent) {
                return false;
            }
            at = parent_of.get(parent);
        }
        false
    };
    let tops: HashSet<String> = results
        .iter()
        .filter(|(row, _)| !under_a_result(&row.id))
        .map(|(row, _)| row.id.clone())
        .collect();

    let mut children: HashMap<String, Vec<Row>> = HashMap::new();
    for (row, _) in descendants {
        if let Some(parent) = row.parent_id.clone() {
            children.entry(parent).or_default().push(row);
        }
    }
    let mut tree = Subtrees {
        children,
        placed: HashSet::new(),
    };
    let mut below = HashMap::new();
    for id in results
        .iter()
        .map(|(row, _)| &row.id)
        .filter(|id| tops.contains(*id))
    {
        below.insert(id.clone(), tree.lay(id));
    }
    let mut roots = Vec::new();
    for item in results {
        let id = &item.0.id;
        if !tops.contains(id) {
            if tree.placed.contains(id) {
                continue;
            }
            below.insert(id.clone(), tree.lay(id));
        }
        roots.push(item);
    }
    Nested { roots, below }
}

struct Subtrees {
    children: HashMap<String, Vec<Row>>,
    placed: HashSet<String>,
}

impl Subtrees {
    /// The root's descendants not placed yet, depth first.
    fn lay(&mut self, root: &str) -> Vec<Row> {
        self.placed.insert(root.to_string());
        let mut out = Vec::new();
        self.walk(root, 1, &mut out);
        out
    }

    fn walk(&mut self, parent: &str, depth: u8, out: &mut Vec<Row>) {
        if depth > MAX_DEPTH {
            return;
        }
        for mut child in self.children.get(parent).cloned().unwrap_or_default() {
            if !self.placed.insert(child.id.clone()) {
                continue;
            }
            child.depth = depth;
            let id = child.id.clone();
            out.push(child);
            self.walk(&id, depth + 1, out);
        }
    }
}

/// Puts each root's subtree right after it. A group's count stays its roots'.
pub fn expand(groups: &mut [Group], below: &HashMap<String, Vec<Row>>) {
    for group in groups {
        group.row_ids = group
            .row_ids
            .iter()
            .flat_map(|id| {
                let subtree = below.get(id).into_iter().flatten();
                std::iter::once(id.clone()).chain(subtree.map(|row| row.id.clone()))
            })
            .collect();
    }
}
