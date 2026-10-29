//! Which Jira projects Taskboard tracks: Jira's list in the cache, the owner's choice in `local.space`.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::db::{Db, LocalPart};
use crate::error::AppResult;
use crate::jira::JiraClient;
use crate::jira::dto::Project;
use crate::lists::ensure_space;
use crate::sync::store::store_versions;

/// How many Space tile colours the palette has (`--space-tile-*`).
const SPACE_COLORS: i64 = 12;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ProjectRow {
    pub id: String,
    pub key: String,
    pub name: String,
    pub tracked: bool,
}

/// Fetches every project the user can browse into the cache; projects Jira no longer lists are dropped
/// there (their Space settings stay).
pub async fn refresh_projects(db: &Db, client: &JiraClient) -> AppResult<Vec<ProjectRow>> {
    store_spaces(db, client.projects().await?).await
}

/// Replaces the cached spaces with the source's list and returns them as the Projects page lists them.
pub async fn store_spaces(db: &Db, projects: Vec<Project>) -> AppResult<Vec<ProjectRow>> {
    db.write(move |tx, _| {
        drop_unlisted(tx, &projects)?;
        for project in &projects {
            upsert_project(tx, project)?;
        }
        Ok(())
    })
    .await?;
    list_projects(db).await
}

fn drop_unlisted(conn: &Connection, listed: &[Project]) -> AppResult<()> {
    let mut cached = conn.prepare("SELECT id FROM cache.space")?;
    let ids: Vec<String> = cached
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for id in ids.iter().filter(|id| !listed.iter().any(|p| &p.id == *id)) {
        conn.execute("DELETE FROM cache.space WHERE id = ?1", [id])?;
    }
    Ok(())
}

fn upsert_project(conn: &Connection, project: &Project) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.space (id, key, name, raw) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (id) DO UPDATE SET key = ?2, name = ?3, raw = ?4",
        params![
            project.id,
            project.key,
            project.name,
            serde_json::to_string(project)?
        ],
    )?;
    Ok(())
}

/// The cached projects by name, with the owner's choice.
pub async fn list_projects(db: &Db) -> AppResult<Vec<ProjectRow>> {
    db.read(|conn| {
        let mut query = conn.prepare(
            "SELECT p.id, p.key, p.name, COALESCE(s.tracked, 0)
             FROM cache.space p LEFT JOIN space s ON s.project_id = p.id
             ORDER BY p.name COLLATE NOCASE",
        )?;
        let rows = query.query_map([], |row| {
            Ok(ProjectRow {
                id: row.get(0)?,
                key: row.get(1)?,
                name: row.get(2)?,
                tracked: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
    .await
}

/// The tracked projects in Space order.
pub async fn tracked_projects(db: &Db) -> AppResult<Vec<ProjectRow>> {
    db.read(|conn| {
        let mut query = conn.prepare(
            "SELECT p.id, p.key, p.name FROM space s JOIN cache.space p ON p.id = s.project_id
             WHERE s.tracked = 1 ORDER BY s.position",
        )?;
        let rows = query.query_map([], |row| {
            Ok(ProjectRow {
                id: row.get(0)?,
                key: row.get(1)?,
                name: row.get(2)?,
                tracked: true,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
    .await
}

/// Tracking a project for the first time makes its Space: next position, the name's first letter as
/// icon, a tile colour by position, and its default views and filter lists. Untracking keeps all of it, so
/// the settings survive re-tracking.
pub async fn track_project(db: &Db, project_id: String, tracked: bool) -> AppResult<()> {
    db.write(move |tx, changes| {
        let updated = tx.execute(
            "UPDATE space SET tracked = ?2 WHERE project_id = ?1",
            params![project_id, tracked],
        )?;
        if updated == 0 {
            insert_space(tx, &project_id, tracked)?;
        }
        changes.local_changed(LocalPart::Spaces);
        if tracked && ensure_space(tx, &project_id)? {
            changes.local_changed(LocalPart::Lists);
            changes.local_changed(LocalPart::Views);
        }
        Ok(())
    })
    .await
}

/// Caches a project's versions, so tracking it can offer a Release filter list before the first sync.
pub async fn fetch_versions(db: &Db, client: &JiraClient, project_id: String) -> AppResult<()> {
    let key: Option<String> = {
        let id = project_id.clone();
        db.read(move |conn| {
            Ok(conn
                .query_row("SELECT key FROM cache.space WHERE id = ?1", [id], |row| {
                    row.get(0)
                })
                .optional()?)
        })
        .await?
    };
    let Some(key) = key else {
        return Ok(());
    };
    let versions = client.versions(&key).await?;
    db.write(move |tx, _| store_versions(tx, &project_id, &versions))
        .await
}

fn insert_space(conn: &Connection, project_id: &str, tracked: bool) -> AppResult<()> {
    let position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM space",
        [],
        |row| row.get(0),
    )?;
    let name: Option<String> = conn
        .query_row(
            "SELECT name FROM cache.space WHERE id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .optional()?;
    let icon = name
        .and_then(|name| name.chars().find(|c| c.is_alphanumeric()))
        .map(|c| c.to_uppercase().to_string());
    let color = (position % SPACE_COLORS + 1).to_string();
    conn.execute(
        "INSERT INTO space (project_id, tracked, icon, color, position) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![project_id, tracked, icon, color, position],
    )?;
    Ok(())
}

pub async fn tracked_keys(db: &Db) -> AppResult<Vec<String>> {
    db.read(|conn| {
        let mut query = conn.prepare(
            "SELECT p.key FROM space s JOIN cache.space p ON p.id = s.project_id
             WHERE s.tracked = 1 ORDER BY s.position",
        )?;
        let keys = query.query_map([], |row| row.get(0))?;
        Ok(keys.collect::<Result<_, _>>()?)
    })
    .await
}
