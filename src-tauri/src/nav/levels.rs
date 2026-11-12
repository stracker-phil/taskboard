//! The sidebar's levels as navigation sees them: Everything, the tracked Spaces, a section's levels in order and
//! the Space above a level.

use rusqlite::{Connection, OptionalExtension};

use crate::error::AppResult;
use crate::lists::{Kind, Node, node};

pub(super) fn everything(conn: &Connection) -> AppResult<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM filter_list WHERE kind = 'everything' ORDER BY id LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

/// The Space node of tracked projects, in sidebar order.
pub(super) fn tracked_space_nodes(conn: &Connection) -> AppResult<Vec<i64>> {
    let mut query = conn.prepare(
        "SELECT f.id FROM space s
         JOIN cache.space p ON p.id = s.project_id
         JOIN filter_list f ON f.kind = 'space' AND f.project_id = s.project_id
         WHERE s.tracked = 1 ORDER BY s.position, p.key",
    )?;
    let ids = query.query_map([], |row| row.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
}

/// A section's levels in sidebar order (each before its children): the Space `space` and its lists, or without one
/// every level outside the Spaces, Everything first.
pub(super) fn section_levels(nodes: &[Node], space: Option<i64>) -> Vec<i64> {
    fn walk(nodes: &[Node], parent: Option<i64>, out: &mut Vec<i64>) {
        for child in nodes.iter().filter(|n| n.parent_id == parent) {
            if parent.is_none() && matches!(child.kind, Kind::Space | Kind::Everything) {
                continue;
            }
            out.push(child.id);
            walk(nodes, Some(child.id), out);
        }
    }
    let mut levels = Vec::from_iter(space);
    if space.is_none() {
        levels.extend(
            nodes
                .iter()
                .filter(|n| n.kind == Kind::Everything)
                .map(|n| n.id),
        );
    }
    walk(nodes, space, &mut levels);
    levels
}

/// The Space a level belongs to: itself, or the Space above a filter list.
pub(super) fn space_of(conn: &Connection, list_id: i64) -> AppResult<Option<i64>> {
    let mut id = Some(list_id);
    while let Some(current) = id {
        let level = node(conn, current)?;
        if level.kind == Kind::Space {
            return Ok(Some(level.id));
        }
        id = level.parent_id;
    }
    Ok(None)
}
