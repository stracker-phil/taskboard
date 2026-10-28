//! Numbered SQL migrations per file, embedded in the binary. `PRAGMA <schema>.user_version` counts the ones
//! applied; a shipped migration is never edited, only followed by a new one.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub const LOCAL: &[&str] = &[
    include_str!("../../migrations/local/0001_init.sql"),
    include_str!("../../migrations/local/0002_tree.sql"),
    include_str!("../../migrations/local/0003_view_widths.sql"),
    include_str!("../../migrations/local/0004_view_wrap.sql"),
    include_str!("../../migrations/local/0005_view_collapsed.sql"),
];
pub const CACHE: &[&str] = &[include_str!("../../migrations/cache/0001_init.sql")];

/// Brings the connection's file up to date in one transaction. A file migrated by a newer Taskboard is
/// refused, never touched.
pub fn migrate(conn: &mut Connection, name: &str, migrations: &[&str]) -> AppResult<()> {
    let version: usize = conn.query_row("PRAGMA user_version", [], |row| {
        row.get::<_, i64>(0).map(|v| v.max(0) as usize)
    })?;
    if version > migrations.len() {
        return Err(AppError::Db(format!(
            "The {name} database is from a newer Taskboard (version {version}, this one knows {})",
            migrations.len()
        )));
    }
    if version == migrations.len() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for sql in &migrations[version..] {
        tx.execute_batch(sql)?;
    }
    tx.execute_batch(&format!("PRAGMA user_version = {}", migrations.len()))?;
    tx.commit()?;
    Ok(())
}
