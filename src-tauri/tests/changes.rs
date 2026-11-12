//! Unseen changes on the demo site synced from the in-process fake Jira into a temp database: what the inbox
//! lists and counts, Mark seen, Mark unread, and the Since banner of an issue.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use tempfile::TempDir;

use taskboard_lib::changes::{Since, UNSEEN, mark_seen, mark_unread};
use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, fetch_versions, refresh_projects, save, track_project,
};
use taskboard_lib::db::{Changes, Db};
use taskboard_lib::error::ErrorKind;
use taskboard_lib::inbox;
use taskboard_lib::issues::{ActivityItem, issue_activity, issue_get, issue_related};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::{Kind, SidebarTree, Viewer, sidebar_tree};
use taskboard_lib::pins::{PinDuration, PinKind, pin_create};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::{Emit, Job, SyncContext, full_sync, run_job};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::{Narrowing, ViewData, view_rows, views_of};

const ME_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const NW: &str = "10001";

const EIGHT: &str = "2026-10-28T08:00:00.000Z";
const NINE: &str = "2026-10-28T09:00:00.000Z";
const TEN: &str = "2026-10-28T10:00:00.000Z";
const LATER: &str = "2026-10-28T11:30:00.000Z";
const AHEAD: &str = "2026-10-28T12:30:00.000Z";
const DELTA_AT: &str = "2026-10-28T13:00:00.000Z";
const LAST_WEEK: &str = "2026-10-20T08:00:00.000Z";
const THREE_DAYS_AGO: &str = "2026-10-25T09:00:00.000Z";

fn viewer() -> Viewer {
    Viewer {
        me: Some(ME_ACCOUNT_ID.into()),
        time_zone: Some("Europe/Berlin".into()),
        now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
    }
}

/// Collects what the database announces from now on.
fn watch(db: &Db) -> Arc<Mutex<Vec<Changes>>> {
    let announced = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&announced);
    db.set_notifier(Box::new(move |changes: &Changes| {
        sink.lock().unwrap().push(changes.clone());
    }));
    announced
}

struct Site {
    fake: FakeJira,
    _dir: TempDir,
    ctx: SyncContext,
}

impl Site {
    async fn new() -> Self {
        let dir = temp_config_dir();
        let ctx = SyncContext {
            settings: SettingsStore::open(dir.path()),
            access: JiraAccess::new(
                Arc::new(FakeSecrets::default()),
                Arc::new(Limiter::new(1000, 1000)),
            ),
            db: Db::open(dir.path()).unwrap(),
        };
        let fake = FakeJira::start_demo().await;
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
            NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(),
        )
        .await
        .unwrap();
        let client = fake.client(TokenKind::Classic);
        refresh_projects(&ctx.db, &client).await.unwrap();
        for id in PROJECT_IDS {
            fetch_versions(&ctx.db, &client, id.into()).await.unwrap();
            track_project(&ctx.db, id.into(), true).await.unwrap();
        }
        let emit: Emit = Arc::new(|_| {});
        full_sync(&ctx, &*emit).await.unwrap();
        Self {
            fake,
            _dir: dir,
            ctx,
        }
    }

    fn db(&self) -> &Db {
        &self.ctx.db
    }

    async fn id_of(&self, key: &str) -> String {
        self.text(format!("SELECT id FROM cache.issue WHERE key = '{key}'"))
            .await
    }

    async fn text(&self, sql: String) -> String {
        self.db()
            .read(move |conn| Ok(conn.query_row(&sql, [], |r| r.get::<_, String>(0))?))
            .await
            .unwrap()
    }

    async fn nw_list(&self) -> i64 {
        self.text(format!(
            "SELECT CAST(id AS TEXT) FROM filter_list WHERE kind = 'space' AND project_id = '{NW}'"
        ))
        .await
        .parse()
        .unwrap()
    }

    async fn data_of(&self, list: i64) -> ViewData {
        let view = views_of(self.db(), list).await.unwrap()[0].id;
        view_rows(self.db(), viewer(), list, view, Narrowing::default())
            .await
            .unwrap()
    }

    async fn nw_data(&self) -> ViewData {
        self.data_of(self.nw_list().await).await
    }

    /// The inbox's note of the issue, none when the inbox doesn't list it.
    async fn note_of(&self, key: &str) -> Option<String> {
        let listed = self
            .db()
            .read(|conn| {
                let taken = inbox::snapshot(conn, Some(ME_ACCOUNT_ID))?;
                inbox::rows(conn, &taken)
            })
            .await
            .unwrap();
        listed
            .rows
            .into_iter()
            .find(|row| row.key == key)
            .map(|row| row.note)
    }

    /// The keys the inbox lists, sorted.
    async fn inbox_keys(&self) -> Vec<String> {
        let listed = self
            .db()
            .read(|conn| {
                let taken = inbox::snapshot(conn, Some(ME_ACCOUNT_ID))?;
                inbox::rows(conn, &taken)
            })
            .await
            .unwrap();
        let mut keys: Vec<String> = listed.rows.into_iter().map(|row| row.key).collect();
        keys.sort();
        keys
    }

    async fn run(&self, sql: String) {
        self.db()
            .write(move |tx, _| {
                tx.execute(&sql, [])?;
                Ok(())
            })
            .await
            .unwrap();
    }

    async fn record_change(
        &self,
        key: &str,
        field: &str,
        old: Option<&str>,
        new: Option<&str>,
        at: &str,
    ) {
        let quote = |v: Option<&str>| v.map_or("NULL".to_string(), |v| format!("'{v}'"));
        self.run(format!(
            "INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
             SELECT id, '{field}', {}, {}, '{at}' FROM cache.issue WHERE key = '{key}'",
            quote(old),
            quote(new)
        ))
        .await;
    }

    async fn set_seen(&self, key: &str, at: &str) {
        self.run(format!(
            "INSERT INTO issue_seen (issue_id, seen_at)
             SELECT id, '{at}' FROM cache.issue WHERE key = '{key}'
             ON CONFLICT (issue_id) DO UPDATE SET seen_at = excluded.seen_at"
        ))
        .await;
    }

    async fn forget_seen(&self, key: &str) {
        self.run(format!(
            "DELETE FROM issue_seen WHERE issue_id = (SELECT id FROM cache.issue WHERE key = '{key}')"
        ))
        .await;
    }

    async fn seen_at(&self, key: &str) -> String {
        self.text(format!(
            "SELECT seen_at FROM issue_seen WHERE issue_id = (SELECT id FROM cache.issue WHERE key = '{key}')"
        ))
        .await
    }

    async fn mark(&self, keys: &[&str], now: &str) {
        let mut ids = Vec::new();
        for key in keys {
            ids.push(self.id_of(key).await);
        }
        mark_seen(self.db(), ids, now.into()).await.unwrap();
    }

    /// An issue seen at 08:00 with a comment added at 09:00.
    async fn comment_after_seen(&self, key: &str) {
        self.set_seen(key, EIGHT).await;
        self.record_change(key, "comment_added", Some("4"), Some("5"), NINE)
            .await;
    }

    async fn detail_unread(&self, key: &str) -> bool {
        let id = self.id_of(key).await;
        issue_get(self.db(), viewer(), id).await.unwrap().unread
    }

    async fn detail_since(&self, key: &str) -> Option<Since> {
        let id = self.id_of(key).await;
        issue_get(self.db(), viewer(), id).await.unwrap().since
    }

    /// The bell's badge: how many tracked issues are unread.
    async fn badge(&self) -> u32 {
        self.db().read(inbox::count).await.unwrap()
    }

    async fn tree(&self) -> SidebarTree {
        sidebar_tree(self.db(), viewer()).await.unwrap()
    }
}

