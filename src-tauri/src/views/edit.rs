//! Changing the view tabs of a sidebar level. Every change keeps positions dense.

use std::collections::BTreeMap;

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params};

use super::model::{GroupBy, Layout, Sort, View, to_text};
use super::read::{columns_for_new_view, group_for_new_view, list_views, read_view};
use super::validate::{clean_columns, clean_name, clean_widths};
use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::order::Siblings;
use crate::space_config::level_layouts;

pub(super) fn siblings(list_id: i64) -> Siblings<'static> {
    Siblings {
        table: "view",
        key: "id",
        of: "list_id = ?",
        params: vec![SqlValue::Integer(list_id)],
    }
}

/// Appends a view; the level's first view becomes its default.
pub fn insert_view(
    conn: &Connection,
    list_id: i64,
    name: &str,
    layout: Layout,
    group_by: GroupBy,
    columns: &[String],
) -> AppResult<i64> {
    let position = siblings(list_id).end(conn)?;
    let sort = Sort::default();
    conn.execute(
        "INSERT INTO view (list_id, name, layout, columns, position, is_default, group_by, sort_by, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            list_id,
            name,
            to_text(layout),
            serde_json::to_string(columns)?,
            position,
            position == 0,
            to_text(group_by),
            to_text(sort.by),
            sort_order(sort.descending),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

fn sort_order(descending: bool) -> i64 {
    if descending { -1 } else { 1 }
}

pub(super) fn write_sort(conn: &Connection, id: i64, sort: Sort) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET sort_by = ?2, sort_order = ?3 WHERE id = ?1",
        params![id, to_text(sort.by), sort_order(sort.descending)],
    )?;
    Ok(())
}

pub(super) fn write_collapsed(
    conn: &Connection,
    id: i64,
    collapsed: &BTreeMap<String, bool>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET collapsed = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(collapsed)?],
    )?;
    Ok(())
}

pub(super) fn write_widths(
    conn: &Connection,
    id: i64,
    widths: &BTreeMap<String, i64>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET widths = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(widths)?],
    )?;
    Ok(())
}

pub async fn view_create(db: &Db, list_id: i64, name: String, layout: Layout) -> AppResult<View> {
    let name = clean_name(&name)?;
    db.write(move |tx, changes| {
        let group_by = group_for_new_view(tx, list_id)?;
        let layout = level_layouts(tx, list_id)?.shown(layout);
        let columns = columns_for_new_view(tx, list_id)?;
        let id = insert_view(tx, list_id, &name, layout, group_by, &columns)?;
        changes.local_changed(LocalPart::Views);
        read_view(tx, id)
    })
    .await
}

/// Errors with NotFound when the view doesn't exist.
pub(super) async fn edit<F>(db: &Db, id: i64, change: F) -> AppResult<View>
where
    F: FnOnce(&Connection) -> AppResult<()> + Send + 'static,
{
    db.write(move |tx, changes| {
        read_view(tx, id)?;
        change(tx)?;
        changes.local_changed(LocalPart::Views);
        read_view(tx, id)
    })
    .await
}

pub async fn view_rename(db: &Db, id: i64, name: String) -> AppResult<View> {
    let name = clean_name(&name)?;
    edit(db, id, move |conn| {
        conn.execute("UPDATE view SET name = ?2 WHERE id = ?1", params![id, name])?;
        Ok(())
    })
    .await
}

pub async fn view_set_layout(db: &Db, id: i64, layout: Layout) -> AppResult<View> {
    edit(db, id, move |conn| {
        let view = read_view(conn, id)?;
        if !view.layouts.contains(&layout) {
            return Err(AppError::InvalidInput(
                "This Space doesn't offer that layout; turn it on in the Space's settings.".into(),
            ));
        }
        conn.execute(
            "UPDATE view SET layout = ?2 WHERE id = ?1",
            params![id, to_text(layout)],
        )?;
        Ok(())
    })
    .await
}

pub async fn view_set_columns(db: &Db, id: i64, columns: Vec<String>) -> AppResult<View> {
    let columns = clean_columns(columns)?;
    edit(db, id, move |conn| {
        conn.execute(
            "UPDATE view SET columns = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&columns)?],
        )?;
        Ok(())
    })
    .await
}

pub async fn view_set_group(db: &Db, id: i64, group_by: GroupBy) -> AppResult<View> {
    edit(db, id, move |conn| {
        conn.execute(
            "UPDATE view SET group_by = ?2 WHERE id = ?1",
            params![id, to_text(group_by)],
        )?;
        Ok(())
    })
    .await
}

pub async fn view_set_sort(db: &Db, id: i64, sort: Sort) -> AppResult<View> {
    edit(db, id, move |conn| write_sort(conn, id, sort)).await
}

/// Collapses or expands one Board column.
pub async fn view_set_column_collapsed(
    db: &Db,
    id: i64,
    status_id: String,
    collapsed: bool,
) -> AppResult<View> {
    edit(db, id, move |conn| {
        let mut all = read_view(conn, id)?.collapsed;
        all.insert(status_id, collapsed);
        write_collapsed(conn, id, &all)
    })
    .await
}

pub async fn view_set_widths(db: &Db, id: i64, widths: BTreeMap<String, i64>) -> AppResult<View> {
    let widths = clean_widths(widths);
    edit(db, id, move |conn| write_widths(conn, id, &widths)).await
}

pub async fn view_move(db: &Db, id: i64, position: i64) -> AppResult<Vec<View>> {
    db.write(move |tx, changes| {
        let view = read_view(tx, id)?;
        siblings(view.list_id).move_to(tx, SqlValue::Integer(id), position)?;
        changes.local_changed(LocalPart::Views);
        list_views(tx, view.list_id)
    })
    .await
}

/// A level keeps at least one view; when the default goes, the first one left takes over.
pub async fn view_delete(db: &Db, id: i64) -> AppResult<Vec<View>> {
    db.write(move |tx, changes| {
        let view = read_view(tx, id)?;
        if list_views(tx, view.list_id)?.len() <= 1 {
            return Err(AppError::InvalidInput(
                "The last view of a list can't be deleted".into(),
            ));
        }
        tx.execute("DELETE FROM view WHERE id = ?1", [id])?;
        let order = siblings(view.list_id);
        order.renumber(tx)?;
        if view.is_default {
            tx.execute(
                "UPDATE view SET is_default = (position = 0) WHERE list_id = ?1",
                [view.list_id],
            )?;
        }
        changes.local_changed(LocalPart::Views);
        list_views(tx, view.list_id)
    })
    .await
}

/// "Use as Space default": the columns new views in this Space start with.
pub async fn space_default_columns(
    db: &Db,
    project_id: String,
    columns: Vec<String>,
) -> AppResult<()> {
    let columns = clean_columns(columns)?;
    db.write(move |tx, changes| {
        let updated = tx.execute(
            "UPDATE space SET default_columns = ?2 WHERE project_id = ?1",
            params![project_id, serde_json::to_string(&columns)?],
        )?;
        if updated == 0 {
            return Err(AppError::NotFound(format!(
                "Project {project_id} has no Space"
            )));
        }
        changes.local_changed(LocalPart::Spaces);
        Ok(())
    })
    .await
}
