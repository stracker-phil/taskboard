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
    /// The cache's size, on disk or in memory.
    #[serde(default)]
    #[ts(type = "number")]
    pub cache_bytes: u64,
}

pub async fn overview(db: &Db, state: SyncState) -> AppResult<SyncOverview> {
    let (last_full_at, projects) = db
        .read(|conn| Ok((last_full_at(conn)?, project_counts(conn)?)))
        .await?;
    Ok(SyncOverview {
        state,
        last_full_at,
        projects,
        cache_bytes: db.cache_bytes().await?,
    })
}

fn project_counts(conn: &Connection) -> AppResult<Vec<ProjectCounts>> {
    let mut query = conn.prepare(
        "SELECT p.id, p.key, COALESCE(s.name, p.name),
           (SELECT COUNT(*) FROM cache.issue i WHERE i.project_id = p.id AND i.gone = 0),
           (SELECT COUNT(*) FROM cache.issue i
              WHERE i.project_id = p.id AND i.gone = 0 AND i.status_category != 'done'),
           (SELECT last_full_at FROM cache.sync_state WHERE scope_id = ?1 || p.id)
         FROM space s JOIN cache.space p ON p.id = s.project_id
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;

    #[tokio::test]
    async fn the_overview_reports_the_size_of_a_fresh_cache() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();

        let overview = overview(&db, SyncState::Idle { last_ok: None })
            .await
            .unwrap();

        assert!(overview.cache_bytes > 0);
        assert_eq!(overview.cache_bytes, db.cache_bytes().await.unwrap());
    }

    #[test]
    fn an_overview_without_cache_bytes_reads_as_zero() {
        let json = serde_json::json!({
            "state": { "state": "idle", "lastOk": null },
            "lastFullAt": null,
            "projects": [],
        });

        let overview: SyncOverview = serde_json::from_value(json).unwrap();

        assert_eq!(overview.cache_bytes, 0);
    }

    #[test]
    fn the_overview_serializes_the_cache_size_as_cache_bytes() {
        let overview = SyncOverview {
            state: SyncState::Idle { last_ok: None },
            last_full_at: None,
            projects: vec![],
            cache_bytes: 8192,
        };

        assert_eq!(
            serde_json::to_value(&overview).unwrap()["cacheBytes"],
            serde_json::json!(8192)
        );
    }
}
