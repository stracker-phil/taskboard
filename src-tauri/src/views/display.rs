//! How a view shows beyond columns and grouping: wrapped titles, where a click opens an issue, its tab's look.

use rusqlite::{Connection, params};

use super::edit::edit;
use super::model::{IssueOpening, View, to_text};
use crate::db::Db;
use crate::error::AppResult;
use crate::look::{clean_color, clean_icon};

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

pub(super) fn write_look(
    conn: &Connection,
    id: i64,
    icon: Option<&str>,
    color: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE view SET icon = ?2, color = ?3 WHERE id = ?1",
        params![id, icon, color],
    )?;
    Ok(())
}

/// The tab's icon and tile color; no icon shows the layout's, no color shows the icon plain.
pub async fn view_set_look(
    db: &Db,
    id: i64,
    icon: Option<String>,
    color: Option<String>,
) -> AppResult<View> {
    let icon = clean_icon(icon)?;
    let color = clean_color(color)?;
    edit(db, id, move |conn| {
        write_look(conn, id, icon.as_deref(), color.as_deref())
    })
    .await
}
