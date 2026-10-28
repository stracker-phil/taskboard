//! Reading and changing the view tabs of a sidebar level. Every change keeps positions dense.

use std::collections::BTreeMap;

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::model::{DEFAULT_COLUMNS, GroupBy, Layout, Sort, SortBy, View, from_text, to_text};
use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::lists::{Kind, node};
use crate::order::Siblings;

const COLUMNS: &str = "id, list_id, name, layout, columns, group_by, sort_by, sort_order, position, is_default, widths, wrap_titles, collapsed";

fn siblings(list_id: i64) -> Siblings<'static> {
    Siblings {
        table: "view",
        key: "id",
        of: "list_id = ?",
        params: vec![SqlValue::Integer(list_id)],
    }
}

fn from_row(row: &Row) -> rusqlite::Result<View> {
    let columns: Option<String> = row.get(4)?;
    Ok(View {
        id: row.get(0)?,
        list_id: row.get(1)?,
        name: row.get(2)?,
        layout: from_text(&row.get::<_, String>(3)?).unwrap_or(Layout::List),
        columns: columns
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_else(default_columns),
        group_by: from_text(&row.get::<_, String>(5)?).unwrap_or(GroupBy::None),
        sort: Sort {
            by: from_text(&row.get::<_, String>(6)?).unwrap_or(SortBy::Updated),
            descending: row.get::<_, i64>(7)? < 0,
        },
        position: row.get(8)?,
        is_default: row.get(9)?,
        widths: row
            .get::<_, Option<String>>(10)?
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default(),
        wrap_titles: row.get(11)?,
        collapsed: row
            .get::<_, Option<String>>(12)?
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default(),
    })
}

fn default_columns() -> Vec<String> {
    DEFAULT_COLUMNS.iter().map(|c| c.to_string()).collect()
}

pub fn read_view(conn: &Connection, id: i64) -> AppResult<View> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM view WHERE id = ?1"),
        [id],
        from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("View {id} doesn't exist")))
}

pub fn list_views(conn: &Connection, list_id: i64) -> AppResult<Vec<View>> {
    let mut query = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM view WHERE list_id = ?1 ORDER BY position, id"
    ))?;
    let views = query.query_map([list_id], from_row)?;
    Ok(views.collect::<Result<_, _>>()?)
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
    Ok(id)
}

/// The Space's columns for new views, else the app's.
pub fn columns_for_new_view(conn: &Connection, list_id: i64) -> AppResult<Vec<String>> {
    let stored: Option<String> = conn
        .query_row(
            "WITH RECURSIVE up (id, parent_id, kind, project_id) AS (
               SELECT id, parent_id, kind, project_id FROM filter_list WHERE id = ?1
               UNION ALL
               SELECT f.id, f.parent_id, f.kind, f.project_id FROM filter_list f JOIN up ON f.id = up.parent_id
             )
             SELECT s.default_columns FROM up JOIN space s ON s.project_id = up.project_id
             WHERE up.kind = 'space'",
            [list_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    Ok(stored
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_else(default_columns))
}

fn group_for_new_view(conn: &Connection, list_id: i64) -> AppResult<GroupBy> {
    Ok(match node(conn, list_id)?.kind {
        Kind::Everything | Kind::Recent => GroupBy::Project,
        Kind::Pinned => GroupBy::None,
        Kind::Space | Kind::List => GroupBy::Status,
    })
}

fn sort_order(descending: bool) -> i64 {
    if descending { -1 } else { 1 }
}

fn write_sort(conn: &Connection, id: i64, sort: Sort) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET sort_by = ?2, sort_order = ?3 WHERE id = ?1",
        params![id, to_text(sort.by), sort_order(sort.descending)],
    )?;
    Ok(())
}

fn write_wrap(conn: &Connection, id: i64, wrap: bool) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET wrap_titles = ?2 WHERE id = ?1",
        params![id, wrap],
    )?;
    Ok(())
}

fn write_collapsed(
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

fn write_widths(conn: &Connection, id: i64, widths: &BTreeMap<String, i64>) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET widths = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(widths)?],
    )?;
    Ok(())
}

/// Narrowest and widest a Table column can be dragged.
const MIN_WIDTH: i64 = 48;
const MAX_WIDTH: i64 = 800;

pub fn clean_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput("The name can't be empty".into()));
    }
    Ok(name.to_string())
}

fn clean_columns(columns: Vec<String>) -> AppResult<Vec<String>> {
    let mut clean: Vec<String> = Vec::with_capacity(columns.len());
    for column in columns {
        let column = column.trim().to_string();
        if column.is_empty() {
            return Err(AppError::InvalidInput("A column id can't be empty".into()));
        }
        if !clean.contains(&column) {
            clean.push(column);
        }
    }
    Ok(clean)
}

pub async fn views_of(db: &Db, list_id: i64) -> AppResult<Vec<View>> {
    db.read(move |conn| list_views(conn, list_id)).await
}

pub async fn view_create(db: &Db, list_id: i64, name: String, layout: Layout) -> AppResult<View> {
    let name = clean_name(&name)?;
    db.write(move |tx, changes| {
        let group_by = group_for_new_view(tx, list_id)?;
        let columns = columns_for_new_view(tx, list_id)?;
        let id = insert_view(tx, list_id, &name, layout, group_by, &columns)?;
        changes.local_changed(LocalPart::Views);
        read_view(tx, id)
    })
    .await
}

/// Errors with NotFound when the view doesn't exist.
async fn edit<F>(db: &Db, id: i64, change: F) -> AppResult<View>
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

pub async fn view_set_layout(db: &Db, id: i64, layout: Layout) -> AppResult<View> {
    edit(db, id, move |conn| {
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

/// Clamps each width to the Table column bounds.
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
    let widths: BTreeMap<String, i64> = widths
        .into_iter()
        .map(|(column, width)| (column, width.clamp(MIN_WIDTH, MAX_WIDTH)))
        .collect();
    edit(db, id, move |conn| write_widths(conn, id, &widths)).await
}

pub async fn view_set_wrap(db: &Db, id: i64, wrap: bool) -> AppResult<View> {
    edit(db, id, move |conn| write_wrap(conn, id, wrap)).await
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
