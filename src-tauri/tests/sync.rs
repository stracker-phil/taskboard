//! The sync engine against the loopback fake Jira on temp folders: the full sync's stored rows, progress,
//! change history and announcements, failures turned into states, the overview and the engine task.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use tempfile::TempDir;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, refresh_projects, save, track_project,
};
use taskboard_lib::db::{Changes, Db};
use taskboard_lib::error::ErrorKind;
use taskboard_lib::jira::jql::{date_literal, default_scope};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::{Viewer, sidebar_tree};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::map::utc;
use taskboard_lib::sync::store::last_delta_at;
use taskboard_lib::sync::{
    Emit, Job, SyncContext, SyncEngine, SyncEvent, SyncProgress, SyncState, full_sync, overview,
    run_job,
};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};

const SEARCH_PATH: &str = "/rest/api/3/search/jql";
const PAGE: u32 = 7;
const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
/// In Space order: what the demo site holds in scope per tracked project.
const COUNTS: [(&str, u32); 5] = [("NW", 33), ("NWS", 8), ("LUM", 11), ("FBA", 13), ("INT", 9)];
const TOTAL: i64 = 74;
const NW_1042: &str = "101042";
const NW_1044: &str = "101044";
const NW_1049: &str = "101049";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()
}

fn later() -> DateTime<Utc> {
    DEMO_NOW.parse::<DateTime<Utc>>().unwrap() + chrono::Duration::hours(2)
}

const LATER_UTC: &str = "2026-10-28T13:00:00.000Z";

fn tracked_keys() -> Vec<String> {
    COUNTS.iter().map(|(key, _)| key.to_string()).collect()
}

struct Rig {
    fake: FakeJira,
    _dir: TempDir,
    ctx: SyncContext,
    announced: Arc<Mutex<Vec<BTreeSet<String>>>>,
    events: Arc<Mutex<Vec<SyncEvent>>>,
}

impl Rig {
    /// Nothing saved yet: no connection, no projects.
    async fn new() -> Self {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        let announced = Arc::new(Mutex::new(Vec::new()));
        let sink = announced.clone();
        db.set_notifier(Box::new(move |changes: &Changes| {
            if !changes.issues.is_empty() {
                sink.lock().unwrap().push(changes.issues.clone());
            }
        }));
        let ctx = SyncContext {
            settings: SettingsStore::open(dir.path()),
            access: JiraAccess::new(
                Arc::new(FakeSecrets::default()),
                Arc::new(Limiter::new(1000, 1000)),
            ),
            db,
        };
        Self {
            fake: FakeJira::start_demo().await,
            _dir: dir,
            ctx,
            announced,
            events: Arc::default(),
        }
    }

    fn input(&self) -> ConnectionInput {
        ConnectionInput {
            site_url: self.fake.url().to_string(),
            email: ME_EMAIL.into(),
            token_kind: TokenKind::Classic,
            token_expires: None,
            cloud_id: None,
        }
    }

    /// Connected with a classic token, the projects cached and the five demo Spaces tracked.
    async fn connected() -> Self {
        let rig = Self::new().await;
        save(
            &rig.ctx.settings,
            &rig.ctx.access,
            rig.input(),
            Some(DEMO_TOKEN.into()),
            today(),
        )
        .await
        .unwrap();
        refresh_projects(&rig.ctx.db, &rig.fake.client(TokenKind::Classic))
            .await
            .unwrap();
        for id in PROJECT_IDS {
            track_project(&rig.ctx.db, id.into(), true).await.unwrap();
        }
        rig
    }

    fn emit(&self) -> Emit {
        let events = self.events.clone();
        Arc::new(move |event: SyncEvent| events.lock().unwrap().push(event))
    }

    async fn full_sync(&self) {
        let emit = self.emit();
        full_sync(&self.ctx, &*emit).await.unwrap();
    }

    async fn run(&self, last_ok: Option<String>) -> SyncState {
        self.job(Job::Full, last_ok).await
    }

    async fn job(&self, job: Job, last_ok: Option<String>) -> SyncState {
        let emit = self.emit();
        run_job(&self.ctx, job, &*emit, last_ok).await
    }

    async fn cursor(&self) -> Option<String> {
        self.ctx.db.read(last_delta_at).await.unwrap()
    }

    /// `(Everything's open count, the NW Space's open count)` as the sidebar shows them.
    async fn open_counts(&self) -> (Option<i64>, i64) {
        let viewer = Viewer {
            me: None,
            time_zone: Some("Europe/Vienna".into()),
            now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
        };
        let tree = sidebar_tree(&self.ctx.db, viewer).await.unwrap();
        let nw = tree.spaces.iter().find(|s| s.key == "NW").unwrap();
        (tree.everything.count, nw.open_count)
    }

    /// The search requests the fake received, oldest first.
    fn search_jqls(&self) -> Vec<String> {
        self.fake
            .requests()
            .into_iter()
            .filter(|request| request.path == SEARCH_PATH)
            .map(|request| request.body["jql"].as_str().unwrap().to_string())
            .collect()
    }

    fn progress(&self) -> Vec<SyncProgress> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                SyncEvent::Progress(progress) => Some(progress.clone()),
                SyncEvent::State(_) => None,
            })
            .collect()
    }

    fn states(&self) -> Vec<SyncState> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                SyncEvent::State(state) => Some(state.clone()),
                SyncEvent::Progress(_) => None,
            })
            .collect()
    }

    fn announced_ids(&self) -> BTreeSet<String> {
        self.announced
            .lock()
            .unwrap()
            .iter()
            .flatten()
            .cloned()
            .collect()
    }

    fn forget_announcements(&self) {
        self.announced.lock().unwrap().clear();
    }

    fn requests(&self) -> usize {
        self.fake.requests().len()
    }

    fn searches(&self) -> usize {
        self.fake
            .paths()
            .iter()
            .filter(|path| *path == SEARCH_PATH)
            .count()
    }
}

