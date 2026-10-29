//! Copying views: a duplicated tab, or every tab of a duplicated filter list, with all their settings,
//! their own conditions and their Me chip.

use rusqlite::Connection;
use rusqlite::types::Value as SqlValue;

use super::display::{
    write_inline_labels, write_look, write_open_with, write_subtasks, write_wrap,
};
use super::edit::{
    insert_view, list_views, read_view, siblings, write_collapsed, write_sort, write_widths,
};
use super::filter::{write_filter, write_only_mine};
use super::model::View;
use crate::db::{Db, LocalPart};
use crate::error::AppResult;

pub fn copy_views(conn: &Connection, from: i64, to: i64) -> AppResult<()> {
    for view in list_views(conn, from)? {
        copy_view(conn, &view, to, &view.name)?;
    }
    Ok(())
}

fn copy_view(conn: &Connection, view: &View, list_id: i64, name: &str) -> AppResult<i64> {
    let id = insert_view(
        conn,
        list_id,
        name,
        view.layout,
        view.group_by,
        &view.columns,
    )?;
    write_sort(conn, id, view.sort)?;
    write_widths(conn, id, &view.widths)?;
    write_collapsed(conn, id, &view.collapsed)?;
    write_wrap(conn, id, view.wrap_titles)?;
    write_inline_labels(conn, id, view.inline_labels)?;
    write_open_with(conn, id, view.open_with)?;
    write_subtasks(conn, id, view.subtasks)?;
    write_look(conn, id, view.icon.as_deref(), view.color.as_deref())?;
    write_filter(conn, id, view.filter.as_deref())?;
    write_only_mine(conn, id, view.only_mine)?;
    Ok(id)
}

/// A copy right after the original, named "<name> copy".
pub async fn view_duplicate(db: &Db, id: i64) -> AppResult<View> {
    db.write(move |tx, changes| {
        let view = read_view(tx, id)?;
        let copy = copy_view(tx, &view, view.list_id, &format!("{} copy", view.name))?;
        siblings(view.list_id).move_to(tx, SqlValue::Integer(copy), view.position + 1)?;
        changes.local_changed(LocalPart::Views);
        read_view(tx, copy)
    })
    .await
}
