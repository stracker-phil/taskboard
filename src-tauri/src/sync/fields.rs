//! The custom fields the sync stores and asks for: the site's field list, which ones the tracked projects fill,
//! and which ones the owner opted in or uses.

use std::collections::BTreeSet;

use rusqlite::{Connection, params};

use super::map::FieldIds;
use crate::error::AppResult;
use crate::filter::FIELDS_IN_USE;
use crate::jira::dto::Field;

const SPRINT_TYPE: &str = "com.pyxis.greenhopper.jira:gh-sprint";
const EPIC_COLOR_TYPE: &str = "com.pyxis.greenhopper.jira:gh-epic-color";

/// Stores the field list and finds the site's Sprint and Epic Color fields.
pub fn store_fields(conn: &Connection, fields: &[Field]) -> AppResult<FieldIds> {
    let mut ids = FieldIds::default();
    for field in fields {
        match field.schema.as_ref().and_then(|s| s.custom.as_deref()) {
            Some(SPRINT_TYPE) => ids.sprint = Some(field.id.clone()),
            Some(EPIC_COLOR_TYPE) => ids.epic_color = Some(field.id.clone()),
            _ => {}
        }
        conn.execute(
            "INSERT INTO cache.field (id, name, type) VALUES (?1, ?2, ?3)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, type = excluded.type",
            params![field.id, field.name, field.schema.as_ref().map(|s| &s.kind)],
        )?;
    }
    Ok(ids)
}

/// Marks which custom fields the tracked projects' issues fill; the rest drop out of pickers (`filter::RELEVANT_FIELD`).
pub fn store_field_usage(conn: &Connection, used: &BTreeSet<String>) -> AppResult<()> {
    let used = serde_json::to_string(used)?;
    conn.execute(
        "UPDATE cache.field SET used = (id IN (SELECT value FROM json_each(?1)))
         WHERE id LIKE 'customfield\\_%' ESCAPE '\\'",
        [used],
    )?;
    Ok(())
}

/// Custom fields fetched with every issue: those opted in app-wide, those a view or filter uses, and the ones
/// the tracked projects fill (once a sample has marked some unused; before that `used` is 1 for every field).
pub fn fetched_fields(conn: &Connection) -> AppResult<Vec<String>> {
    let mut query = conn.prepare(&format!(
        "SELECT field_id FROM field_choice
         UNION SELECT value FROM ({FIELDS_IN_USE}) WHERE value LIKE 'customfield\\_%' ESCAPE '\\'
         UNION SELECT id FROM cache.field
         WHERE used = 1 AND id LIKE 'customfield\\_%' ESCAPE '\\'
           AND EXISTS (SELECT 1 FROM cache.field WHERE used = 0)
         ORDER BY 1"
    ))?;
    let ids = query.query_map([], |row| row.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
}
