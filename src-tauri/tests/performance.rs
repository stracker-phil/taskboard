//! Query speed on a synthetic site of 5000 issues in a temp database.
//!
//! Meant for the release profile: `cargo test --release --test performance -- --ignored`. Debug builds may be
//! slower, so the limits are 10 times looser there. The loopback fake Jira is only reachable in debug builds,
//! so the cache is filled from the synthetic site's data with the sync's own store functions.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use tempfile::TempDir;

use taskboard_lib::connection::track_project;
use taskboard_lib::db::Db;
use taskboard_lib::jira::dto::{Field, Issue, Project, Status, Version};
use taskboard_lib::lists::{Viewer, sidebar_tree};
use taskboard_lib::sync::map::format_utc;
use taskboard_lib::sync::store::{store_fields, store_page, store_statuses, store_versions};
use taskboard_lib::test_support::demo::{DEMO_NOW, synthetic};
use taskboard_lib::test_support::temp_config_dir;
use taskboard_lib::views::{GroupBy, Narrowing, view_rows, view_set_group, views_of};

const ME_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const ISSUES: usize = 5000;
const RUNS: usize = 20;

fn viewer() -> Viewer {
    Viewer {
        me: Some(ME_ACCOUNT_ID.into()),
        time_zone: Some("Europe/Vienna".into()),
        now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
    }
}

/// The limit in milliseconds, 10 times looser in debug builds.
fn limit(release_ms: u64) -> Duration {
    let factor = if cfg!(debug_assertions) { 10 } else { 1 };
    Duration::from_millis(release_ms * factor)
}

fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

fn parsed<T: DeserializeOwned>(values: Vec<serde_json::Value>) -> Vec<T> {
    values
        .into_iter()
        .map(|value| serde_json::from_value(value).unwrap())
        .collect()
}

/// A database holding the synthetic site with all its projects tracked.
async fn synthetic_site() -> (TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    let data = synthetic(ISSUES);
    let fields: Vec<Field> = parsed(data.fields);
    let statuses: Vec<Status> = parsed(data.statuses);
    let projects: Vec<Project> = parsed(data.projects);
    let issues: Vec<Issue> = parsed(data.issues);
    let versions: BTreeMap<String, Vec<Version>> = data
        .versions
        .into_iter()
        .map(|(key, list)| (key, parsed(list)))
        .collect();
    let project_ids: Vec<String> = projects.iter().map(|p| p.id.clone()).collect();
    let now = format_utc(Utc::now());
    db.write(move |tx, changes| {
        for project in &projects {
            tx.execute(
                "INSERT INTO cache.project (id, key, name, raw) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    project.id,
                    project.key,
                    project.name,
                    serde_json::to_string(project)?
                ],
            )?;
        }
        let ids = store_fields(tx, &fields)?;
        store_statuses(tx, &statuses)?;
        for project in &projects {
            if let Some(list) = versions.get(&project.key) {
                store_versions(tx, &project.id, list)?;
            }
        }
        store_page(tx, changes, &issues, &ids, true, &now)?;
        Ok(())
    })
    .await
    .unwrap();
    for id in project_ids {
        track_project(&db, id, true).await.unwrap();
    }
    (dir, db)
}

async fn everything(db: &Db) -> (i64, i64) {
    let list = db
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT id FROM filter_list WHERE kind = 'everything'",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    let view = views_of(db, list).await.unwrap()[0].id;
    (list, view)
}

async fn time_view(db: &Db, list: i64, view: i64, narrowing: Narrowing) -> Duration {
    let mut times = Vec::new();
    for _ in 0..RUNS {
        let started = Instant::now();
        let data = view_rows(db, viewer(), list, view, narrowing.clone())
            .await
            .unwrap();
        times.push(started.elapsed());
        assert!(!data.rows.is_empty());
    }
    median(times)
}

#[tokio::test]
#[ignore = "fills 5000 issues; meant for the release profile"]
async fn everything_stays_fast_to_group_and_narrow_on_5000_issues() {
    let (_dir, db) = synthetic_site().await;
    let (list, view) = everything(&db).await;

    view_set_group(&db, view, GroupBy::Status).await.unwrap();
    let by_status = time_view(&db, list, view, Narrowing::default()).await;
    view_set_group(&db, view, GroupBy::Project).await.unwrap();
    let by_project = time_view(&db, list, view, Narrowing::default()).await;
    let narrowed = time_view(
        &db,
        list,
        view,
        Narrowing {
            text: "a".into(),
            ..Narrowing::default()
        },
    )
    .await;

    eprintln!("view_rows by status: {by_status:?}");
    eprintln!("view_rows by project: {by_project:?}");
    eprintln!("view_rows with text \"a\": {narrowed:?}");
    let max = limit(50);
    assert!(by_status < max, "by status {by_status:?} >= {max:?}");
    assert!(by_project < max, "by project {by_project:?} >= {max:?}");
    assert!(narrowed < max, "narrowed {narrowed:?} >= {max:?}");
}

#[tokio::test]
#[ignore = "fills 5000 issues; meant for the release profile"]
async fn the_sidebar_tree_stays_fast_on_5000_issues() {
    let (_dir, db) = synthetic_site().await;

    let mut times = Vec::new();
    for _ in 0..RUNS {
        let started = Instant::now();
        let tree = sidebar_tree(&db, viewer()).await.unwrap();
        times.push(started.elapsed());
        assert!(tree.everything.count.unwrap_or(0) > 0);
    }
    let sidebar = median(times);

    eprintln!("sidebar_tree: {sidebar:?}");
    let max = limit(20);
    assert!(sidebar < max, "sidebar tree {sidebar:?} >= {max:?}");
}
