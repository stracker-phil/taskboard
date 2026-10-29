//! Reading and changing the view tabs of a sidebar level. Every change keeps positions dense.

use std::collections::{BTreeMap, HashSet};

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::model::{DEFAULT_COLUMNS, GroupBy, Layout, Sort, SortBy, View, from_text, to_text};
use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::filter::Filter;
use crate::lists::{Kind, node};
use crate::order::Siblings;
use crate::space_config::{ALL_LAYOUTS, SpaceLayouts, hidden_columns, level_layouts};

const COLUMNS: &str = "id, list_id, name, layout, columns, group_by, sort_by, sort_order, position, is_default, widths, wrap_titles, collapsed, open_with, icon, color, inline_labels, filter, subtasks, only_mine";

pub(super) fn siblings(list_id: i64) -> Siblings<'static> {
    Siblings {
        table: "view",
        key: "id",
        of: "list_id = ?",
        params: vec![SqlValue::Integer(list_id)],
    }
}

fn from_row(row: &Row) -> rusqlite::Result<View> {
    let columns: Option<String> = row.get(4)?;
    let filter: Option<String> = row.get(17)?;
    Ok(View {
        id: row.get(0)?,
        list_id: row.get(1)?,
        name: row.get(2)?,
        layout: from_text(&row.get::<_, String>(3)?).unwrap_or(Layout::List),
        layouts: ALL_LAYOUTS.to_vec(),
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
        open_with: row
            .get::<_, Option<String>>(13)?
            .and_then(|text| from_text(&text)),
        icon: row.get(14)?,
        color: row.get(15)?,
        inline_labels: row.get(16)?,
        subtasks: row
            .get::<_, Option<String>>(18)?
            .and_then(|text| from_text(&text))
            .unwrap_or_default(),
        only_mine: row.get(19)?,
        own_conditions: Filter::parse(filter.as_deref()).map_or(0, |f| f.conditions.len()),
        filter,
    })
}

fn default_columns() -> Vec<String> {
    DEFAULT_COLUMNS.iter().map(|c| c.to_string()).collect()
}

/// A view as its Space lets it show: an offered layout, and none of the columns the Space hides from lists.
fn fit(mut view: View, layouts: &SpaceLayouts, hidden: &HashSet<String>) -> View {
    view.layout = layouts.shown(view.layout);
    view.layouts = layouts.layouts.clone();
    view.columns.retain(|column| !hidden.contains(column));
    view
}

pub fn read_view(conn: &Connection, id: i64) -> AppResult<View> {
    let view = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM view WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("View {id} doesn't exist")))?;
    let layouts = level_layouts(conn, view.list_id)?;
    let hidden = hidden_columns(conn, view.list_id)?;
    Ok(fit(view, &layouts, &hidden))
}