impl Site {
    async fn delta_sync(&self) {
        let emit: Emit = Arc::new(|_| {});
        run_job(&self.ctx, Job::Delta, &*emit, None).await;
    }

    /// Everything counts as read at 10:00 and Jira's clock moves on to the delta's time.
    async fn quiet_until_the_delta(&self) {
        self.run(format!("UPDATE issue_seen SET seen_at = '{TEN}'"))
            .await;
        self.fake
            .set_now(DELTA_AT.parse::<DateTime<Utc>>().unwrap());
    }

    /// The cache is thrown away and filled by a full sync again, as after a reset or a restart in memory mode;
    /// what the owner has read stays.
    async fn refill_cache(&self) {
        self.db().reset_cache().await.unwrap();
        refresh_projects(self.db(), &self.fake.client(TokenKind::Classic))
            .await
            .unwrap();
        for id in PROJECT_IDS {
            track_project(self.db(), id.into(), true).await.unwrap();
        }
        let emit: Emit = Arc::new(|_| {});
        full_sync(&self.ctx, &*emit).await.unwrap();
    }

    async fn strings(&self, sql: String) -> Vec<String> {
        self.db()
            .read(move |conn| {
                let mut query = conn.prepare(&sql)?;
                let found = query
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(found)
            })
            .await
            .unwrap()
    }

    async fn number(&self, sql: &str) -> i64 {
        self.text(format!("SELECT CAST(({sql}) AS TEXT)"))
            .await
            .parse()
            .unwrap()
    }

    /// The fields of the changes recorded for the issue, oldest first.
    async fn recorded_fields(&self, key: &str) -> Vec<String> {
        self.strings(format!(
            "SELECT c.field FROM cache.issue_change c JOIN cache.issue i ON i.id = c.issue_id
             WHERE i.key = '{key}' ORDER BY c.id"
        ))
        .await
    }

    /// The keys of the issues with unseen changes or marked unread, sorted.
    async fn unread_keys(&self) -> Vec<String> {
        let mut keys = self
            .strings(format!(
                "SELECT i.key FROM cache.issue i WHERE i.gone = 0 AND {UNSEEN}"
            ))
            .await;
        keys.sort();
        keys
    }
}

mod unread_issues {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_first_sync_leaves_no_issue_unread() {
        let site = Site::new().await;

        assert_eq!(site.inbox_keys().await, Vec::<String>::new());
        assert_eq!(site.unread_keys().await, Vec::<String>::new());
        assert_eq!(site.badge().await, 0);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_change_after_the_seen_time_makes_the_issue_unread_with_a_note() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;

        assert_eq!(
            site.note_of("NW-1042").await.as_deref(),
            Some("1 new comment.")
        );
        assert_eq!(site.note_of("NW-1044").await, None);
        assert_eq!(site.badge().await, 1);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_change_before_the_seen_time_leaves_the_issue_read() {
        let site = Site::new().await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), NINE)
            .await;
        site.set_seen("NW-1042", TEN).await;

