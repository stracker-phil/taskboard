//! How a view shows its issues beyond columns and grouping: wrapped titles and where a click opens an issue.

use rusqlite::{Connection, params};

use super::edit::edit;
use super::model::{IssueOpening, View, to_text};
use crate::db::Db;
use crate::error::AppResult;

pub(super) fn write_wrap(conn: &Connection, id: i64, wrap: bool) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET wrap_titles = ?2 WHERE id = ?1",
        params![id, wrap],
    )?;
    Ok(())
}

pub(super) fn write_open_with(
    conn: &Connection,
    id: i64,
    open_with: Option<IssueOpening>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET open_with = ?2 WHERE id = ?1",
        params![id, open_with.map(to_text)],
    )?;
    Ok(())
}

pub async fn view_set_wrap(db: &Db, id: i64, wrap: bool) -> AppResult<View> {
    edit(db, id, move |conn| write_wrap(conn, id, wrap)).await
}

/// None follows Settings → General.
pub async fn view_set_open_with(
    db: &Db,
    id: i64,
    open_with: Option<IssueOpening>,
) -> AppResult<View> {
    edit(db, id, move |conn| write_open_with(conn, id, open_with)).await
}
