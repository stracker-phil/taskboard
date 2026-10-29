//! A view's own conditions (ADR 0033): ANDed with its list's chain, so two views of one list can show
//! different issues.

use rusqlite::{Connection, params};

use super::edit::{edit, read_view};
use super::model::View;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::filter::{Condition, Filter, compile};
use crate::lists::{Viewer, all_nodes, mark_fields_shown, node, scope_of};

pub(super) fn write_filter(conn: &Connection, id: i64, filter: Option<&str>) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET filter = ?2 WHERE id = ?1",
        params![id, filter],
    )?;
    Ok(())
}

/// The view's own conditions; an error names the view when its stored filter can't be read.
pub fn view_conditions(view: &View) -> AppResult<Vec<Condition>> {
    Filter::parse(view.filter.as_deref())
        .map(|filter| filter.conditions)
        .map_err(|e| AppError::InvalidInput(format!("The view “{}”: {e}", view.name)))
}

/// Compiled under its list's chain first, so a stored view filter always compiles.
pub async fn view_set_filter(db: &Db, viewer: Viewer, id: i64, filter: Filter) -> AppResult<View> {
    edit(db, id, move |conn| {
        let view = read_view(conn, id)?;
        let nodes = all_nodes(conn)?;
        let mut conditions = scope_of(&nodes, &node(conn, view.list_id)?).all()?;
        conditions.extend(filter.conditions.iter().cloned());
        compile(&conditions, &viewer.ctx(conn)?)?;
        mark_fields_shown(conn, &filter)?;
        write_filter(conn, id, filter.to_json()?.as_deref())
    })
    .await
}
