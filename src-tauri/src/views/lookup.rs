//! What grouping looks up besides the rows: the tracked Spaces and the releases.

use std::collections::HashMap;

use rusqlite::Connection;

use super::groups::{SpaceInfo, Version};
use crate::error::AppResult;

/// The owner's icon letter, else the name's first letter.
fn space_letter(icon: Option<String>, name: &str) -> String {
    icon.filter(|i| !i.is_empty())
        .unwrap_or_else(|| name.chars().take(1).flat_map(char::to_uppercase).collect())
}

pub(super) fn spaces(conn: &Connection) -> AppResult<HashMap<String, SpaceInfo>> {
    let mut query = conn.prepare(
        "SELECT s.project_id, COALESCE(s.name, p.name), s.icon, s.color, s.client
         FROM space s JOIN cache.space p ON p.id = s.project_id
         WHERE s.tracked = 1 ORDER BY s.position, p.key",
    )?;
    let rows = query.query_map([], |row| {
        let name: String = row.get(1)?;
        let icon: Option<String> = row.get(2)?;
        Ok((
            row.get::<_, String>(0)?,
            SpaceInfo {
                position: 0,
                letter: space_letter(icon, &name),
                name,
                color: row.get(3)?,
                client: row.get(4)?,
            },
        ))
    })?;
    let mut spaces = HashMap::new();
    for (position, row) in rows.enumerate() {
        let (project_id, space) = row?;
        spaces.insert(project_id, SpaceInfo { position, ..space });
    }
    Ok(spaces)
}

pub(super) fn versions(conn: &Connection) -> AppResult<HashMap<String, Version>> {
    let mut query = conn.prepare("SELECT id, name, release_date FROM cache.version")?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get(0)?,
            Version {
                name: row.get(1)?,
                release_date: row.get(2)?,
            },
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}