async fn rows<T: Send + 'static>(
    db: &Db,
    sql: &'static str,
    map: fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Vec<T> {
    db.read(move |conn| {
        let mut query = conn.prepare(sql)?;
        let found = query.query_map([], map)?.collect::<Result<Vec<_>, _>>()?;
        Ok(found)
    })
    .await
    .unwrap()
}

async fn count(db: &Db, sql: &'static str) -> i64 {
    rows(db, sql, |row| row.get::<_, i64>(0)).await[0]
}

async fn issues_per_project(db: &Db) -> BTreeMap<String, i64> {
    rows(
        db,
        "SELECT p.key, COUNT(*) FROM cache.issue i JOIN cache.project p ON p.id = i.project_id
         GROUP BY p.key",
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .await
    .into_iter()
    .collect()
}

fn expected_counts() -> BTreeMap<String, i64> {
    COUNTS
        .iter()
        .map(|(key, count)| (key.to_string(), *count as i64))
        .collect()
}

/// `(scope_id, has last_full_at, last_delta_at, approx_total)`
type StateRow = (String, bool, Option<String>, Option<i64>);

async fn state_rows(db: &Db) -> Vec<StateRow> {
    rows(
        db,
        "SELECT scope_id, last_full_at IS NOT NULL, last_delta_at, approx_total
         FROM cache.sync_state ORDER BY scope_id",
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .await
}

async fn scope_full_at(db: &Db) -> Option<String> {
    rows(
        db,
        "SELECT last_full_at FROM cache.sync_state WHERE scope_id = 'scope'",
        |row| row.get::<_, Option<String>>(0),
    )
    .await
    .into_iter()
    .next()
    .flatten()
}

/// `(issue key, field, old value, new value, changed_at)`
type ChangeRow = (String, String, Option<String>, Option<String>, String);

async fn changes(db: &Db) -> Vec<ChangeRow> {
    rows(
        db,
        "SELECT i.key, c.field, c.old_value, c.new_value, c.changed_at
         FROM cache.issue_change c JOIN cache.issue i ON i.id = c.issue_id
         ORDER BY i.key, c.field",
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )
    .await
}

async fn issue_id(db: &Db, key: &str) -> String {
    let key = key.to_string();
    db.read(move |conn| {
        Ok(
            conn.query_row("SELECT id FROM cache.issue WHERE key = ?1", [key], |row| {
                row.get::<_, String>(0)
            })?,
        )
    })
    .await
    .unwrap()
}

async fn text_of(db: &Db, sql: &'static str) -> String {
    rows(db, sql, |row| row.get::<_, String>(0)).await.remove(0)
}

fn newest_updated_in_scope() -> String {
    demo::demo()
        .issues
        .iter()
        .filter(|issue| issue["fields"]["project"]["key"] != "ARC")
        .filter_map(|issue| utc(&issue["fields"]["updated"]))
        .max()
        .unwrap()
}

fn expected_progress(page_size: u32) -> Vec<SyncProgress> {
    let mut all = Vec::new();
    for (index, (key, count)) in COUNTS.iter().enumerate() {
        for pages_done in 0..=count.div_ceil(page_size) {
            all.push(SyncProgress {
                job: Job::Full,
                project_id: PROJECT_IDS[index].into(),
                project_key: key.to_string(),
                pages_done,
                fetched: (pages_done * page_size).min(*count),
                approx_total: Some(*count as u64),
            });
        }
    }
    all
}

/// Runs `act` as soon as the fake has been asked for one more search page than it had at the call.
async fn after_the_next_search_request(rig: &Rig, act: impl FnOnce()) {
    let baseline = rig.searches();
    tokio::time::timeout(Duration::from_secs(30), async {
        while rig.searches() == baseline {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("the sync never searched");
    act();
}

async fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(30), async {
        while !done() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

fn done_status() -> Value {
    json!({
        "id": "10001",
        "name": "Done",
        "statusCategory": { "id": 3, "key": "done", "name": "Done" },
    })
}

mod the_full_sync {
    use super::*;

    #[tokio::test]
    async fn stores_every_issue_in_scope_across_pages_and_nothing_outside_it() {
        let rig = Rig::connected().await;
        rig.fake.set_page_size(PAGE as usize);

        rig.full_sync().await;

        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
        assert_eq!(
            count(&rig.ctx.db, "SELECT COUNT(*) FROM cache.issue").await,
            TOTAL
        );
        assert_eq!(
            count(&rig.ctx.db, "SELECT COUNT(*) FROM cache.issue_fts").await,
            TOTAL
        );
        let outside = count(
            &rig.ctx.db,
            "SELECT COUNT(*) FROM cache.issue WHERE key IN ('NW-912', 'NW-934')
               OR key LIKE 'ARC-%'",
        )
        .await;
        assert_eq!(outside, 0);
    }

    #[tokio::test]
    async fn reports_progress_per_project_page_by_page() {
        let rig = Rig::connected().await;
        rig.fake.set_page_size(PAGE as usize);

        rig.full_sync().await;

        assert_eq!(rig.progress(), expected_progress(PAGE));
    }

    #[tokio::test]
    async fn records_the_sync_state_of_each_project_and_of_the_scope() {
        let rig = Rig::connected().await;
        rig.fake.set_page_size(PAGE as usize);

        rig.full_sync().await;

        let mut expected: Vec<StateRow> = PROJECT_IDS
            .iter()
            .zip(COUNTS)
            .map(|(id, (_, count))| (format!("full:{id}"), true, None, Some(count as i64)))
            .collect();
        expected.push(("scope".into(), true, Some(newest_updated_in_scope()), None));
        assert_eq!(state_rows(&rig.ctx.db).await, expected);
        assert_eq!(
            text_of(
                &rig.ctx.db,
                "SELECT jql FROM cache.sync_state WHERE scope_id = 'scope'"
            )
            .await,
            default_scope(&tracked_keys(), 90).unwrap()
        );
    }

    #[tokio::test]
    async fn marks_every_issue_seen_and_records_no_changes_on_the_first_sync() {
        let rig = Rig::connected().await;

        rig.full_sync().await;

        assert_eq!(
            count(&rig.ctx.db, "SELECT COUNT(*) FROM cache.issue_seen").await,
            TOTAL
        );
        assert_eq!(changes(&rig.ctx.db).await, vec![]);
    }

    #[tokio::test]
    async fn announces_the_issues_of_the_first_sync() {
        let rig = Rig::connected().await;

        rig.full_sync().await;

        assert_eq!(rig.announced_ids().len() as i64, TOTAL);
        assert!(rig.announced_ids().contains(NW_1042));
    }

    #[tokio::test]
    async fn stores_comments_epics_subtasks_and_the_sites_metadata() {
        let rig = Rig::connected().await;

        rig.full_sync().await;

        let db = &rig.ctx.db;
        assert_eq!(
            count(
                db,
                "SELECT COUNT(*) FROM cache.comment WHERE issue_id = '101042'"
            )
            .await,
            4
        );
        assert_eq!(
            count(
                db,
                "SELECT COUNT(*) FROM cache.issue WHERE parent_id = '101020' AND type_is_subtask = 0"
            )
            .await,
            9
        );
        assert_eq!(
            rows(
                db,
                "SELECT key FROM cache.issue WHERE parent_id = '101042' ORDER BY key",
                |row| row.get::<_, String>(0)
            )
            .await,
            ["NW-1043", "NW-1045", "NW-1059"]
        );
        assert_eq!(
            text_of(
                db,
                "SELECT epic_color FROM cache.issue WHERE key = 'NW-1020'"
            )
            .await,
            "ghx-label-7"
        );
        assert_eq!(
            text_of(db, "SELECT sprint FROM cache.issue WHERE key = 'NW-1033'").await,
            "Sprint 22"
        );
        assert_eq!(count(db, "SELECT COUNT(*) FROM cache.status").await, 5);
        let versions: Vec<(String, String)> = rows(
            db,
            "SELECT p.key, v.name FROM cache.version v JOIN cache.project p ON p.id = v.project_id
             ORDER BY p.key, v.name",
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .await;
        let expected: Vec<(String, String)> = [
            ("LUM", "3.2.0"),
            ("LUM", "3.3.0"),
            ("LUM", "4.0"),
            ("NW", "2.3.1"),
            ("NW", "2.4.0"),
            ("NW", "2.5.0"),
        ]
        .iter()
        .map(|(key, name)| (key.to_string(), name.to_string()))
        .collect();
        assert_eq!(versions, expected);
    }

    #[tokio::test]
    async fn stores_who_the_user_is_in_the_connection_settings() {
        let rig = Rig::connected().await;

        rig.full_sync().await;

        let connection = rig.ctx.settings.get().connection;
        assert_eq!(connection.display_name.as_deref(), Some("Alex Morgan"));
        assert_eq!(connection.time_zone.as_deref(), Some("Europe/Vienna"));
        assert_eq!(
            connection.account_id.as_deref(),
            Some("712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01")
        );
    }

    #[tokio::test]
    async fn a_second_sync_without_changes_announces_and_records_nothing() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.forget_announcements();

        rig.full_sync().await;

        assert_eq!(rig.announced_ids(), BTreeSet::new());
        assert_eq!(changes(&rig.ctx.db).await, vec![]);
        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
    }

    #[tokio::test]
    async fn records_and_announces_what_changed_on_the_site_since_the_last_sync() {
        let rig = Rig::connected().await;
        rig.fake.set_page_size(PAGE as usize);
        rig.full_sync().await;
        rig.forget_announcements();
        rig.fake.set_now(later());
        rig.fake
            .edit_issue("NW-1044", |fields| fields["status"] = done_status());
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["priority"] = json!({ "id": "4", "name": "Low" });
        });
        rig.fake.add_comment(
            "NW-1042",
            demo::demo().users[0].clone(),
            "Postcode rules for Liechtenstein are merged.",
        );
        let new_key =
            rig.fake
                .add_issue("NW-1055", "NW", "Order confirmation email: shipping notice");

        rig.full_sync().await;

        let db = &rig.ctx.db;
        let row = |key: &str, field: &str, old: Option<&str>, new: Option<&str>| -> ChangeRow {
            (
                key.into(),
                field.into(),
                old.map(String::from),
                new.map(String::from),
                LATER_UTC.into(),
            )
        };
        assert_eq!(
            changes(db).await,
            vec![
                row("NW-1042", "comment_added", Some("4"), Some("5")),
                row("NW-1044", "status", Some("10002"), Some("10001")),
                row("NW-1049", "priority", Some("Highest"), Some("Low")),
                row(&new_key, "created", None, Some(&new_key)),
            ]
        );
        let new_id = issue_id(db, &new_key).await;
        assert_eq!(
            rig.announced_ids(),
            BTreeSet::from([
                NW_1042.into(),
                NW_1044.into(),
                NW_1049.into(),
                new_id.clone()
            ])
        );
        assert_eq!(
            count(
                db,
                "SELECT COUNT(*) FROM cache.comment WHERE issue_id = '101042'"
            )
            .await,
            5
        );
        assert_eq!(
            text_of(
                db,
                "SELECT status_category FROM cache.issue WHERE key = 'NW-1044'"
            )
            .await,
            "done"
        );
        assert_eq!(
            count(db, "SELECT COUNT(*) FROM cache.issue_seen").await,
            TOTAL
        );
        let seen = count(
            db,
            "SELECT COUNT(*) FROM cache.issue_seen s JOIN cache.issue i ON i.id = s.issue_id
             WHERE i.key LIKE 'NW-106%'",
        )
        .await;
        assert_eq!(seen, 0);
        let scope = state_rows(db).await.pop().unwrap();
        assert_eq!(scope.2.as_deref(), Some(LATER_UTC));
    }

    #[tokio::test]
    async fn waits_out_a_rate_limit_answer_mid_run_and_completes() {
        let rig = Rig::connected().await;
        rig.fake.set_page_size(PAGE as usize);
        rig.fake.set_latency(Duration::from_millis(30));
        let emit = rig.emit();

        let (result, ()) = tokio::join!(
            full_sync(&rig.ctx, &*emit),
            after_the_next_search_request(&rig, || rig.fake.fail_next(1, 429, Some("0")))
        );

        assert_eq!(result, Ok(()));
        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
        let pages: u32 = COUNTS.iter().map(|(_, c)| c.div_ceil(PAGE)).sum();
        assert_eq!(rig.searches() as u32, pages + 1);
    }

    #[tokio::test]
    async fn retries_a_rate_limited_first_request() {
        let rig = Rig::connected().await;
        rig.fake.fail_next(1, 429, Some("0"));

        rig.full_sync().await;

        let field_requests = rig
            .fake
            .paths()
            .iter()
            .filter(|path| *path == "/rest/api/3/field")
            .count();
        assert_eq!(field_requests, 2);
        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
    }

    #[tokio::test]
    async fn syncs_everything_when_the_site_caps_the_page_size_below_what_is_asked() {
        let rig = Rig::connected().await;
        rig.fake.cap_max_results(10);

        rig.full_sync().await;

        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
        let asked: Vec<Value> = rig
            .fake
            .requests()
            .into_iter()
            .filter(|request| request.path == SEARCH_PATH)
            .map(|request| request.body["maxResults"].clone())
            .collect();
        assert_eq!(asked, vec![json!(100); 10]);
    }

    #[tokio::test]
    async fn untracked_projects_are_not_searched() {
        let rig = Rig::connected().await;
        track_project(&rig.ctx.db, "10003".into(), false)
            .await
            .unwrap();

        rig.full_sync().await;

        let mut expected = expected_counts();
        expected.remove("LUM");
        assert_eq!(issues_per_project(&rig.ctx.db).await, expected);
    }

    #[tokio::test]
    async fn without_tracked_projects_nothing_is_searched() {
        let rig = Rig::connected().await;
        for id in PROJECT_IDS {
            track_project(&rig.ctx.db, id.into(), false).await.unwrap();
        }

        rig.full_sync().await;

        assert_eq!(rig.searches(), 0);
        assert_eq!(state_rows(&rig.ctx.db).await, vec![]);
    }
}

mod running_a_job {
    use super::*;

    #[tokio::test]
    async fn a_finished_job_is_idle_with_its_finish_time_after_announcing_that_it_runs() {
        let rig = Rig::connected().await;

        let state = rig.run(None).await;

        match state {
            SyncState::Idle { last_ok: Some(at) } => assert!(at.ends_with('Z'), "{at}"),
            other => panic!("expected idle after a sync, got {other:?}"),
        }
        assert_eq!(rig.states(), vec![SyncState::Running { job: Job::Full }]);
        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
    }

    #[tokio::test]
    async fn a_server_error_on_a_search_page_ends_in_an_error_and_leaves_the_sync_state_alone() {
        let rig = Rig::connected().await;
        rig.fake.set_page_size(PAGE as usize);
        rig.full_sync().await;
        let before = state_rows(&rig.ctx.db).await;
        let scope_before = scope_full_at(&rig.ctx.db).await;
        rig.fake.set_now(later());
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["priority"] = json!({ "id": "4", "name": "Low" });
        });
        rig.fake.set_latency(Duration::from_millis(30));
        let emit = rig.emit();

        let (state, ()) = tokio::join!(
            run_job(&rig.ctx, Job::Full, &*emit, None),
            after_the_next_search_request(&rig, || rig.fake.fail_next(1, 500, None))
        );

        match state {
            SyncState::Error { kind, message } => {
                assert_eq!(kind, ErrorKind::Internal);
                assert_eq!(message, format!("Jira answered HTTP 500 to {SEARCH_PATH}"));
            }
            other => panic!("expected an error, got {other:?}"),
        }
        assert_eq!(state_rows(&rig.ctx.db).await, before);
        assert_eq!(scope_full_at(&rig.ctx.db).await, scope_before);
        assert_eq!(
            text_of(
                &rig.ctx.db,
                "SELECT priority FROM cache.issue WHERE key = 'NW-1049'"
            )
            .await,
            "Highest"
        );
    }

    #[tokio::test]
    async fn a_failed_first_sync_writes_no_scope_row() {
        let rig = Rig::connected().await;
        rig.fake.set_page_size(PAGE as usize);
        rig.fake.set_latency(Duration::from_millis(30));
        let emit = rig.emit();

        let (state, ()) = tokio::join!(
            run_job(&rig.ctx, Job::Full, &*emit, None),
            after_the_next_search_request(&rig, || rig.fake.fail_next(1, 500, None))
        );

        assert!(matches!(state, SyncState::Error { .. }), "{state:?}");
        assert_eq!(state_rows(&rig.ctx.db).await, vec![]);
    }

    #[tokio::test]
    async fn refused_credentials_pause_the_sync() {
        let rig = Rig::connected().await;
        rig.fake.fail_next(1, 401, None);

        let state = rig.run(None).await;

        assert!(matches!(state, SyncState::Paused { .. }), "{state:?}");
        assert_eq!(issues_per_project(&rig.ctx.db).await, BTreeMap::new());
    }

    #[tokio::test]
    async fn an_unreachable_site_is_offline_and_no_request_gets_through() {
        let rig = Rig::connected().await;
        rig.fake.go_offline().await;
        let before = rig.requests();

        let state = rig.run(None).await;

        match state {
            SyncState::Offline { since } => assert!(since.ends_with('Z'), "{since}"),
            other => panic!("expected offline, got {other:?}"),
        }
        assert_eq!(rig.requests(), before);
        assert_eq!(issues_per_project(&rig.ctx.db).await, BTreeMap::new());
    }

    #[tokio::test]
    async fn a_site_that_comes_back_syncs_on_the_next_run() {
        let rig = Rig::connected().await;
        rig.fake.go_offline().await;
        assert!(matches!(rig.run(None).await, SyncState::Offline { .. }));
        rig.fake.go_online().await;

        let state = rig.run(None).await;

        assert!(
            matches!(state, SyncState::Idle { last_ok: Some(_) }),
            "{state:?}"
        );
        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
    }

    #[tokio::test]
    async fn without_a_saved_connection_nothing_runs_and_nothing_is_sent() {
        let rig = Rig::new().await;
        let last_ok = Some("2026-10-27T09:00:00.000Z".to_string());

        let state = rig.run(last_ok.clone()).await;

        assert_eq!(state, SyncState::Idle { last_ok });
        assert_eq!(rig.requests(), 0);
        assert_eq!(rig.states(), vec![]);
    }

    #[tokio::test]
    async fn without_a_token_nothing_runs_and_nothing_is_sent() {
        let rig = Rig::new().await;
        save(
            &rig.ctx.settings,
            &rig.ctx.access,
            rig.input(),
            None,
            today(),
        )
        .await
        .unwrap();

        let state = rig.run(None).await;

        assert_eq!(state, SyncState::Idle { last_ok: None });
        assert_eq!(rig.requests(), 0);
        assert_eq!(rig.states(), vec![]);
    }
}

mod the_overview {
    use super::*;

    /// `(id, key, name, cached, open, has last_full_at)`
    type Summary = (String, String, String, u32, u32, bool);

    fn summaries(overview: &taskboard_lib::sync::SyncOverview) -> Vec<Summary> {
        overview
            .projects
            .iter()
            .map(|p| {
                (
                    p.id.clone(),
                    p.key.clone(),
                    p.name.clone(),
                    p.cached,
                    p.open,
                    p.last_full_at.is_some(),
                )
            })
            .collect()
    }

    fn summary(id: &str, key: &str, name: &str, cached: u32, open: u32, synced: bool) -> Summary {
        (id.into(), key.into(), name.into(), cached, open, synced)
    }

    #[tokio::test]
    async fn counts_cached_and_open_issues_per_tracked_project_in_space_order() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        let state = SyncState::Idle { last_ok: None };

        let report = overview(&rig.ctx.db, state.clone()).await.unwrap();

        assert_eq!(
            summaries(&report),
            vec![
                summary("10001", "NW", "Website relaunch", 33, 21, true),
                summary("10002", "NWS", "Northwind support", 8, 6, true),
                summary("10003", "LUM", "Lumen iOS app", 11, 9, true),
                summary("10004", "FBA", "Fernweh booking API", 13, 11, true),
                summary("10005", "INT", "Internal tools", 9, 8, true),
            ]
        );
        assert_eq!(report.state, state);
        assert_eq!(report.last_full_at, scope_full_at(&rig.ctx.db).await);
        assert!(report.last_full_at.is_some());
    }

    #[tokio::test]
    async fn a_project_shows_when_it_was_last_synced() {
        let rig = Rig::connected().await;
        rig.full_sync().await;

        let report = overview(&rig.ctx.db, SyncState::Idle { last_ok: None })
            .await
            .unwrap();

        let stored = text_of(
            &rig.ctx.db,
            "SELECT last_full_at FROM cache.sync_state WHERE scope_id = 'full:10001'",
        )
        .await;
        assert_eq!(
            report.projects[0].last_full_at.as_deref(),
            Some(stored.as_str())
        );
    }

    #[tokio::test]
    async fn before_any_sync_the_tracked_projects_are_empty_and_never_synced() {
        let rig = Rig::connected().await;

        let report = overview(&rig.ctx.db, SyncState::Idle { last_ok: None })
            .await
            .unwrap();

        assert_eq!(report.last_full_at, None);
        assert_eq!(
            summaries(&report),
            vec![
                summary("10001", "NW", "Website relaunch", 0, 0, false),
                summary("10002", "NWS", "Northwind support", 0, 0, false),
                summary("10003", "LUM", "Lumen iOS app", 0, 0, false),
                summary("10004", "FBA", "Fernweh booking API", 0, 0, false),
                summary("10005", "INT", "Internal tools", 0, 0, false),
            ]
        );
    }

    #[tokio::test]
    async fn an_untracked_project_leaves_the_overview() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        track_project(&rig.ctx.db, "10002".into(), false)
            .await
            .unwrap();

        let report = overview(&rig.ctx.db, SyncState::Idle { last_ok: None })
            .await
            .unwrap();

        let keys: Vec<&str> = report.projects.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys, ["NW", "LUM", "FBA", "INT"]);
    }

    #[tokio::test]
    async fn done_issues_are_cached_but_not_open() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.fake.set_now(later());
        rig.fake
            .edit_issue("NW-1044", |fields| fields["status"] = done_status());
        rig.full_sync().await;

        let report = overview(&rig.ctx.db, SyncState::Idle { last_ok: None })
            .await
            .unwrap();

        assert_eq!(
            (report.projects[0].cached, report.projects[0].open),
            (33, 20)
        );
    }

    #[tokio::test]
    async fn serializes_for_the_page_in_camel_case() {
        let rig = Rig::connected().await;

        let report = overview(
            &rig.ctx.db,
            SyncState::Offline {
                since: LATER_UTC.into(),
            },
        )
        .await
        .unwrap();

        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(
            value["state"],
            json!({ "state": "offline", "since": LATER_UTC })
        );
        assert_eq!(value["lastFullAt"], Value::Null);
        assert_eq!(
            value["projects"][0],
            json!({
                "id": "10001",
                "key": "NW",
                "name": "Website relaunch",
                "cached": 0,
                "open": 0,
                "lastFullAt": null,
            })
        );
    }
}

