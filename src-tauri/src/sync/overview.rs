//! What the cache holds per tracked project: Settings → Sync and `taskboard status` show it.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::SyncState;
use super::store::{PROJECT_ROW_PREFIX, last_full_at};
use crate::db::Db;
use crate::error::AppResult;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ProjectCounts {
    pub id: String,
    pub key: String,
    pub name: String,
    /// Issues in the cache, open or done.
    pub cached: u32,
    pub open: u32,
    pub last_full_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SyncOverview {
    pub state: SyncState,
    pub last_full_at: Option<String>,
    pub projects: Vec<ProjectCounts>,
}

pub async fn overview(db: &Db, state: SyncState) -> AppResult<SyncOverview> {
    let (last_full_at, projects) = db
        .read(|conn| Ok((last_full_at(conn)?, project_counts(conn)?)))
        .await?;
    Ok(SyncOverview {
        state,
        last_full_at,
        projects,
    })
}

fn project_counts(conn: &Connection) -> AppResult<Vec<ProjectCounts>> {
    let mut query = conn.prepare(
        "SELECT p.id, p.key, COALESCE(s.name, p.name),
           (SELECT COUNT(*) FROM cache.issue i WHERE i.project_id = p.id AND i.gone = 0),
           (SELECT COUNT(*) FROM cache.issue i
              WHERE i.project_id = p.id AND i.gone = 0 AND i.status_category != 'done'),
           (SELECT last_full_at FROM cache.sync_state WHERE scope_id = ?1 || p.id)
         FROM space s JOIN cache.project p ON p.id = s.project_id
         WHERE s.tracked = 1 ORDER BY s.position",
    )?;
    let rows = query.query_map([PROJECT_ROW_PREFIX], |row| {
        Ok(ProjectCounts {
            id: row.get(0)?,
            key: row.get(1)?,
            name: row.get(2)?,
            cached: row.get(3)?,
            open: row.get(4)?,
            last_full_at: row.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}
