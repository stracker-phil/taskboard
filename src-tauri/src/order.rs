//! Dense positions (0, 1, 2, …) among siblings of the local tables, kept dense after every change.

use rusqlite::{Connection, params_from_iter};

use crate::error::AppResult;

/// Rows ordered among each other: `table` and the condition that selects one set of siblings.
pub struct Siblings<'a> {
    pub table: &'static str,
    pub key: &'static str,
    /// SQL condition with `?` parameters selecting the siblings.
    pub of: &'a str,
    pub params: Vec<rusqlite::types::Value>,
}

impl Siblings<'_> {
    pub fn ids(&self, conn: &Connection) -> AppResult<Vec<rusqlite::types::Value>> {
        let sql = format!(
            "SELECT {key} FROM {table} WHERE {of} ORDER BY position, {key}",
            key = self.key,
            table = self.table,
            of = self.of
        );
        let mut query = conn.prepare(&sql)?;
        let ids = query.query_map(params_from_iter(&self.params), |row| row.get(0))?;
        Ok(ids.collect::<Result<_, _>>()?)
    }

    /// The next free position at the end.
    pub fn end(&self, conn: &Connection) -> AppResult<i64> {
        Ok(self.ids(conn)?.len() as i64)
    }

    /// Writes 0, 1, 2, … in the current order.
    pub fn renumber(&self, conn: &Connection) -> AppResult<()> {
        let ids = self.ids(conn)?;
        self.write(conn, &ids)
    }

    /// Puts `id` at `position` (clamped) and renumbers the rest around it.
    pub fn move_to(
        &self,
        conn: &Connection,
        id: rusqlite::types::Value,
        position: i64,
    ) -> AppResult<()> {
        let mut ids = self.ids(conn)?;
        ids.retain(|other| *other != id);
        let at = usize::try_from(position.max(0)).unwrap_or(0).min(ids.len());
        ids.insert(at, id);
        self.write(conn, &ids)
    }

    fn write(&self, conn: &Connection, ids: &[rusqlite::types::Value]) -> AppResult<()> {
        let sql = format!(
            "UPDATE {} SET position = ?1 WHERE {} = ?2",
            self.table, self.key
        );
        let mut update = conn.prepare(&sql)?;
        for (position, id) in ids.iter().enumerate() {
            update.execute(rusqlite::params![position as i64, id])?;
        }
        Ok(())
    }
}