mod the_engine {
    use super::*;

    fn is_synced(state: &SyncState) -> bool {
        matches!(state, SyncState::Idle { last_ok: Some(_) })
    }

    #[tokio::test]
    async fn starting_on_a_cache_that_was_never_synced_runs_a_full_sync_by_itself() {
        let rig = Rig::connected().await;

        let _engine = SyncEngine::start(rig.ctx.clone(), rig.emit());
        wait_until("the first sync to finish", || {
            rig.states().iter().any(is_synced)
        })
        .await;

        let states = rig.states();
        assert_eq!(states[0], SyncState::Idle { last_ok: None });
        assert_eq!(states[1], SyncState::Running { job: Job::Full });
        assert!(is_synced(&states[2]), "{states:?}");
        assert_eq!(states.len(), 3);
        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
        assert_eq!(rig.progress(), expected_progress(100));
    }

    #[tokio::test]
    async fn the_state_reports_running_while_a_full_sync_runs_and_idle_after_it() {
        let rig = Rig::connected().await;
        rig.fake.set_latency(Duration::from_millis(50));

        let engine = SyncEngine::start(rig.ctx.clone(), rig.emit());
        wait_until("the state to report the running sync", || {
            engine.state() == SyncState::Running { job: Job::Full }
        })
        .await;
        wait_until("the state to be idle after the sync", || {
            is_synced(&engine.state())
        })
        .await;

        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
    }

