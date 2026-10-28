//! What a newly tracked project starts with: its Space node with three views, and the default filter lists.

use rusqlite::{Connection, OptionalExtension, params};

use super::nodes::space_node;
use crate::error::AppResult;
use crate::filter::{Condition, Field, Filter, Op, Value};
use crate::order::Siblings;
use crate::views::model::{GroupBy, Layout};
use crate::views::{columns_for_new_view, insert_view};

/// The view a new filter list starts with.
pub const LIST_VIEW: &str = "List";

/// Creates the project's Space node, views and default lists unless it has them. Returns whether it did.
pub fn ensure_space(conn: &Connection, project_id: &str) -> AppResult<bool> {
    if space_node(conn, project_id)?.is_some() {
        return Ok(false);
    }
    let name: String = conn
        .query_row(
            "SELECT name FROM cache.project WHERE id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_else(|| project_id.to_string());
    conn.execute(
        "INSERT INTO filter_list (kind, project_id, name) VALUES ('space', ?1, ?2)",
        params![project_id, name],
    )?;
    let space = conn.last_insert_rowid();
    let columns = columns_for_new_view(conn, space)?;
    for (name, layout) in [
        ("All issues", Layout::List),
        ("Board", Layout::Board),
        ("Table", Layout::Table),
    ] {
        insert_view(conn, space, name, layout, GroupBy::Status, &columns)?;
    }
    for (name, conditions) in default_lists(conn, project_id)? {
        insert_list(conn, Some(space), &name, &Filter { conditions })?;
    }
    Ok(true)
}

/// Current sprint, Epics, Release <next version> when the project has an unreleased one, Open bugs.
fn default_lists(conn: &Connection, project_id: &str) -> AppResult<Vec<(String, Vec<Condition>)>> {
    let mut lists = vec![
        (
            "Current sprint".to_string(),
            vec![Condition::is(Field::Sprint, "active")],
        ),
        (
            "Epics".to_string(),
            vec![Condition::is(Field::Type, "Epic")],
        ),
    ];
    if let Some((id, name)) = next_release(conn, project_id)? {
        lists.push((
            format!("Release {name}"),
            vec![Condition::is(Field::Release, id)],
        ));
    }
    lists.push((
        "Open bugs".to_string(),
        vec![
            Condition::is(Field::Type, "Bug"),
            Condition::new(
                Field::StatusCategory,
                Op::IsNot,
                Some(Value::One("done".into())),
            ),
        ],
    ));
    Ok(lists)
}

/// The unreleased version due first; versions without a date come last.
fn next_release(conn: &Connection, project_id: &str) -> AppResult<Option<(String, String)>> {
    Ok(conn
        .query_row(
            "SELECT id, name FROM cache.version WHERE project_id = ?1 AND released = 0
             ORDER BY release_date IS NULL, release_date, name LIMIT 1",
            [project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

pub fn list_siblings(parent_id: Option<i64>) -> Siblings<'static> {
    match parent_id {
        Some(parent) => Siblings {
            table: "filter_list",
            key: "id",
            of: "parent_id = ?",
            params: vec![rusqlite::types::Value::Integer(parent)],
        },
        // Home: the top-level nodes other than Everything and the Spaces.
        None => Siblings {
            table: "filter_list",
            key: "id",
            of: "parent_id IS NULL AND kind IN ('recent', 'pinned', 'list')",
            params: Vec::new(),
        },
    }
}

/// Appends a filter list with one List view.
pub fn insert_list(
    conn: &Connection,
    parent_id: Option<i64>,
    name: &str,
    filter: &Filter,
) -> AppResult<i64> {
    let position = list_siblings(parent_id).end(conn)?;
    conn.execute(
        "INSERT INTO filter_list (parent_id, kind, name, filter, position) VALUES (?1, 'list', ?2, ?3, ?4)",
        params![parent_id, name, filter.to_json()?, position],
    )?;
    let id = conn.last_insert_rowid();
    let columns = columns_for_new_view(conn, id)?;
    insert_view(conn, id, LIST_VIEW, Layout::List, GroupBy::Status, &columns)?;
    Ok(id)
}