pub fn list_views(conn: &Connection, list_id: i64) -> AppResult<Vec<View>> {
    let mut query = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM view WHERE list_id = ?1 ORDER BY position, id"
    ))?;
    let views = query.query_map([list_id], from_row)?;
    let layouts = level_layouts(conn, list_id)?;
    let hidden = hidden_columns(conn, list_id)?;
    views
        .map(|view| Ok(fit(view?, &layouts, &hidden)))
        .collect()
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
        Kind::Space | Kind::List => level_layouts(conn, list_id)?.default_group,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::space_config::{FieldVisibility, space_set_field, space_set_layouts};

    const A: &str = "10001";
    const B: &str = "10002";
    const ZONE: &str = "customfield_10001";

    /// Two tracked Spaces (each with its Space views and default lists) and one custom field.
    async fn lab() -> (tempfile::TempDir, Db) {
        let dir = crate::test_support::temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| {
            tx.execute_batch(
                "INSERT INTO cache.space (id, key, name, raw) VALUES
                   ('10001', 'NW', 'Website relaunch', '{}'), ('10002', 'NWS', 'Northwind support', '{}');
                 INSERT INTO cache.field (id, name, type, used) VALUES
                   ('customfield_10001', 'Zone', 'string', 1)",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        for id in [A, B] {
            crate::connection::track_project(&db, id.into(), true)
                .await
                .unwrap();
        }
        (dir, db)
    }

    async fn level(db: &Db, clause: &str) -> i64 {
        let sql = format!("SELECT id FROM filter_list WHERE {clause} ORDER BY id LIMIT 1");
        db.read(move |conn| Ok(conn.query_row(&sql, [], |r| r.get(0))?))
            .await
            .unwrap()
    }

    async fn space(db: &Db, project: &str) -> i64 {
        level(db, &format!("kind = 'space' AND project_id = '{project}'")).await
    }

    async fn list_in_a(db: &Db) -> i64 {
        level(
            db,
            "kind = 'list' AND parent_id IN
               (SELECT id FROM filter_list WHERE kind = 'space' AND project_id = '10001')",
        )
        .await
    }

    async fn everything(db: &Db) -> i64 {
        level(db, "kind = 'everything'").await
    }

    async fn offer(
        db: &Db,
        project: &str,
        layouts: &[Layout],
        default_layout: Layout,
        group: GroupBy,
    ) {
        space_set_layouts(
            db,
            project.into(),
            SpaceLayouts {
                layouts: layouts.to_vec(),
                default_layout,
                default_group: group,
            },
        )
        .await
        .unwrap();
    }

    async fn view_named(db: &Db, list_id: i64, name: &str) -> View {
        views_of(db, list_id)
            .await
            .unwrap()
            .into_iter()
            .find(|v| v.name == name)
            .unwrap()
    }

    async fn stored_layout(db: &Db, id: i64) -> String {
        db.read(move |conn| {
            Ok(conn.query_row("SELECT layout FROM view WHERE id = ?1", [id], |r| r.get(0))?)
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_space_without_settings_shows_its_views_as_stored_with_every_layout() {
        let (_dir, db) = lab().await;
        let space = space(&db, A).await;

        let views = views_of(&db, space).await.unwrap();

        let shown: Vec<(&str, Layout)> =
            views.iter().map(|v| (v.name.as_str(), v.layout)).collect();
        assert_eq!(
            shown,
            [
                ("All issues", Layout::List),
                ("Board", Layout::Board),
                ("Table", Layout::Table)
            ]
        );
        assert!(views.iter().all(|v| v.layouts == ALL_LAYOUTS));
    }

    #[tokio::test]
    async fn a_view_in_a_layout_the_space_switched_off_shows_the_spaces_default_layout() {
        let (_dir, db) = lab().await;
        offer(
            &db,
            A,
            &[Layout::List, Layout::Table],
            Layout::Table,
            GroupBy::Status,
        )
        .await;
        let space = space(&db, A).await;
        let board = view_named(&db, space, "Board").await;

        let id = board.id;
        let read = db.read(move |conn| read_view(conn, id)).await.unwrap();

        assert_eq!(board.layout, Layout::Table);
        assert_eq!(board.layouts, [Layout::List, Layout::Table]);
        assert_eq!(read.layout, Layout::Table);
        assert_eq!(read.layouts, [Layout::List, Layout::Table]);
        assert_eq!(stored_layout(&db, board.id).await, "board");
        assert_eq!(
            view_named(&db, space, "All issues").await.layout,
            Layout::List
        );
    }

    #[tokio::test]
    async fn the_layouts_of_a_filter_list_follow_its_space_and_other_spaces_are_untouched() {
        let (_dir, db) = lab().await;
        offer(&db, A, &[Layout::Board], Layout::Board, GroupBy::Status).await;

        let in_a = views_of(&db, list_in_a(&db).await).await.unwrap();
        let in_b = views_of(&db, space(&db, B).await).await.unwrap();

        assert!(
            in_a.iter()
                .all(|v| v.layouts == [Layout::Board] && v.layout == Layout::Board)
        );
        assert!(in_b.iter().all(|v| v.layouts == ALL_LAYOUTS));
    }

    #[tokio::test]
    async fn everything_keeps_every_layout_whatever_the_spaces_offer() {
        let (_dir, db) = lab().await;
        offer(&db, A, &[Layout::Table], Layout::Table, GroupBy::Status).await;
        offer(&db, B, &[Layout::Table], Layout::Table, GroupBy::Status).await;

        let views = views_of(&db, everything(&db).await).await.unwrap();

        assert!(views.iter().all(|v| v.layouts == ALL_LAYOUTS));
    }

    #[tokio::test]
    async fn a_view_drops_the_columns_its_space_hides_from_lists() {
        let (_dir, db) = lab().await;
        let space = space(&db, A).await;
        let view = view_named(&db, space, "All issues").await;
        view_set_columns(&db, view.id, vec!["status".into(), ZONE.into()])
            .await
            .unwrap();

        for (visibility, columns) in [
            (FieldVisibility::HiddenInList, vec!["status"]),
            (FieldVisibility::Hidden, vec!["status"]),
            (FieldVisibility::HiddenInDetail, vec!["status", ZONE]),
            (FieldVisibility::Visible, vec!["status", ZONE]),
        ] {
            space_set_field(&db, A.into(), ZONE.into(), visibility)
                .await
                .unwrap();

            let found = view_named(&db, space, "All issues").await;

            assert_eq!(found.columns, columns, "{visibility:?}");
        }
    }

    #[tokio::test]
    async fn everything_drops_a_custom_column_only_when_every_tracked_space_hides_it() {
        let (_dir, db) = lab().await;
        let everything = everything(&db).await;
        let view = views_of(&db, everything).await.unwrap().remove(0);
        view_set_columns(&db, view.id, vec!["status".into(), ZONE.into()])
            .await
            .unwrap();
        space_set_field(&db, A.into(), ZONE.into(), FieldVisibility::Hidden)
            .await
            .unwrap();

        let one_hides = views_of(&db, everything).await.unwrap().remove(0);
        space_set_field(&db, B.into(), ZONE.into(), FieldVisibility::HiddenInList)
            .await
            .unwrap();
        let both_hide = views_of(&db, everything).await.unwrap().remove(0);

        assert_eq!(one_hides.columns, ["status", ZONE]);
        assert_eq!(both_hide.columns, ["status"]);
    }

    #[tokio::test]
    async fn a_layout_the_space_doesnt_offer_is_refused() {
        let (_dir, db) = lab().await;
        offer(
            &db,
            A,
            &[Layout::List, Layout::Table],
            Layout::List,
            GroupBy::Status,
        )
        .await;
        let space = space(&db, A).await;
        let view = view_named(&db, space, "All issues").await;

        let err = view_set_layout(&db, view.id, Layout::Board)
            .await
            .unwrap_err();
        let table = view_set_layout(&db, view.id, Layout::Table).await.unwrap();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(table.layout, Layout::Table);
        assert_eq!(stored_layout(&db, view.id).await, "table");
    }

    #[tokio::test]
    async fn a_new_view_asking_for_a_layout_the_space_lacks_gets_the_spaces_default() {
        let (_dir, db) = lab().await;
        offer(
            &db,
            A,
            &[Layout::List, Layout::Table],
            Layout::Table,
            GroupBy::Status,
        )
        .await;
        let space = space(&db, A).await;

        let board = view_create(&db, space, "Wall".into(), Layout::Board)
            .await
            .unwrap();
        let list = view_create(&db, space, "Plain".into(), Layout::List)
            .await
            .unwrap();

        assert_eq!(board.layout, Layout::Table);
        assert_eq!(stored_layout(&db, board.id).await, "table");
        assert_eq!(list.layout, Layout::List);
    }

    #[tokio::test]
    async fn new_views_in_a_space_and_its_lists_group_by_the_spaces_default_group() {
        let (_dir, db) = lab().await;
        offer(&db, A, &ALL_LAYOUTS, Layout::List, GroupBy::Assignee).await;

        let in_space = view_create(&db, space(&db, A).await, "By person".into(), Layout::List)
            .await
            .unwrap();
        let in_list = view_create(&db, list_in_a(&db).await, "By person".into(), Layout::List)
            .await
            .unwrap();
        let other_space = view_create(&db, space(&db, B).await, "Plain".into(), Layout::List)
            .await
            .unwrap();
        let in_everything = view_create(&db, everything(&db).await, "Plain".into(), Layout::List)
            .await
            .unwrap();

        assert_eq!(in_space.group_by, GroupBy::Assignee);
        assert_eq!(in_list.group_by, GroupBy::Assignee);
        assert_eq!(other_space.group_by, GroupBy::Status);
        assert_eq!(in_everything.group_by, GroupBy::Project);
    }
}