    #[tokio::test]
    async fn a_synced_cache_waits_for_a_queued_job_and_then_runs_it() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        let synced_at = scope_full_at(&rig.ctx.db).await;
        let before = rig.requests();

        let engine = SyncEngine::start(rig.ctx.clone(), rig.emit());
        wait_until("the engine to report its state", || {
            !rig.states().is_empty()
        })
        .await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        assert_eq!(rig.states(), vec![SyncState::Idle { last_ok: synced_at }]);
        assert_eq!(rig.requests(), before);
        assert!(is_synced(&engine.state()));

        engine.queue(Job::Full);
        wait_until("the queued sync to finish", || {
            let states = rig.states();
            states.contains(&SyncState::Running { job: Job::Full })
                && states.last().is_some_and(is_synced)
        })
        .await;

        assert!(rig.requests() > before);
    }

    #[tokio::test]
    async fn refused_credentials_pause_the_engine_until_the_connection_changes() {
        let rig = Rig::connected().await;
        rig.fake.fail_next(1, 401, None);

        let engine = SyncEngine::start(rig.ctx.clone(), rig.emit());
        wait_until("the engine to pause", || {
            matches!(engine.state(), SyncState::Paused { .. })
        })
        .await;
        let paused_requests = rig.requests();
        engine.queue(Job::Full);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(matches!(engine.state(), SyncState::Paused { .. }));
        assert_eq!(rig.requests(), paused_requests);

        engine.connection_changed();
        engine.queue(Job::Full);
        wait_until("the sync after the pause", || is_synced(&engine.state())).await;

        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
    }
}

