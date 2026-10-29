//! Creating and changing filter lists and Spaces. Every change keeps positions dense.

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::context::Viewer;
use super::defaults::{ensure_space, insert_list, list_siblings};
use super::nodes::{Kind, Node, all_nodes, node, scope_of};
use crate::db::{Changes, Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::filter::{Field, Filter, OPEN_ISSUES, TOP_LEVEL, compile};
use crate::look::{clean_color, clean_icon};
use crate::order::Siblings;
use crate::views::{clean_name, copy_views};

/// Only filter lists can be renamed, duplicated, moved, filtered or deleted here.
fn filter_list(conn: &Connection, id: i64) -> AppResult<Node> {
    let found = node(conn, id)?;
    if found.kind != Kind::List {
        return Err(AppError::InvalidInput(format!(
            "“{}” isn't a filter list",
            found.name
        )));
    }
    Ok(found)
}

/// Compiles the filter under `parent`, so a stored filter always compiles.
fn check_filter(
    conn: &Connection,
    viewer: &Viewer,
    parent: Option<&Node>,
    filter: &Filter,
) -> AppResult<()> {
    let nodes = all_nodes(conn)?;
    let mut conditions = match parent {
        Some(parent) => scope_of(&nodes, parent).all()?,
        None => Vec::new(),
    };
    conditions.extend(filter.conditions.iter().cloned());
    compile(&conditions, &viewer.ctx(conn)?)?;
    Ok(())
}

/// The next sync fetches the custom fields a filter uses.
pub fn mark_fields_shown(conn: &Connection, filter: &Filter) -> AppResult<()> {
    for condition in &filter.conditions {
        if let Field::Custom(id) = &condition.field {
            conn.execute("UPDATE cache.field SET shown = 1 WHERE id = ?1", [id])?;
        }
    }
    Ok(())
}

fn lists_and_views_changed(changes: &mut Changes) {
    changes.local_changed(LocalPart::Lists);
    changes.local_changed(LocalPart::Views);
}

/// A new filter list under a Space, or a global one under Home when `parent_id` is Everything.
pub async fn list_create(
    db: &Db,
    viewer: Viewer,
    parent_id: i64,
    name: String,
    filter: Option<Filter>,
) -> AppResult<i64> {
    let name = clean_name(&name)?;
    let filter = filter.unwrap_or_default();
    db.write(move |tx, changes| {
        let parent = node(tx, parent_id)?;
        let parent = match parent.kind {
            Kind::Space => Some(parent),
            Kind::Everything => None,
            _ => {
                return Err(AppError::InvalidInput(format!(
                    "A filter list can't go under “{}”",
                    parent.name
                )));
            }
        };
        check_filter(tx, &viewer, parent.as_ref(), &filter)?;
        mark_fields_shown(tx, &filter)?;
        let id = insert_list(tx, parent.map(|p| p.id), &name, &filter)?;
        lists_and_views_changed(changes);
        Ok(id)
    })
    .await
}

pub async fn list_rename(db: &Db, id: i64, name: String) -> AppResult<()> {
    let name = clean_name(&name)?;
    db.write(move |tx, changes| {
        filter_list(tx, id)?;
        tx.execute(
            "UPDATE filter_list SET name = ?2 WHERE id = ?1",
            params![id, name],
        )?;
        changes.local_changed(LocalPart::Lists);
        Ok(())
    })
    .await
}

/// A filter list's look; no icon is the funnel, no color shows the icon plain.
pub async fn list_set_look(
    db: &Db,
    id: i64,
    icon: Option<String>,
    color: Option<String>,
) -> AppResult<()> {
    let icon = clean_icon(icon)?;
    let color = clean_color(color)?;
    db.write(move |tx, changes| {
        filter_list(tx, id)?;
        tx.execute(
            "UPDATE filter_list SET icon = ?2, color = ?3 WHERE id = ?1",
            params![id, icon, color],
        )?;
        changes.local_changed(LocalPart::Lists);
        Ok(())
    })
    .await
}

/// A copy with its views right after the original, named "<name> copy".
pub async fn list_duplicate(db: &Db, id: i64) -> AppResult<i64> {
    db.write(move |tx, changes| {
        let original = filter_list(tx, id)?;
        let order = list_siblings(original.parent_id);
        tx.execute(
            "INSERT INTO filter_list (parent_id, kind, name, filter, position, icon, color)
             VALUES (?1, 'list', ?2, ?3, ?4, ?5, ?6)",
            params![
                original.parent_id,
                format!("{} copy", original.name),
                original.filter,
                order.end(tx)?,
                original.icon,
                original.color
            ],
        )?;
        let copy = tx.last_insert_rowid();
        copy_views(tx, id, copy)?;
        order.move_to(tx, SqlValue::Integer(copy), original.position + 1)?;
        lists_and_views_changed(changes);
        Ok(copy)
    })
    .await
}

pub async fn list_delete(db: &Db, id: i64) -> AppResult<()> {
    db.write(move |tx, changes| {
        let list = filter_list(tx, id)?;
        tx.execute(
            "DELETE FROM pin WHERE target_kind = 'filter_list' AND target_id = ?1",
            [id.to_string()],
        )?;
        tx.execute("DELETE FROM view WHERE list_id = ?1", [id])?;
        tx.execute("DELETE FROM filter_list WHERE id = ?1", [id])?;
        list_siblings(list.parent_id).renumber(tx)?;
        lists_and_views_changed(changes);
        changes.local_changed(LocalPart::Pins);
        Ok(())
    })
    .await
}

pub async fn list_move(db: &Db, id: i64, position: i64) -> AppResult<()> {
    db.write(move |tx, changes| {
        let list = filter_list(tx, id)?;
        list_siblings(list.parent_id).move_to(tx, SqlValue::Integer(id), position)?;
        changes.local_changed(LocalPart::Lists);
        Ok(())
    })
    .await
}

pub async fn list_set_filter(db: &Db, viewer: Viewer, id: i64, filter: Filter) -> AppResult<()> {
    db.write(move |tx, changes| {
        let list = filter_list(tx, id)?;
        let parent = list.parent_id.map(|p| node(tx, p)).transpose()?;
        check_filter(tx, &viewer, parent.as_ref(), &filter)?;
        mark_fields_shown(tx, &filter)?;
        tx.execute(
            "UPDATE filter_list SET filter = ?2 WHERE id = ?1",
            params![id, filter.to_json()?],
        )?;
        changes.local_changed(LocalPart::Lists);
        Ok(())
    })
    .await
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct MatchCount {
    pub open: i64,
    pub total: i64,
}

/// The Filter popover's live count: what `node_id` inherits ANDed with `draft` in place of its own
/// conditions. On a Space or Everything the draft is a new filter list's. Top-level issues only, like the
/// sidebar's counts (ADR 0037).
pub async fn list_match_count(
    db: &Db,
    viewer: Viewer,
    node_id: i64,
    draft: Filter,
) -> AppResult<MatchCount> {
    db.read(move |conn| {
        let nodes = all_nodes(conn)?;
        let target = node(conn, node_id)?;
        let mut conditions = scope_of(&nodes, &target).inherited;
        conditions.extend(draft.conditions);
        let compiled = compile(&conditions, &viewer.ctx(conn)?)?;
        let sql = format!(
            "SELECT COUNT(*) FILTER (WHERE {OPEN_ISSUES}), COUNT(*) FROM cache.issue i WHERE {} AND {TOP_LEVEL}",
            compiled.where_clause()
        );
        Ok(
            conn.query_row(&sql, params_from_iter(&compiled.params), |row| {
                Ok(MatchCount {
                    open: row.get(0)?,
                    total: row.get(1)?,
                })
            })?,
        )
    })
    .await
}

pub async fn space_move(db: &Db, project_id: String, position: i64) -> AppResult<()> {
    db.write(move |tx, changes| {
        require_space(tx, &project_id)?;
        let spaces = Siblings {
            table: "space",
            key: "project_id",
            of: "tracked = 1",
            params: Vec::new(),
        };
        spaces.move_to(tx, SqlValue::Text(project_id), position)?;
        changes.local_changed(LocalPart::Spaces);
        Ok(())
    })
    .await
}

/// The owner's names for a Space; empty values go back to the project's.
#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SpaceUpdate {
    pub name: Option<String>,
    /// One letter, digit or emoji, or a line icon's name (`look`).
    pub icon: Option<String>,
    /// A tile color number (`look::TILE_COLORS`).
    pub color: Option<String>,
    pub client: Option<String>,
}

pub async fn space_update(db: &Db, project_id: String, update: SpaceUpdate) -> AppResult<()> {
    let blank_is_none = |value: Option<String>| {
        value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let icon = clean_icon(update.icon)?;
    let color = clean_color(update.color)?;
    let name = blank_is_none(update.name);
    let client = blank_is_none(update.client);
    db.write(move |tx, changes| {
        require_space(tx, &project_id)?;
        // A name equal to the project's isn't the owner's own, so a renamed project renames the Space too.
        let project_name: Option<String> = tx
            .query_row(
                "SELECT name FROM cache.space WHERE id = ?1",
                [&project_id],
                |row| row.get(0),
            )
            .optional()?;
        let name = name.filter(|name| Some(name) != project_name.as_ref());
        tx.execute(
            "UPDATE space SET name = ?2, icon = ?3, color = ?4, client = ?5 WHERE project_id = ?1",
            params![project_id, name, icon, color, client],
        )?;
        changes.local_changed(LocalPart::Spaces);
        Ok(())
    })
    .await
}

fn require_space(conn: &Connection, project_id: &str) -> AppResult<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM space WHERE project_id = ?1)",
        [project_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!(
            "Project {project_id} has no Space"
        )));
    }
    Ok(())
}

/// Creates the Space node and default lists for every tracked project that lacks them.
pub async fn ensure_tracked_spaces(db: &Db) -> AppResult<()> {
    db.write(|tx, changes| {
        let ids: Vec<String> = {
            let mut query = tx.prepare("SELECT project_id FROM space WHERE tracked = 1")?;
            let ids = query.query_map([], |row| row.get(0))?;
            ids.collect::<Result<_, _>>()?
        };
        let mut created = false;
        for id in ids {
            created |= ensure_space(tx, &id)?;
        }
        if created {
            lists_and_views_changed(changes);
        }
        Ok(())
    })
    .await
}
