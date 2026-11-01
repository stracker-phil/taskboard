//! The sync engine and the cache on a big site: 5000 demo issues with 20000 comments served by the loopback
//! fake Jira, synced into a temp folder, changed, deleted and compacted.
//!
//! Slow, so ignored: `cargo test --test sync_stress -- --include-ignored`.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::Viewer;
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::store::forget_project;
use taskboard_lib::sync::{Emit, Job, SyncContext, SyncState, full_sync, run_job};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL, synthetic};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::{Narrowing, view_rows, views_of};

const ISSUES: usize = 5000;
const COMMENTS_PER_ISSUE: usize = 4;
const EDITED: usize = 50;
const DELETED: usize = 100;
const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const KEPT_PROJECT: &str = "10001";
/// Every issue is stamped with this, so the scope's 90 days keep all of them whatever their status.
const UPDATED: &str = "2026-10-01T09:00:00.000+0000";
/// Generous for debug builds on a slow machine; a regression to a per-row query would be seconds.
const LIST_BUDGET: Duration = Duration::from_millis(500);
const LIST_RUNS: usize = 5;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()
}

fn later() -> DateTime<Utc> {
    DEMO_NOW.parse::<DateTime<Utc>>().unwrap() + chrono::Duration::hours(2)
}

fn viewer() -> Viewer {
    Viewer {
        me: None,
        time_zone: Some("Europe/Berlin".into()),
        now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
    }
}

struct Site {
    fake: FakeJira,
    ctx: SyncContext,
    emit: Emit,
    keys: Vec<String>,
    _dir: tempfile::TempDir,
}