fn low_priority(fields: &mut Value) {
    fields["priority"] = json!({ "id": "4", "name": "Low" });
}

async fn priority_of(db: &Db, key: &str) -> String {
    let key = key.to_string();
    db.read(move |conn| {
        Ok(conn.query_row(
            "SELECT priority FROM cache.issue WHERE key = ?1",
            [key],
            |row| row.get::<_, String>(0),
        )?)
    })
    .await
    .unwrap()
}

mod the_delta_sync {
    use super::*;

    fn idle_after_a_sync(state: &SyncState) -> bool {
        matches!(state, SyncState::Idle { last_ok: Some(_) })
    }

    #[tokio::test]
    async fn without_changes_on_the_site_nothing_changes_and_nothing_is_announced() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        let cursor = rig.cursor().await;
        rig.forget_announcements();

        let state = rig.job(Job::Delta, None).await;

        assert!(idle_after_a_sync(&state), "{state:?}");
        assert_eq!(changes(&rig.ctx.db).await, vec![]);
        assert_eq!(rig.announced_ids(), BTreeSet::new());
        assert_eq!(rig.cursor().await, cursor);
        assert_eq!(
            count(&rig.ctx.db, "SELECT COUNT(*) FROM cache.issue").await,
            TOTAL
        );
    }

    #[tokio::test]
    async fn an_edited_issue_arrives_with_its_change_and_moves_the_cursor_to_the_newest_update() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.forget_announcements();
        rig.fake.set_now(later());
        rig.fake.edit_issue("NW-1049", low_priority);

        let state = rig.job(Job::Delta, None).await;

        assert!(idle_after_a_sync(&state), "{state:?}");
        assert_eq!(
            changes(&rig.ctx.db).await,
            vec![(
                "NW-1049".into(),
                "priority".into(),
                Some("Highest".into()),
                Some("Low".into()),
                LATER_UTC.into()
            )]
        );
        assert_eq!(priority_of(&rig.ctx.db, "NW-1049").await, "Low");
        assert_eq!(rig.announced_ids(), BTreeSet::from([NW_1049.to_string()]));
        assert_eq!(rig.cursor().await.as_deref(), Some(LATER_UTC));
    }

    #[tokio::test]
    async fn a_new_issue_on_the_site_arrives_as_created() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.fake.set_now(later());
        let key = rig
            .fake
            .add_issue("NW-1055", "NW", "Order confirmation email: shipping notice");

        rig.job(Job::Delta, None).await;

        assert_eq!(
            changes(&rig.ctx.db).await,
            vec![(
                key.clone(),
                "created".into(),
                None,
                Some(key),
                LATER_UTC.into()
            )]
        );
        assert_eq!(
            count(&rig.ctx.db, "SELECT COUNT(*) FROM cache.issue").await,
            TOTAL + 1
        );
    }

    #[tokio::test]
    async fn searches_the_scope_updated_since_the_cursor_minus_two_minutes_in_the_users_zone() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        let cursor = rig.cursor().await.unwrap();
        let since = cursor.parse::<DateTime<Utc>>().unwrap() - chrono::Duration::minutes(2);
        let literal = date_literal(since, "Europe/Vienna").unwrap();
        let scope = default_scope(&tracked_keys(), 90).unwrap();

        rig.job(Job::Delta, None).await;

        let jqls = rig.search_jqls();
        assert_eq!(
            jqls.last().unwrap(),
            &format!("({scope}) AND updated >= \"{literal}\" ORDER BY updated ASC")
        );
    }

    #[tokio::test]
    async fn a_failed_page_keeps_the_cursor_and_the_next_run_fetches_the_change() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        let cursor = rig.cursor().await;
        rig.fake.set_now(later());
        rig.fake.edit_issue("NW-1049", low_priority);
        rig.fake.set_latency(Duration::from_millis(30));
        let emit = rig.emit();

        let (state, ()) = tokio::join!(
            run_job(&rig.ctx, Job::Delta, &*emit, None),
            after_the_next_search_request(&rig, || rig.fake.fail_next(1, 500, None))
        );

        match state {
            SyncState::Error { kind, message } => {
                assert_eq!(kind, ErrorKind::Internal);
                assert_eq!(message, format!("Jira answered HTTP 500 to {SEARCH_PATH}"));
            }
            other => panic!("expected an error, got {other:?}"),
        }
        assert_eq!(rig.cursor().await, cursor);
        assert_eq!(priority_of(&rig.ctx.db, "NW-1049").await, "Highest");

        let retry = rig.job(Job::Delta, None).await;

        assert!(idle_after_a_sync(&retry), "{retry:?}");
        assert_eq!(priority_of(&rig.ctx.db, "NW-1049").await, "Low");
        assert_eq!(rig.cursor().await.as_deref(), Some(LATER_UTC));
    }

    #[tokio::test]
    async fn without_a_cursor_it_runs_a_full_sync() {
        let rig = Rig::connected().await;
        assert_eq!(rig.cursor().await, None);

        let state = rig.job(Job::Delta, None).await;

        assert!(idle_after_a_sync(&state), "{state:?}");
        assert_eq!(issues_per_project(&rig.ctx.db).await, expected_counts());
        assert_eq!(rig.progress(), expected_progress(100));
        assert_eq!(rig.cursor().await, Some(newest_updated_in_scope()));
    }

    #[tokio::test]
    async fn offline_it_reports_offline_and_changes_nothing() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        let cursor = rig.cursor().await;
        rig.fake.set_now(later());
        rig.fake.edit_issue("NW-1049", low_priority);
        rig.fake.go_offline().await;
        let before = rig.requests();

        let state = rig.job(Job::Delta, None).await;

        match state {
            SyncState::Offline { since } => assert!(since.ends_with('Z'), "{since}"),
            other => panic!("expected offline, got {other:?}"),
        }
        assert_eq!(rig.requests(), before);
        assert_eq!(rig.cursor().await, cursor);
        assert_eq!(priority_of(&rig.ctx.db, "NW-1049").await, "Highest");
        assert_eq!(changes(&rig.ctx.db).await, vec![]);
    }
}

