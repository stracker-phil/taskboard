//! Values of a custom field that is no longer fetched leave the cache, so it holds only the fields in use.

use rusqlite::{Connection, params};

use crate::db::Changes;
use crate::error::AppResult;

/// Removes `field_id` from every cached issue's custom values and from the rendered text `issue_body.raw` keeps.
pub fn forget_values(conn: &Connection, changes: &mut Changes, field_id: &str) -> AppResult<()> {
    let custom = format!("$.\"{field_id}\"");
    let rendered = format!("$.renderedFields.\"{field_id}\"");
    let ids: Vec<String> = {
        let mut query = conn.prepare(
            "SELECT id FROM cache.issue WHERE json_type(custom, ?1) IS NOT NULL
             UNION SELECT issue_id FROM cache.issue_body WHERE json_type(raw, ?2) IS NOT NULL",
        )?;
        query
            .query_map(params![custom, rendered], |row| row.get(0))?
            .collect::<Result<_, _>>()?
    };
    // An issue without custom values stores NULL, as `sync::raw::custom_values` does.
    conn.execute(
        "UPDATE cache.issue SET custom = NULLIF(json_remove(custom, ?1), '{}')
         WHERE json_type(custom, ?1) IS NOT NULL",
        [&custom],
    )?;
    conn.execute(
        "UPDATE cache.issue_body SET raw = json_remove(raw, ?1) WHERE json_type(raw, ?1) IS NOT NULL",
        [&rendered],
    )?;
    for id in &ids {
        changes.issue_changed(id);
    }
    Ok(())
}
