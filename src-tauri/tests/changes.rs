//! Unseen changes on the demo site synced from the in-process fake Jira into a temp database: the highlight and
//! chip on rows, Recently changed and its badge, Mark seen and Mark all seen, and the Since banner of an issue.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use tempfile::TempDir;

use taskboard_lib::changes::{Since, mark_all_seen, mark_seen};
use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, fetch_versions, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::issues::{ActivityItem, issue_activity, issue_get, issue_related};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::{Kind, Viewer, sidebar_tree};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::{Emit, Job, SyncContext, full_sync, run_job};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::rows::Row;
use taskboard_lib::views::{Narrowing, ViewData, unseen_ids, view_rows, views_of};

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

    async fn list_of_kind(&self, kind: &str) -> i64 {
        self.text(format!(
            "SELECT CAST(id AS TEXT) FROM filter_list WHERE kind = '{kind}'"
        ))
        .await
        .parse()
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

    async fn recent_data(&self) -> ViewData {
        self.data_of(self.list_of_kind("recent").await).await
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
            "INSERT INTO cache.issue_seen (issue_id, seen_at)
             SELECT id, '{at}' FROM cache.issue WHERE key = '{key}'
             ON CONFLICT (issue_id) DO UPDATE SET seen_at = excluded.seen_at"
        ))
        .await;
    }

    async fn forget_seen(&self, key: &str) {
        self.run(format!(
            "DELETE FROM cache.issue_seen WHERE issue_id = (SELECT id FROM cache.issue WHERE key = '{key}')"
        ))
        .await;
    }

    async fn seen_at(&self, key: &str) -> String {
        self.text(format!(
            "SELECT seen_at FROM cache.issue_seen WHERE issue_id = (SELECT id FROM cache.issue WHERE key = '{key}')"
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

    async fn detail_since(&self, key: &str) -> Option<Since> {
        let id = self.id_of(key).await;
        issue_get(self.db(), viewer(), id).await.unwrap().since
    }
}

fn row_of<'a>(data: &'a ViewData, key: &str) -> &'a Row {
    data.rows
        .values()
        .find(|row| row.key == key)
        .unwrap_or_else(|| panic!("no row {key}"))
}

fn sorted_keys(data: &ViewData) -> Vec<String> {
    let mut keys: Vec<String> = data.rows.values().map(|row| row.key.clone()).collect();
    keys.sort();
    keys
}

fn state(row: &Row) -> (bool, Option<&str>) {
    (row.changed, row.change_note.as_deref())
}

mod rows {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_first_sync_leaves_no_issue_changed_and_recently_changed_empty() {
        let site = Site::new().await;

        let nw = site.nw_data().await;
        let recent = site.recent_data().await;

        assert!(nw.rows.values().all(|row| state(row) == (false, None)));
        assert!(recent.rows.is_empty());
        let tree = sidebar_tree(site.db(), viewer()).await.unwrap();
        let badge = tree.home.iter().find(|n| n.kind == Kind::Recent).unwrap();
        assert_eq!(badge.count, Some(0));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_change_after_the_seen_time_highlights_the_row_with_a_chip() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;

        let data = site.nw_data().await;

        assert_eq!(
            state(row_of(&data, "NW-1042")),
            (true, Some("1 new comment"))
        );
        assert_eq!(state(row_of(&data, "NW-1044")), (false, None));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_change_before_the_seen_time_does_not_highlight_the_row() {
        let site = Site::new().await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), NINE)
            .await;
        site.set_seen("NW-1042", TEN).await;

        let data = site.nw_data().await;