mod reconciliation {
    use super::*;

    const FAR_PAST: &str = "2000-01-01T00:00:00.000Z";

    async fn gone_flag(db: &Db, key: &str) -> i64 {
        let key = key.to_string();
        db.read(move |conn| {
            Ok(conn.query_row(
                "SELECT gone FROM cache.issue WHERE key = ?1",
                [key],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap()
    }

    /// `(issue key, field)` of the gone changes.
    async fn gone_changes(db: &Db) -> Vec<(String, String)> {
        changes(db)
            .await
            .into_iter()
            .filter(|change| change.1 == "gone")
            .map(|change| (change.0, change.1))
            .collect()
    }

    async fn assert_issue_leaves_the_cache_views(rig: &Rig, id: &str, key: &str) {
        let (everything, nw) = rig.open_counts().await;
        let state = rig.job(Job::Reconcile, None).await;

        assert!(
            matches!(state, SyncState::Idle { last_ok: Some(_) }),
            "{state:?}"
        );
        assert_eq!(gone_flag(&rig.ctx.db, key).await, 1);
        assert_eq!(
            gone_changes(&rig.ctx.db).await,
            vec![(key.to_string(), "gone".to_string())]
        );
        assert_eq!(rig.announced_ids(), BTreeSet::from([id.to_string()]));
        assert_eq!(rig.open_counts().await, (everything.map(|n| n - 1), nw - 1));
    }

    #[tokio::test]
    async fn a_deleted_issue_becomes_gone_and_leaves_the_lists_and_counts() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.forget_announcements();
        rig.fake.delete_issue("NW-1044");

        assert_issue_leaves_the_cache_views(&rig, NW_1044, "NW-1044").await;
    }

    #[tokio::test]
    async fn an_issue_the_user_lost_access_to_becomes_gone_too() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.forget_announcements();
        rig.fake.revoke_access("NW-1044");

        assert_issue_leaves_the_cache_views(&rig, NW_1044, "NW-1044").await;
    }

    #[tokio::test]
    async fn without_deletions_nothing_changes_and_the_run_is_recorded() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.forget_announcements();

        let state = rig.job(Job::Reconcile, None).await;

        assert!(
            matches!(state, SyncState::Idle { last_ok: Some(_) }),
            "{state:?}"
        );
        assert_eq!(changes(&rig.ctx.db).await, vec![]);
        assert_eq!(rig.announced_ids(), BTreeSet::new());
        assert_eq!(
            count(
                &rig.ctx.db,
                "SELECT COUNT(*) FROM cache.issue WHERE gone = 1"
            )
            .await,
            0
        );
        assert_eq!(
            count(
                &rig.ctx.db,
                "SELECT COUNT(*) FROM cache.sync_state
                 WHERE scope_id = 'scope' AND last_reconcile_at IS NOT NULL"
            )
            .await,
            1
        );
    }

    #[tokio::test]
    async fn an_issue_moved_out_of_the_scope_is_stored_with_its_new_key_and_project_and_not_gone() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.fake.set_now(later());
        rig.fake.move_issue("NW-1049", "ARC");

        rig.job(Job::Reconcile, None).await;

        let db = &rig.ctx.db;
        assert_eq!(
            text_of(
                db,
                "SELECT p.key FROM cache.issue i JOIN cache.project p ON p.id = i.project_id
                 WHERE i.id = '101049'"
            )
            .await,
            "ARC"
        );
        let key = text_of(db, "SELECT key FROM cache.issue WHERE id = '101049'").await;
        assert!(key.starts_with("ARC-"), "{key}");
        assert_eq!(
            count(db, "SELECT gone FROM cache.issue WHERE id = '101049'").await,
            0
        );
        assert_eq!(gone_changes(db).await, vec![]);
    }