impl Site {
    /// Connected to a fake holding the synthetic issues, all five projects tracked, nothing synced yet.
    async fn connected() -> Self {
        let mut data = synthetic(ISSUES);
        for issue in &mut data.issues {
            issue["fields"]["updated"] = json!(UPDATED);
        }
        let keys: Vec<String> = data
            .issues
            .iter()
            .map(|issue| issue["key"].as_str().unwrap().to_string())
            .collect();
        let fake = FakeJira::start(data, DEMO_NOW.parse().unwrap()).await;
        fake.seed_comments(COMMENTS_PER_ISSUE, &demo::demo().users[0]);

        let dir = temp_config_dir();
        let ctx = SyncContext {
            settings: SettingsStore::open(dir.path()),
            access: JiraAccess::new(
                Arc::new(FakeSecrets::default()),
                Arc::new(Limiter::new(1000, 1000)),
            ),
            db: Db::open(dir.path()).unwrap(),
        };
        let input = ConnectionInput {
            site_url: fake.url().to_string(),
            email: ME_EMAIL.into(),
            token_kind: TokenKind::Classic,
            token_expires: None,
            cloud_id: None,
        };
        save(
            &ctx.settings,
            &ctx.access,
            input,
            Some(DEMO_TOKEN.into()),
            today(),
        )
        .await
        .unwrap();
        refresh_projects(&ctx.db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();
        for id in PROJECT_IDS {
            track_project(&ctx.db, id.into(), true).await.unwrap();
        }
        Self {
            fake,
            ctx,
            emit: Arc::new(|_| {}),
            keys,
            _dir: dir,
        }
    }

    async fn run(&self, job: Job) {
        let state = run_job(&self.ctx, job, &*self.emit, None).await;
        assert!(
            matches!(state, SyncState::Idle { last_ok: Some(_) }),
            "{job:?}: {state:?}"
        );
    }

    async fn count(&self, sql: &'static str) -> i64 {
        self.ctx
            .db
            .read(move |conn| Ok(conn.query_row(sql, [], |row| row.get::<_, i64>(0))?))
            .await
            .unwrap()
    }

    async fn texts(&self, sql: &'static str) -> BTreeSet<String> {
        self.ctx
            .db
            .read(move |conn| {
                let mut query = conn.prepare(sql)?;
                let found = query
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<Result<BTreeSet<_>, _>>()?;
                Ok(found)
            })
            .await
            .unwrap()
    }

    /// The keys of the rows the Everything list shows, and how long the slowest of several fetches took.
    async fn everything_list(&self) -> (BTreeSet<String>, Duration) {
        let list = self
            .count("SELECT id FROM filter_list WHERE kind = 'everything'")
            .await;
        let view = views_of(&self.ctx.db, list).await.unwrap()[0].id;
        let mut slowest = Duration::ZERO;
        let mut keys = BTreeSet::new();
        for _ in 0..LIST_RUNS {
            let started = Instant::now();
            let data = view_rows(&self.ctx.db, viewer(), list, view, Narrowing::default())
                .await
                .unwrap();
            slowest = slowest.max(started.elapsed());
            let comments = COMMENTS_PER_ISSUE as i64;
            assert!(data.rows.values().all(|row| row.comment_count == comments));
            keys = data.rows.values().map(|row| row.key.clone()).collect();
        }
        (keys, slowest)
    }
}

#[tokio::test]
#[ignore = "syncs 5000 issues with 20000 comments through the loopback fake"]
async fn a_site_of_5000_issues_syncs_changes_loses_issues_and_compacts() {
    let site = Site::connected().await;
    let all: BTreeSet<String> = site.keys.iter().cloned().collect();
    let edited: Vec<String> = site.keys[..EDITED].to_vec();
    let deleted: Vec<String> = site.keys[EDITED..EDITED + DELETED].to_vec();
    assert_eq!(all.len(), ISSUES);

    let emit = site.emit.clone();
    full_sync(&site.ctx, &*emit).await.unwrap();

    assert_eq!(
        site.count("SELECT COUNT(*) FROM cache.issue").await,
        ISSUES as i64
    );
    assert_eq!(
        site.count("SELECT COUNT(*) FROM cache.comment").await,
        (ISSUES * COMMENTS_PER_ISSUE) as i64
    );
    let (listed, took) = site.everything_list().await;
    assert_eq!(listed, all);
    assert!(
        took < LIST_BUDGET,
        "the list of {ISSUES} issues took {took:?}"
    );

    site.fake.set_now(later());
    for key in &edited {
        site.fake.edit_issue(key, |fields| {
            fields["summary"] = json!(format!("Edited {key}"))
        });
    }
    site.run(Job::Delta).await;

    assert_eq!(
        site.count("SELECT COUNT(*) FROM cache.issue").await,
        ISSUES as i64
    );
    assert_eq!(
        site.texts("SELECT key FROM cache.issue WHERE summary LIKE 'Edited %'")
            .await,
        edited.iter().cloned().collect()
    );
    assert_eq!(
        site.texts("SELECT summary FROM cache.issue WHERE summary LIKE 'Edited %'")
            .await,
        edited.iter().map(|key| format!("Edited {key}")).collect()
    );

    for key in &deleted {
        site.fake.delete_issue(key);
    }
    site.run(Job::Reconcile).await;

    assert_eq!(
        site.texts("SELECT key FROM cache.issue WHERE gone = 1")
            .await,
        deleted.iter().cloned().collect()
    );
    let (listed, took) = site.everything_list().await;
    let gone: BTreeSet<String> = deleted.iter().cloned().collect();
    let expected: BTreeSet<String> = all.difference(&gone).cloned().collect();
    assert_eq!(listed, expected);
    assert!(took < LIST_BUDGET, "the list after deletions took {took:?}");

    let kept = site
        .count("SELECT COUNT(*) FROM cache.issue WHERE project_id = '10001'")
        .await;
    let dropped: Vec<String> = PROJECT_IDS
        .iter()
        .filter(|id| **id != KEPT_PROJECT)
        .map(|id| id.to_string())
        .collect();
    site.ctx
        .db
        .write(move |tx, _| {
            for id in &dropped {
                forget_project(tx, id)?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let before = site.ctx.db.cache_size().await.unwrap();

    let after = site.ctx.db.compact_cache().await.unwrap();

    assert!(after.bytes < before.bytes, "{before:?} -> {after:?}");
    assert_eq!(site.count("SELECT COUNT(*) FROM cache.issue").await, kept);
    assert_eq!(
        site.count("SELECT COUNT(*) FROM cache.comment").await,
        kept * COMMENTS_PER_ISSUE as i64
    );
}