        assert_eq!(site.note_of("NW-1042").await, None);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_change_without_any_seen_row_makes_the_issue_unread() {
        let site = Site::new().await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), NINE)
            .await;
        site.forget_seen("NW-1042").await;

        assert_eq!(
            site.note_of("NW-1042").await.as_deref(),
            Some("1 new comment.")
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_gone_record_alone_never_makes_an_issue_unread() {
        let site = Site::new().await;
        site.set_seen("NW-1042", EIGHT).await;
        site.record_change("NW-1042", "gone", None, None, NINE)
            .await;

        assert_eq!(site.note_of("NW-1042").await, None);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_note_says_assigned_to_you_for_my_account_and_adds_the_other_changes() {
        let site = Site::new().await;
        site.set_seen("NW-1042", EIGHT).await;
        site.record_change("NW-1042", "assignee", None, Some(ME_ACCOUNT_ID), NINE)
            .await;
        site.record_change(
            "NW-1042",
            "due_date",
            Some("2026-10-30"),
            Some("2026-11-02"),
            TEN,
        )
        .await;

        assert_eq!(
            site.note_of("NW-1042").await.as_deref(),
            Some("Assigned to you, due date changed.")
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn reading_an_issue_never_moves_a_row_of_a_view() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("NW-1044").await;
        let before = site.nw_data().await;

        site.mark(&["NW-1042"], LATER).await;

        let after = site.nw_data().await;
        assert_eq!(after, before);
    }
}

mod marking_seen {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn mark_seen_clears_the_highlight_of_that_issue_only() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("NW-1044").await;

        site.mark(&["NW-1042"], LATER).await;

        assert_eq!(site.note_of("NW-1042").await, None);
        assert_eq!(
            site.note_of("NW-1044").await.as_deref(),
            Some("1 new comment.")
        );
        assert_eq!(site.seen_at("NW-1042").await, LATER);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn marking_an_unseen_issue_seen_announces_it() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        let announced = watch(site.db());

        site.mark(&["NW-1042"], LATER).await;

        assert_eq!(
            *announced.lock().unwrap(),
            [Changes {
                issues: BTreeSet::from([site.id_of("NW-1042").await]),
                ..Changes::default()
            }]
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn marking_an_already_read_issue_seen_announces_nothing() {
        let site = Site::new().await;
        site.set_seen("NW-1042", LATER).await;
        let announced = watch(site.db());

        site.mark(&["NW-1042"], TEN).await;

        assert!(announced.lock().unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn marking_several_issues_seen_announces_only_the_ones_that_were_unseen() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.set_seen("NW-1044", LATER).await;
        let announced = watch(site.db());

        site.mark(&["NW-1042", "NW-1044"], LATER).await;

        assert_eq!(
            *announced.lock().unwrap(),
            [Changes {
                issues: BTreeSet::from([site.id_of("NW-1042").await]),
                ..Changes::default()
            }]
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn mark_seen_uses_the_newest_change_time_when_jiras_clock_is_ahead() {
        let site = Site::new().await;
        site.set_seen("NW-1042", EIGHT).await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), AHEAD)
            .await;

        site.mark(&["NW-1042"], LATER).await;

        assert_eq!(site.seen_at("NW-1042").await, AHEAD);
        assert_eq!(site.note_of("NW-1042").await, None);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn mark_seen_never_moves_the_seen_time_back() {
        let site = Site::new().await;
        site.set_seen("NW-1042", AHEAD).await;

        site.mark(&["NW-1042"], LATER).await;

        assert_eq!(site.seen_at("NW-1042").await, AHEAD);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn mark_seen_creates_the_seen_row_of_an_issue_without_one() {
        let site = Site::new().await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), NINE)
            .await;
        site.forget_seen("NW-1042").await;

        site.mark(&["NW-1042"], LATER).await;

        assert_eq!(site.seen_at("NW-1042").await, LATER);
        assert_eq!(site.note_of("NW-1042").await, None);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn marking_seen_leaves_the_unseen_issues_of_other_spaces_unread() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("NW-1044").await;
        site.comment_after_seen("LUM-318").await;

        site.mark(&["NW-1042", "NW-1044"], LATER).await;

        assert_eq!(site.inbox_keys().await, ["LUM-318"]);
    }
}

mod inbox_listing {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_unread_subtask_type_issue_is_listed_and_counted() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1059").await;

        assert_eq!(site.inbox_keys().await, ["NW-1059"]);
        assert_eq!(site.badge().await, 1);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn lists_exactly_the_unseen_issues_including_done_ones() {
        let site = Site::new().await;
        let done = site
            .text(format!(
                "SELECT key FROM cache.issue
                 WHERE project_id = '{NW}' AND status_category = 'done' AND gone = 0 ORDER BY key LIMIT 1"
            ))
            .await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("LUM-318").await;
        site.comment_after_seen(&done).await;
        site.set_seen("NW-1044", EIGHT).await;
        site.record_change("NW-1044", "gone", None, None, NINE)
            .await;
        let mut expected = vec!["NW-1042".to_string(), "LUM-318".to_string(), done];
        expected.sort();

        assert_eq!(site.inbox_keys().await, expected);
        assert_eq!(site.badge().await, 3);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_updated_today_but_seen_is_not_listed() {
        let site = Site::new().await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), NINE)
            .await;
        site.set_seen("NW-1042", TEN).await;

        assert_eq!(site.inbox_keys().await, Vec::<String>::new());
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn marking_an_issue_seen_removes_it_from_the_list_and_the_badge() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("NW-1044").await;

        site.mark(&["NW-1042"], LATER).await;

        assert_eq!(site.inbox_keys().await, ["NW-1044"]);
        assert_eq!(site.badge().await, 1);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_sidebar_counts_the_open_issues_whatever_is_unread() {
        let site = Site::new().await;
        let bookmarked = site.id_of("NW-1044").await;
        pin_create(
            site.db(),
            viewer(),
            PinKind::Issue,
            bookmarked,
            PinDuration::UntilUnpinned,
            None,
        )
        .await
        .unwrap();
        let before = site.tree().await;

        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("LUM-318").await;
        let after = site.tree().await;

        assert_eq!(after, before);
        let bookmarks = after.home.iter().find(|n| n.kind == Kind::Pinned).unwrap();
        assert_eq!(bookmarks.count, Some(1));
    }
}

mod marking_unread {
    use super::*;

    async fn unread_flag(site: &Site, key: &str) -> bool {
        site.detail_unread(key).await
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_without_any_change_becomes_unread_everywhere() {
        let site = Site::new().await;
        let id = site.id_of("NW-1042").await;
        assert!(!unread_flag(&site, "NW-1042").await);

        mark_unread(site.db(), id, None).await.unwrap();

        assert_eq!(site.inbox_keys().await, ["NW-1042"]);
        assert_eq!(
            site.note_of("NW-1042").await.as_deref(),
            Some("Marked unread.")
        );
        assert_eq!(site.badge().await, 1);
        assert!(unread_flag(&site, "NW-1042").await);
        assert!(!unread_flag(&site, "NW-1044").await);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn without_a_time_the_seen_time_and_the_activity_stay_as_they_were() {
        let site = Site::new().await;
        site.set_seen("NW-1042", TEN).await;
        let id = site.id_of("NW-1042").await;

        mark_unread(site.db(), id.clone(), None).await.unwrap();

        assert_eq!(site.seen_at("NW-1042").await, TEN);
        let found = issue_activity(site.db(), viewer(), id).await.unwrap();
        assert!(!found.is_empty());
        for item in &found {
            let (ActivityItem::Comment { at, fresh, .. } | ActivityItem::Change { at, fresh, .. }) =
                item;
            assert_eq!(*fresh, at.as_str() > TEN, "{at}");
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn marking_a_never_seen_issue_unread_lists_it() {
        let site = Site::new().await;
        site.forget_seen("NW-1042").await;
        let id = site.id_of("NW-1042").await;

        mark_unread(site.db(), id, None).await.unwrap();

        assert_eq!(site.inbox_keys().await, ["NW-1042"]);
        assert!(unread_flag(&site, "NW-1042").await);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn mark_seen_clears_the_unread_flag_again() {
        let site = Site::new().await;
        let id = site.id_of("NW-1042").await;
        mark_unread(site.db(), id, None).await.unwrap();

        site.mark(&["NW-1042"], LATER).await;

        assert!(!unread_flag(&site, "NW-1042").await);
        assert_eq!(site.inbox_keys().await, Vec::<String>::new());
        assert_eq!(site.badge().await, 0);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn from_an_activity_time_that_item_and_newer_ones_are_fresh() {
        let site = Site::new().await;
        site.record_change(
            "NW-1042",
            "due_date",
            Some("2026-10-30"),
            Some("2026-11-02"),
            EIGHT,
        )
        .await;
        site.record_change(
            "NW-1042",
            "due_date",
            Some("2026-11-02"),
            Some("2026-11-05"),
            TEN,
        )
        .await;
        site.set_seen("NW-1042", LATER).await;
        let id = site.id_of("NW-1042").await;

        mark_unread(site.db(), id.clone(), Some(TEN.into()))
            .await
            .unwrap();

        assert_eq!(site.seen_at("NW-1042").await, "2026-10-28T09:59:59.999Z");
        let found = issue_activity(site.db(), viewer(), id).await.unwrap();
        let mut changes: Vec<(String, bool)> = found
            .iter()
            .filter_map(|item| match item {
                ActivityItem::Change { at, fresh, .. } => Some((at.clone(), *fresh)),
                ActivityItem::Comment { .. } => None,
            })
            .collect();
        changes.sort();
        assert_eq!(
            changes,
            [(EIGHT.to_string(), false), (TEN.to_string(), true)]
        );
        assert_eq!(site.inbox_keys().await, ["NW-1042"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_time_after_the_seen_time_never_moves_it_forward() {
        let site = Site::new().await;
        site.set_seen("NW-1042", EIGHT).await;
        let id = site.id_of("NW-1042").await;

        mark_unread(site.db(), id, Some(LATER.into()))
            .await
            .unwrap();

        assert_eq!(site.seen_at("NW-1042").await, EIGHT);
        assert!(unread_flag(&site, "NW-1042").await);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_time_that_is_not_rfc_3339_is_invalid_input() {
        let site = Site::new().await;
        let id = site.id_of("NW-1042").await;

        let err = mark_unread(site.db(), id, Some("yesterday".into()))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(!unread_flag(&site, "NW-1042").await);
    }
}

mod fresh_activity {
    use super::*;

    async fn items(site: &Site, key: &str) -> Vec<ActivityItem> {
        let id = site.id_of(key).await;
        issue_activity(site.db(), viewer(), id).await.unwrap()
    }

    fn freshness(item: &ActivityItem) -> bool {
        match item {
            ActivityItem::Comment { fresh, .. } | ActivityItem::Change { fresh, .. } => *fresh,
        }
    }

    /// `(at, fresh)` of the field changes, sorted oldest first for stable comparison.
    fn change_freshness(items: &[ActivityItem]) -> Vec<(String, bool)> {
        let mut found: Vec<(String, bool)> = items
            .iter()
            .filter_map(|item| match item {
                ActivityItem::Change { at, fresh, .. } => Some((at.clone(), *fresh)),
                ActivityItem::Comment { .. } => None,
            })
            .collect();
        found.sort();
        found
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn items_after_the_seen_time_are_fresh_while_the_issue_has_unseen_changes() {
        let site = Site::new().await;
        site.record_change(
            "NW-1042",
            "due_date",
            Some("2026-10-30"),
            Some("2026-11-02"),
            EIGHT,
        )
        .await;
        site.set_seen("NW-1042", NINE).await;
        site.record_change(
            "NW-1042",
            "due_date",
            Some("2026-11-02"),
            Some("2026-11-05"),
            TEN,
        )
        .await;

        let found = items(&site, "NW-1042").await;

        assert_eq!(
            change_freshness(&found),
            [(EIGHT.to_string(), false), (TEN.to_string(), true)]
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn every_item_is_fresh_when_the_issue_was_never_seen_and_has_changes() {
        let site = Site::new().await;
        site.record_change(
            "NW-1042",
            "due_date",
            Some("2026-10-30"),
            Some("2026-11-02"),
            EIGHT,
        )
        .await;
        site.forget_seen("NW-1042").await;

        let found = items(&site, "NW-1042").await;

        assert!(found.len() > 1);
        assert!(found.iter().all(freshness), "{found:?}");
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn nothing_is_fresh_after_the_issue_is_marked_seen() {
        let site = Site::new().await;
        site.set_seen("NW-1042", EIGHT).await;
        site.record_change(
            "NW-1042",
            "due_date",
            Some("2026-10-30"),
            Some("2026-11-02"),
            NINE,
        )
        .await;
        site.mark(&["NW-1042"], LATER).await;

        let found = items(&site, "NW-1042").await;

        assert!(!found.is_empty());
        assert!(found.iter().all(|item| !freshness(item)), "{found:?}");
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn nothing_is_fresh_on_an_issue_without_unseen_changes() {
        let site = Site::new().await;
        site.record_change(
            "NW-1042",
            "due_date",
            Some("2026-10-30"),
            Some("2026-11-02"),
            EIGHT,
        )
        .await;
        site.set_seen("NW-1042", NINE).await;

        let found = items(&site, "NW-1042").await;

        assert!(found.iter().all(|item| !freshness(item)), "{found:?}");
    }
}

mod since_banner {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn changes_from_today_read_since_yesterday_until_the_issue_is_marked_seen() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;

        let before = site.detail_since("NW-1042").await;
        site.mark(&["NW-1042"], LATER).await;
        let after = site.detail_since("NW-1042").await;

        assert_eq!(
            before,
            Some(Since {
                label: "Since yesterday".into(),
                text: "1 new comment.".into(),
            })
        );
        assert_eq!(after, None);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn changes_that_go_back_further_than_a_day_read_since_you_last_looked() {
        let site = Site::new().await;
        site.set_seen("NW-1042", LAST_WEEK).await;
        site.record_change(
            "NW-1042",
            "comment_added",
            Some("4"),
            Some("5"),
            THREE_DAYS_AGO,
        )
        .await;
        site.record_change("NW-1042", "comment_added", Some("5"), Some("6"), NINE)
            .await;

        let since = site.detail_since("NW-1042").await;

        assert_eq!(
            since,
            Some(Since {
                label: "Since you last looked".into(),
                text: "2 new comments.".into(),
            })
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_old_seen_time_with_only_recent_changes_still_reads_since_yesterday() {
        let site = Site::new().await;
        site.set_seen("NW-1042", LAST_WEEK).await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), NINE)
            .await;

        let since = site.detail_since("NW-1042").await;

        assert_eq!(
            since,
            Some(Since {
                label: "Since yesterday".into(),
                text: "1 new comment.".into(),
            })
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn without_a_seen_row_every_recorded_change_counts() {
        let site = Site::new().await;
        site.record_change(
            "NW-1042",
            "comment_added",
            Some("4"),
            Some("5"),
            THREE_DAYS_AGO,
        )
        .await;
        site.forget_seen("NW-1042").await;

        let since = site.detail_since("NW-1042").await;

        assert_eq!(
            since,
            Some(Since {
                label: "Since you last looked".into(),
                text: "1 new comment.".into(),
            })
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_without_unseen_changes_has_no_banner() {
        let site = Site::new().await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), NINE)
            .await;
        site.set_seen("NW-1042", TEN).await;

        assert_eq!(site.detail_since("NW-1042").await, None);
        assert_eq!(site.detail_since("NW-1044").await, None);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_sentence_names_the_status_the_issue_moved_to() {
        let site = Site::new().await;
        let review = site
            .text("SELECT id FROM cache.status WHERE name = 'In Review'".into())
            .await;
        site.set_seen("NW-1042", EIGHT).await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("6"), NINE)
            .await;
        site.record_change("NW-1042", "status", Some("10002"), Some(&review), TEN)
            .await;

        let since = site.detail_since("NW-1042").await;

        assert_eq!(
            since,
            Some(Since {
                label: "Since yesterday".into(),
                text: "2 new comments, moved to In Review.".into(),
            })
        );
    }
}

mod related_issues {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_epics_child_with_unseen_changes_is_changed_until_marked_seen() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        let epic = site.id_of("NW-1020").await;

        let changed_children = |related: taskboard_lib::issues::Related| -> Vec<String> {
            let mut keys: Vec<String> = related
                .children
                .open
                .into_iter()
                .chain(related.children.done)
                .filter(|child| child.changed)
                .map(|child| child.key)
                .collect();
            keys.sort();
            keys
        };
        let before = issue_related(site.db(), viewer(), epic.clone())
            .await
            .unwrap();
        site.mark(&["NW-1042"], LATER).await;
        let after = issue_related(site.db(), viewer(), epic).await.unwrap();

        assert_eq!(changed_children(before), ["NW-1042"]);
        assert_eq!(changed_children(after), Vec::<String>::new());
    }
}

mod after_a_delta_sync {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn what_the_site_did_shows_as_notes_on_the_edited_issues() {
        let site = Site::new().await;
        site.run(format!("UPDATE issue_seen SET seen_at = '{TEN}'"))
            .await;
        site.fake
            .set_now(DELTA_AT.parse::<DateTime<Utc>>().unwrap());
        site.fake.add_comment(
            "NW-1042",
            demo::demo().users[1].clone(),
            "Postcode rules for Liechtenstein are merged.",
        );
        let done = demo::demo()
            .statuses
            .into_iter()
            .find(|s| s["name"] == "Done")
            .unwrap();
        site.fake
            .edit_issue("NW-1044", |fields| fields["status"] = done);
        site.fake.edit_issue("NW-1049", |fields| {
            fields["priority"] = serde_json::json!({ "id": "4", "name": "Low" });
        });
        let added =
            site.fake
                .add_issue("NW-1055", "NW", "Order confirmation email: shipping notice");
        let emit: Emit = Arc::new(|_| {});

        run_job(&site.ctx, Job::Delta, &*emit, None).await;

        assert_eq!(
            site.note_of("NW-1042").await.as_deref(),
            Some("1 new comment.")
        );
        assert_eq!(
            site.note_of("NW-1044").await.as_deref(),
            Some("Moved to Done.")
        );
        assert_eq!(
            site.note_of("NW-1049").await.as_deref(),
            Some("Priority lowered.")
        );
        assert_eq!(site.note_of(&added).await.as_deref(), Some("Created."));
        let mut expected = vec![
            "NW-1042".to_string(),
            "NW-1044".to_string(),
            "NW-1049".to_string(),
            added,
        ];
        expected.sort();
        assert_eq!(site.inbox_keys().await, expected);
    }

    fn demo_user(account_id: &str) -> serde_json::Value {
        demo::demo()
            .users
            .into_iter()
            .find(|user| user["accountId"] == account_id)
            .unwrap()
    }

    async fn mention_then_delta(site: &Site, key: &str, author: &str, mentioned: &str) {
        site.run(format!("UPDATE issue_seen SET seen_at = '{TEN}'"))
            .await;
        site.fake
            .set_now(DELTA_AT.parse::<DateTime<Utc>>().unwrap());
        site.fake.add_mention(
            key,
            demo_user(author),
            "Please check the Alpine",
            &demo_user(mentioned),
        );
        let emit: Emit = Arc::new(|_| {});
        run_job(&site.ctx, Job::Delta, &*emit, None).await;
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_comment_mentioning_me_says_mentioned_you_in_the_note() {
        let site = Site::new().await;

        mention_then_delta(
            &site,
            "NW-1049",
            "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03",
            ME_ACCOUNT_ID,
        )
        .await;

        assert_eq!(
            site.note_of("NW-1049").await.as_deref(),
            Some("Mentioned you.")
        );
        assert_eq!(site.note_of("NW-1042").await, None);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_comment_mentioning_someone_else_shows_only_the_new_comment() {
        let site = Site::new().await;

        mention_then_delta(
            &site,
            "NW-1049",
            "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e02",
            "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03",
        )
        .await;

        assert_eq!(
            site.note_of("NW-1049").await.as_deref(),
            Some("1 new comment.")
        );
    }
}

const JONAS_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e02";
const MARTA_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03";

fn person(account_id: &str) -> serde_json::Value {
    demo::demo()
        .users
        .into_iter()
        .find(|user| user["accountId"] == account_id)
        .unwrap()
}

fn done_status() -> serde_json::Value {
    demo::demo()
        .statuses
        .into_iter()
        .find(|status| status["name"] == "Done")
        .unwrap()
}

/// A changelog entry by `author` that changed `field` at `at` (a UTC time like `TEN`).
fn changelog_entry(author: &str, field: &str, at: &str) -> serde_json::Value {
    serde_json::json!({
        "id": format!("{author}/{field}/{at}"),
        "author": person(author),
        "created": at.replace('Z', "+0000"),
        "items": [{
            "field": field,
            "fieldId": field,
            "fieldtype": "jira",
            "from": null,
            "fromString": null,
            "to": null,
            "toString": null
        }]
    })
}

/// The most changelog entries the fake's search embeds per issue.
const EMBEDDED_HISTORIES: usize = 100;

/// `entries` after enough older entries by someone else that the search can't embed the whole changelog, so
/// who made the change is found by the post-sync pass instead.
fn beyond_the_embedded(entries: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    let marta = person(MARTA_ACCOUNT_ID);
    let older = (0..EMBEDDED_HISTORIES).map(|n| {
        serde_json::json!({
            "id": format!("older-{n}"),
            "author": marta,
            "created": LAST_WEEK.replace('Z', "+0000"),
            "items": [{
                "field": "labels",
                "fieldId": "labels",
                "fieldtype": "jira",
                "from": null,
                "fromString": null,
                "to": null,
                "toString": null
            }]
        })
    });
    older.chain(entries).collect()
}

mod own_changes_made_in_jira {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn every_kind_of_change_i_made_in_jira_is_read_after_a_delta() {
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        site.fake
            .edit_issue("NW-1044", |fields| fields["status"] = done_status());
        site.fake.edit_issue("NW-1049", |fields| {
            fields["priority"] = serde_json::json!({ "id": "4", "name": "Low" });
        });
        site.fake.edit_issue("NW-1055", |fields| {
            fields["assignee"] = person(MARTA_ACCOUNT_ID);
        });
        site.fake.edit_issue("NW-1042", |fields| {
            fields["duedate"] = serde_json::json!("2026-11-15");
        });
        let edited = [
            ("NW-1044", "status"),
            ("NW-1049", "priority"),
            ("NW-1055", "assignee"),
            ("NW-1042", "duedate"),
        ];
        for (key, field) in edited {
            site.fake
                .set_changelog(key, vec![changelog_entry(ME_ACCOUNT_ID, field, DELTA_AT)]);
        }

        site.delta_sync().await;

        for (key, field) in edited {
            assert_eq!(
                site.recorded_fields(key).await,
                Vec::<String>::new(),
                "{key} {field}"
            );
            assert_eq!(site.seen_at(key).await, DELTA_AT, "{key} {field}");
        }
        assert_eq!(site.unread_keys().await, Vec::<String>::new());
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_same_change_by_someone_else_is_unread() {
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        site.fake
            .edit_issue("NW-1044", |fields| fields["status"] = done_status());
        site.fake.set_changelog(
            "NW-1044",
            vec![changelog_entry(MARTA_ACCOUNT_ID, "status", DELTA_AT)],
        );

        site.delta_sync().await;

        assert_eq!(
            site.note_of("NW-1044").await.as_deref(),
            Some("Moved to Done.")
        );
        assert_eq!(site.recorded_fields("NW-1044").await, ["status"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn of_a_status_by_me_and_an_assignee_by_someone_else_only_the_assignee_is_left() {
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        site.fake.edit_issue("NW-1044", |fields| {
            fields["status"] = done_status();
            fields["assignee"] = person(JONAS_ACCOUNT_ID);
        });
        site.fake.set_changelog(
            "NW-1044",
            vec![
                changelog_entry(ME_ACCOUNT_ID, "status", DELTA_AT),
                changelog_entry(MARTA_ACCOUNT_ID, "assignee", DELTA_AT),
            ],
        );

        site.delta_sync().await;

        assert_eq!(
            site.note_of("NW-1044").await.as_deref(),
            Some("Reassigned to Jonas Reiter.")
        );
        assert_eq!(site.recorded_fields("NW-1044").await, ["assignee"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_change_stays_when_someone_else_changed_the_field_after_me() {
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        site.fake
            .edit_issue("NW-1044", |fields| fields["status"] = done_status());
        site.fake.set_changelog(
            "NW-1044",
            vec![
                changelog_entry(ME_ACCOUNT_ID, "status", LATER),
                changelog_entry(MARTA_ACCOUNT_ID, "status", AHEAD),
            ],
        );

        site.delta_sync().await;

        assert_eq!(site.recorded_fields("NW-1044").await, ["status"]);
        assert_eq!(site.unread_keys().await, ["NW-1044"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_fetched_changelog_is_kept_as_the_issues_history() {
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        site.fake
            .edit_issue("NW-1044", |fields| fields["status"] = done_status());
        site.fake.set_changelog(
            "NW-1044",
            beyond_the_embedded(vec![changelog_entry(ME_ACCOUNT_ID, "status", DELTA_AT)]),
        );

        site.delta_sync().await;

        let items = issue_activity(site.db(), viewer(), site.id_of("NW-1044").await)
            .await
            .unwrap();
        let history: Vec<(&str, Option<String>)> = items
            .iter()
            .filter_map(|item| match item {
                ActivityItem::Change { field, author, .. } if field == "status" => Some((
                    field.as_str(),
                    author.as_ref().map(|a| a.account_id.clone()),
                )),
                _ => None,
            })
            .collect();
        assert_eq!(history, [("status", Some(ME_ACCOUNT_ID.to_string()))]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_i_created_in_jira_is_read_and_one_by_someone_else_is_unread_as_new() {
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        let mine =
            site.fake
                .add_issue("NW-1055", "NW", "Order confirmation email: shipping notice");
        site.fake
            .edit_issue(&mine, |fields| fields["reporter"] = person(ME_ACCOUNT_ID));
        let theirs = site.fake.add_issue("NW-1055", "NW", "Packing slip layout");
        site.fake.edit_issue(&theirs, |fields| {
            fields["reporter"] = person(MARTA_ACCOUNT_ID);
        });

        site.delta_sync().await;

        assert_eq!(site.note_of(&mine).await, None);
        assert_eq!(site.note_of(&theirs).await.as_deref(), Some("Created."));
        assert_eq!(site.recorded_fields(&mine).await, Vec::<String>::new());
        assert_eq!(site.recorded_fields(&theirs).await, ["created"]);
        assert_eq!(site.seen_at(&mine).await, DELTA_AT);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_comment_by_me_is_read_and_one_by_someone_else_is_a_new_comment() {
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        site.fake.add_comment(
            "NW-1049",
            person(ME_ACCOUNT_ID),
            "Merged into the release branch.",
        );
        site.fake
            .add_comment("NW-1042", person(MARTA_ACCOUNT_ID), "Copy is final.");

        site.delta_sync().await;

        assert_eq!(site.note_of("NW-1049").await, None);
        assert_eq!(
            site.note_of("NW-1042").await.as_deref(),
            Some("1 new comment.")
        );
        assert_eq!(site.recorded_fields("NW-1049").await, Vec::<String>::new());
        assert_eq!(site.recorded_fields("NW-1042").await, ["comment_added"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn comments_by_me_and_by_someone_else_together_are_all_new_comments() {
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        site.fake.add_comment(
            "NW-1049",
            person(ME_ACCOUNT_ID),
            "Merged into the release branch.",
        );
        site.fake
            .add_comment("NW-1049", person(MARTA_ACCOUNT_ID), "Thanks, checking now.");

        site.delta_sync().await;

        assert_eq!(
            site.note_of("NW-1049").await.as_deref(),
            Some("2 new comments.")
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_comment_by_me_that_mentions_someone_records_no_mention() {
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        site.fake.add_mention(
            "NW-1049",
            person(ME_ACCOUNT_ID),
            "Please check the Alpine",
            &person(MARTA_ACCOUNT_ID),
        );

        site.delta_sync().await;

        assert_eq!(site.recorded_fields("NW-1049").await, Vec::<String>::new());
        assert_eq!(site.note_of("NW-1049").await, None);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn without_a_connected_account_nothing_is_dropped() {
        let site = Site::new().await;
        site.ctx
            .settings
            .edit(|settings| settings.connection.account_id = None)
            .await
            .unwrap();
        site.quiet_until_the_delta().await;
        site.fake
            .edit_issue("NW-1044", |fields| fields["status"] = done_status());
        site.fake.set_changelog(
            "NW-1044",
            vec![changelog_entry(ME_ACCOUNT_ID, "status", DELTA_AT)],
        );
        let mine =
            site.fake
                .add_issue("NW-1055", "NW", "Order confirmation email: shipping notice");
        site.fake
            .edit_issue(&mine, |fields| fields["reporter"] = person(ME_ACCOUNT_ID));
        site.fake
            .add_comment("NW-1042", person(ME_ACCOUNT_ID), "Copy is final.");

        site.delta_sync().await;

        assert_eq!(
            site.note_of("NW-1044").await.as_deref(),
            Some("Moved to Done.")
        );
        assert_eq!(site.note_of(&mine).await.as_deref(), Some("Created."));
        assert_eq!(
            site.note_of("NW-1042").await.as_deref(),
            Some("1 new comment.")
        );
        assert_eq!(
            site.number("SELECT COUNT(*) FROM cache.issue WHERE history_synced_at IS NOT NULL")
                .await,
            0
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn only_the_fifty_most_recently_updated_issues_are_checked() {
        const EDITED: i64 = 55;
        const CHECKED: usize = 50;
        let site = Site::new().await;
        site.quiet_until_the_delta().await;
        let keys = site
            .strings(format!(
                "SELECT key FROM cache.issue WHERE gone = 0 ORDER BY id LIMIT {EDITED}"
            ))
            .await;
        assert_eq!(keys.len() as i64, EDITED);
        let first = DELTA_AT.parse::<DateTime<Utc>>().unwrap();
        for (minutes, key) in (0..).zip(&keys) {
            let at = first + chrono::Duration::minutes(minutes);
            site.fake.set_now(at);
            site.fake.edit_issue(key, |fields| {
                fields["duedate"] = serde_json::json!("2030-01-01");
            });
            let stamp = at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
            site.fake.set_changelog(
                key,
                beyond_the_embedded(vec![changelog_entry(ME_ACCOUNT_ID, "duedate", &stamp)]),
            );
        }
        let (oldest, newest) = keys.split_at(keys.len() - CHECKED);

        site.delta_sync().await;

        let mut left_unread = oldest.to_vec();
        left_unread.sort();
        assert_eq!(site.unread_keys().await, left_unread);
        let mut with_history = newest.to_vec();
        with_history.sort();
        assert_eq!(
            site.strings(
                "SELECT key FROM cache.issue WHERE history_synced_at IS NOT NULL ORDER BY key"
                    .into()
            )
            .await,
            with_history
        );
    }
}

mod after_a_restart {
    use super::*;

    const END_OF_DAY: &str = "2026-10-29T00:00:00.000Z";

    /// Everything read after the day's last change.
    async fn all_read(site: &Site) {
        site.run(format!("UPDATE issue_seen SET seen_at = '{END_OF_DAY}'"))
            .await;
    }

    /// Jira stamps the issue `updated` at `at`.
    fn touch(site: &Site, key: &str, at: &str) {
        site.fake.set_now(at.parse::<DateTime<Utc>>().unwrap());
        site.fake.edit_issue(key, |_| {});
    }

    /// NW-1042 seen at 09:00 and updated at 10:00 by someone else, with no other change.
    async fn updated_by_someone_else(site: &Site) {
        all_read(site).await;
        site.set_seen("NW-1042", NINE).await;
        touch(site, "NW-1042", TEN);
        site.fake.set_changelog(
            "NW-1042",
            vec![changelog_entry(MARTA_ACCOUNT_ID, "labels", TEN)],
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_updated_after_it_was_seen_is_unread_as_updated() {
        let site = Site::new().await;
        updated_by_someone_else(&site).await;

        site.refill_cache().await;

        assert_eq!(site.note_of("NW-1042").await.as_deref(), Some("Updated."));
        assert_eq!(site.unread_keys().await, ["NW-1042"]);
        assert_eq!(
            site.detail_since("NW-1042").await,
            Some(Since {
                label: "Since yesterday".into(),
                text: "updated.".into(),
            })
        );
        assert_eq!(site.recorded_fields("NW-1042").await, ["updated"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_not_updated_after_it_was_seen_stays_read() {
        let site = Site::new().await;
        all_read(&site).await;
        for key in ["NW-1044", "NW-1049"] {
            site.set_seen(key, NINE).await;
        }
        touch(&site, "NW-1044", NINE);
        touch(&site, "NW-1049", EIGHT);

        site.refill_cache().await;

        assert_eq!(site.note_of("NW-1044").await, None);
        assert_eq!(site.note_of("NW-1049").await, None);
        assert_eq!(site.unread_keys().await, Vec::<String>::new());
        assert_eq!(
            site.number(
                "SELECT COUNT(*) FROM cache.issue_change WHERE field IN ('updated', 'created')"
            )
            .await,
            0
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_update_after_a_restart_that_only_i_made_is_read() {
        let site = Site::new().await;
        all_read(&site).await;
        site.set_seen("NW-1042", NINE).await;
        touch(&site, "NW-1042", TEN);
        site.fake.set_changelog(
            "NW-1042",
            vec![changelog_entry(ME_ACCOUNT_ID, "labels", TEN)],
        );

        site.refill_cache().await;

        assert_eq!(site.note_of("NW-1042").await, None);
        assert_eq!(site.seen_at("NW-1042").await, TEN);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_update_the_changelog_cannot_tell_the_author_of_stays_unread() {
        let site = Site::new().await;
        all_read(&site).await;
        site.set_seen("NW-1042", NINE).await;
        touch(&site, "NW-1042", TEN);
        site.fake.set_changelog("NW-1042", vec![]);

        site.refill_cache().await;

        assert_eq!(site.note_of("NW-1042").await.as_deref(), Some("Updated."));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_never_seen_in_a_known_project_is_unread_as_new() {
        let site = Site::new().await;
        all_read(&site).await;
        site.forget_seen("NW-1049").await;

        site.refill_cache().await;

        assert_eq!(site.note_of("NW-1049").await.as_deref(), Some("Created."));
        assert_eq!(site.unread_keys().await, ["NW-1049"]);
        assert_eq!(site.recorded_fields("NW-1049").await, ["created"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_i_reported_and_never_saw_in_a_known_project_is_read() {
        let site = Site::new().await;
        all_read(&site).await;
        site.forget_seen("NW-1060").await;

        site.refill_cache().await;

        assert_eq!(site.note_of("NW-1060").await, None);
        assert_eq!(
            site.seen_at("NW-1060").await,
            site.text("SELECT updated_at FROM cache.issue WHERE key = 'NW-1060'".into())
                .await
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_project_with_no_issue_read_yet_marks_everything_read() {
        let site = Site::new().await;
        all_read(&site).await;
        site.run(
            "DELETE FROM issue_seen WHERE issue_id IN
               (SELECT i.id FROM cache.issue i JOIN cache.space p ON p.id = i.project_id WHERE p.key = 'LUM')"
                .into(),
        )
        .await;
        touch(&site, "LUM-318", TEN);

        site.refill_cache().await;

        assert_eq!(site.unread_keys().await, Vec::<String>::new());
        assert_eq!(
            site.number(
                "SELECT COUNT(*) FROM cache.issue i JOIN cache.space p ON p.id = i.project_id
                 LEFT JOIN issue_seen s ON s.issue_id = i.id
                 WHERE p.key = 'LUM' AND s.issue_id IS NULL"
            )
            .await,
            0
        );
        assert_eq!(
            site.number(
                "SELECT COUNT(*) FROM cache.issue_change WHERE field IN ('updated', 'created')"
            )
            .await,
            0
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_activity_leaves_out_the_updated_record() {
        let site = Site::new().await;
        updated_by_someone_else(&site).await;
        site.fake.set_changelog("NW-1042", vec![]);

        site.refill_cache().await;

        assert_eq!(site.recorded_fields("NW-1042").await, ["updated"]);
        let items = issue_activity(site.db(), viewer(), site.id_of("NW-1042").await)
            .await
            .unwrap();
        assert!(!items.is_empty());
        assert!(
            items
                .iter()
                .all(|item| matches!(item, ActivityItem::Comment { .. })),
            "{items:?}"
        );
    }
}