    #[tokio::test]
    async fn purges_issues_gone_for_more_than_a_week_with_their_comments_and_changes() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.fake.delete_issue("NW-1042");
        rig.fake.delete_issue("NW-1049");
        rig.job(Job::Reconcile, None).await;
        let db = &rig.ctx.db;
        assert_eq!(gone_flag(db, "NW-1042").await, 1);
        db.write(|tx, _| {
            tx.execute(
                "UPDATE cache.issue_change SET changed_at = ?1
                 WHERE field = 'gone' AND issue_id = '101042'",
                [FAR_PAST],
            )?;
            Ok(())
        })
        .await
        .unwrap();

        rig.job(Job::Reconcile, None).await;

        for sql in [
            "SELECT COUNT(*) FROM cache.issue WHERE id = '101042'",
            "SELECT COUNT(*) FROM cache.comment WHERE issue_id = '101042'",
            "SELECT COUNT(*) FROM cache.issue_change WHERE issue_id = '101042'",
            "SELECT COUNT(*) FROM cache.issue_fts WHERE rowid = 101042",
        ] {
            assert_eq!(count(db, sql).await, 0, "{sql}");
        }
        assert_eq!(gone_flag(db, "NW-1049").await, 1);
        assert_eq!(
            count(db, "SELECT COUNT(*) FROM cache.issue").await,
            TOTAL - 1
        );
    }

    #[tokio::test]
    async fn a_full_sync_ends_with_a_reconcile() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.fake.delete_issue("NW-1044");

        let state = rig.run(None).await;

        assert!(
            matches!(state, SyncState::Idle { last_ok: Some(_) }),
            "{state:?}"
        );
        assert_eq!(gone_flag(&rig.ctx.db, "NW-1044").await, 1);
    }

    #[tokio::test]
    async fn offline_it_reports_offline_and_changes_nothing() {
        let rig = Rig::connected().await;
        rig.full_sync().await;
        rig.fake.delete_issue("NW-1044");
        rig.fake.go_offline().await;
        let before = rig.requests();

        let state = rig.job(Job::Reconcile, None).await;

        assert!(matches!(state, SyncState::Offline { .. }), "{state:?}");
        assert_eq!(rig.requests(), before);
        assert_eq!(gone_flag(&rig.ctx.db, "NW-1044").await, 0);
    }
}