        assert_eq!(state(row_of(&data, "NW-1042")), (false, None));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_change_without_any_seen_row_highlights_the_row() {
        let site = Site::new().await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), NINE)
            .await;
        site.forget_seen("NW-1042").await;

        let data = site.nw_data().await;

        assert_eq!(
            state(row_of(&data, "NW-1042")),
            (true, Some("1 new comment"))
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_gone_record_alone_never_highlights_a_row() {
        let site = Site::new().await;
        site.set_seen("NW-1042", EIGHT).await;
        site.record_change("NW-1042", "gone", None, None, NINE)
            .await;

        let data = site.nw_data().await;

        assert_eq!(state(row_of(&data, "NW-1042")), (false, None));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_chip_says_assigned_to_you_for_my_account_and_counts_the_other_changes() {
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

        let data = site.nw_data().await;

        assert_eq!(
            state(row_of(&data, "NW-1042")),
            (true, Some("Assigned to you +1"))
        );
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

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1042")), (false, None));
        assert_eq!(
            state(row_of(&data, "NW-1044")),
            (true, Some("1 new comment"))
        );
        assert_eq!(site.seen_at("NW-1042").await, LATER);
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
        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1042")), (false, None));
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
        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1042")), (false, None));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn mark_all_seen_marks_only_the_unseen_issues_of_that_list() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("NW-1044").await;
        site.comment_after_seen("LUM-318").await;
        let nw = site.nw_list().await;

        mark_all_seen(
            site.db(),
            move |conn| unseen_ids(conn, &viewer(), nw),
            LATER.into(),
        )
        .await
        .unwrap();

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1042")), (false, None));
        assert_eq!(state(row_of(&data, "NW-1044")), (false, None));
        let recent = site.recent_data().await;
        assert_eq!(sorted_keys(&recent), ["LUM-318"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn unseen_ids_of_a_list_are_the_issues_with_unseen_changes_in_it() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("LUM-318").await;
        let nw = site.nw_list().await;

        let ids = site
            .db()
            .read(move |conn| unseen_ids(conn, &viewer(), nw))
            .await
            .unwrap();

        assert_eq!(ids, [site.id_of("NW-1042").await]);
    }
}

mod recently_changed {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn lists_exactly_the_unseen_issues_including_done_ones_and_counts_them_in_the_sidebar() {
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

        let data = site.recent_data().await;

        assert_eq!(sorted_keys(&data), expected);
        let tree = sidebar_tree(site.db(), viewer()).await.unwrap();
        let badge = tree.home.iter().find(|n| n.kind == Kind::Recent).unwrap();
        assert_eq!(badge.count, Some(3));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_updated_today_but_seen_is_not_listed() {
        let site = Site::new().await;
        site.record_change("NW-1042", "comment_added", Some("4"), Some("5"), NINE)
            .await;
        site.set_seen("NW-1042", TEN).await;

        let data = site.recent_data().await;

        assert!(data.rows.is_empty());
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn marking_an_issue_seen_removes_it_from_the_list_and_the_badge() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("NW-1044").await;

        site.mark(&["NW-1042"], LATER).await;

        let data = site.recent_data().await;
        assert_eq!(sorted_keys(&data), ["NW-1044"]);
        let tree = sidebar_tree(site.db(), viewer()).await.unwrap();
        let badge = tree.home.iter().find(|n| n.kind == Kind::Recent).unwrap();
        assert_eq!(badge.count, Some(1));
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

    /// `(at, fresh)` of the field changes, oldest first.
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
    async fn what_the_site_did_shows_as_chips_on_the_edited_issues() {
        let site = Site::new().await;
        site.run(format!("UPDATE cache.issue_seen SET seen_at = '{TEN}'"))
            .await;
        site.fake
            .set_now(DELTA_AT.parse::<DateTime<Utc>>().unwrap());
        site.fake.add_comment(
            "NW-1042",
            demo::demo().users[0].clone(),
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

        let data = site.nw_data().await;
        assert_eq!(
            state(row_of(&data, "NW-1042")),
            (true, Some("1 new comment"))
        );
        assert_eq!(
            state(row_of(&data, "NW-1044")),
            (true, Some("Moved to Done"))
        );
        assert_eq!(
            state(row_of(&data, "NW-1049")),
            (true, Some("Priority lowered"))
        );
        assert_eq!(state(row_of(&data, &added)), (true, Some("New")));
        let recent = site.recent_data().await;
        let mut expected = vec![
            "NW-1042".to_string(),
            "NW-1044".to_string(),
            "NW-1049".to_string(),
            added,
        ];
        expected.sort();
        assert_eq!(sorted_keys(&recent), expected);
    }
}
