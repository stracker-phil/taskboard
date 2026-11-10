//! Unseen changes on the demo site synced from the in-process fake Jira into a temp database: the highlight and
//! chip on rows, Unread and its badge, Mark seen and Mark all seen, and the Since banner of an issue.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use tempfile::TempDir;

use taskboard_lib::changes::{Since, UNSEEN, mark_all_seen, mark_seen, mark_unread};
use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, fetch_versions, refresh_projects, save, track_project,
};
use taskboard_lib::db::{Changes, Db};
use taskboard_lib::error::ErrorKind;
use taskboard_lib::filter::{Condition, Field, Filter};
use taskboard_lib::issues::{ActivityItem, issue_activity, issue_get, issue_related};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::{Kind, SidebarTree, Viewer, list_create, sidebar_tree};
use taskboard_lib::pins::{PinDuration, PinKind, pin_create};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::{Emit, Job, SyncContext, full_sync, run_job};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::rows::Row;
use taskboard_lib::views::{
    Narrowing, SubtaskMode, ViewAt, ViewData, unseen_ids, view_rows, view_set_filter,
    view_set_subtasks, views_of,
};

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

    /// Everything with Unread only on.
    async fn recent_data(&self) -> ViewData {
        let list = self.list_of_kind("everything").await;
        let view = views_of(self.db(), list).await.unwrap()[0].id;
        view_rows(self.db(), viewer(), list, view, unread_only(&[]))
            .await
            .unwrap()
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

    /// The unread count of Everything in the sidebar.
    async fn badge(&self) -> Option<i64> {
        self.tree(true).await.everything.count
    }

    async fn tree(&self, unread: bool) -> SidebarTree {
        sidebar_tree(self.db(), viewer(), unread).await.unwrap()
    }

    /// The view at `index` of the list, in the given subtask mode.
    async fn view_in(&self, list: i64, index: usize, mode: SubtaskMode) -> i64 {
        let view = views_of(self.db(), list).await.unwrap()[index].id;
        view_set_subtasks(self.db(), view, mode).await.unwrap();
        view
    }

    async fn filter_view(&self, view: i64, conditions: Vec<Condition>) {
        view_set_filter(self.db(), viewer(), view, Filter { conditions })
            .await
            .unwrap();
    }

    /// The keys Mark all as read would mark, sorted.
    async fn unseen_keys(&self, list: i64, view: Option<ViewAt>) -> Vec<String> {
        let ids = self
            .db()
            .read(move |conn| unseen_ids(conn, &viewer(), list, view.as_ref()))
            .await
            .unwrap();
        let mut keys = Vec::new();
        for id in ids {
            keys.push(
                self.text(format!("SELECT key FROM cache.issue WHERE id = '{id}'"))
                    .await,
            );
        }
        keys.sort();
        keys
    }

    /// A story, its unread subtask and an unread task outside the story's filter.
    async fn unread_story_subtask_and_task(&self) {
        self.comment_after_seen("NW-1042").await;
        self.comment_after_seen("NW-1059").await;
        self.comment_after_seen("NW-1044").await;
    }
}

fn at(view_id: i64, narrowing: Narrowing) -> Option<ViewAt> {
    Some(ViewAt { view_id, narrowing })
}

/// Unread only on, keeping the issues with these ids that were read since the owner came here.
fn unread_only(read_here: &[String]) -> Narrowing {
    Narrowing {
        unread: true,
        read_here: read_here.to_vec(),
        ..Narrowing::default()
    }
}

fn of_type(name: &str) -> Vec<Condition> {
    vec![Condition::is(Field::Type, name)]
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
        assert_eq!(site.badge().await, Some(0));
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
    async fn mark_all_seen_announces_only_the_issues_that_were_unseen() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.set_seen("NW-1044", LATER).await;
        let ids = vec![site.id_of("NW-1042").await, site.id_of("NW-1044").await];
        let announced = watch(site.db());

        mark_all_seen(site.db(), move |_| Ok(ids), LATER.into())
            .await
            .unwrap();

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

        let mut marked = mark_all_seen(
            site.db(),
            move |conn| unseen_ids(conn, &viewer(), nw, None),
            LATER.into(),
        )
        .await
        .unwrap();

        let mut expected = vec![site.id_of("NW-1042").await, site.id_of("NW-1044").await];
        expected.sort();
        marked.sort();
        assert_eq!(marked, expected);
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
            .read(move |conn| unseen_ids(conn, &viewer(), nw, None))
            .await
            .unwrap();

        assert_eq!(ids, [site.id_of("NW-1042").await]);
    }
}

mod mark_all_as_read_per_view {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn without_a_view_every_unread_issue_of_the_list_counts_subtasks_too() {
        let site = Site::new().await;
        site.unread_story_subtask_and_task().await;
        let nw = site.nw_list().await;

        let keys = site.unseen_keys(nw, None).await;

        assert_eq!(keys, ["NW-1042", "NW-1044", "NW-1059"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_view_with_its_own_filter_follows_its_subtask_mode() {
        let site = Site::new().await;
        site.unread_story_subtask_and_task().await;
        let nw = site.nw_list().await;
        let view = site.view_in(nw, 0, SubtaskMode::Hidden).await;
        site.filter_view(view, of_type("Story")).await;

        for (mode, expected) in [
            (SubtaskMode::Hidden, vec!["NW-1042"]),
            (SubtaskMode::Hierarchy, vec!["NW-1042", "NW-1059"]),
            (SubtaskMode::List, vec!["NW-1042", "NW-1059"]),
        ] {
            view_set_subtasks(site.db(), view, mode).await.unwrap();

            let keys = site.unseen_keys(nw, at(view, Narrowing::default())).await;

            assert_eq!(keys, expected, "{mode:?}");
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_board_view_counts_as_hidden_whatever_its_subtask_mode() {
        let site = Site::new().await;
        site.unread_story_subtask_and_task().await;
        let nw = site.nw_list().await;
        let board = site.view_in(nw, 1, SubtaskMode::Hierarchy).await;
        site.filter_view(board, of_type("Story")).await;

        let keys = site.unseen_keys(nw, at(board, Narrowing::default())).await;

        assert_eq!(keys, ["NW-1042"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_view_draft_stands_in_for_the_views_own_filter() {
        let site = Site::new().await;
        site.unread_story_subtask_and_task().await;
        let nw = site.nw_list().await;
        let view = site.view_in(nw, 0, SubtaskMode::Hidden).await;
        site.filter_view(view, of_type("Story")).await;
        let narrowing = Narrowing {
            view_draft: Some(of_type("Task")),
            ..Narrowing::default()
        };

        let keys = site.unseen_keys(nw, at(view, narrowing)).await;

        assert_eq!(keys, ["NW-1044"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_list_draft_stands_in_for_the_filter_lists_own_filter() {
        let site = Site::new().await;
        site.unread_story_subtask_and_task().await;
        let nw = site.nw_list().await;
        let stories = list_create(
            site.db(),
            viewer(),
            nw,
            "Stories".into(),
            Some(Filter {
                conditions: of_type("Story"),
            }),
        )
        .await
        .unwrap();
        let view = site.view_in(stories, 0, SubtaskMode::Hidden).await;
        let narrowing = Narrowing {
            list_draft: Some(of_type("Task")),
            ..Narrowing::default()
        };

        let saved = site
            .unseen_keys(stories, at(view, Narrowing::default()))
            .await;
        let drafted = site.unseen_keys(stories, at(view, narrowing)).await;

        assert_eq!(saved, ["NW-1042"]);
        assert_eq!(drafted, ["NW-1044"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_me_chip_and_the_text_narrow_the_marked_issues() {
        let site = Site::new().await;
        site.unread_story_subtask_and_task().await;
        let nw = site.nw_list().await;
        let view = site.view_in(nw, 0, SubtaskMode::Hidden).await;
        let mine = Narrowing {
            me: true,
            ..Narrowing::default()
        };
        let gallery = Narrowing {
            text: "gallery".into(),
            ..Narrowing::default()
        };

        let assigned_to_me = site.unseen_keys(nw, at(view, mine)).await;
        let matching_the_text = site.unseen_keys(nw, at(view, gallery)).await;

        assert_eq!(assigned_to_me, ["NW-1042", "NW-1059"]);
        assert_eq!(matching_the_text, ["NW-1044"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn on_unread_a_view_with_its_own_filter_leaves_out_the_unread_issues_outside_it() {
        let site = Site::new().await;
        site.unread_story_subtask_and_task().await;
        let all = site.list_of_kind("everything").await;
        let view = site.view_in(all, 0, SubtaskMode::Hidden).await;

        let everything = site.unseen_keys(all, at(view, unread_only(&[]))).await;
        site.filter_view(view, of_type("Story")).await;
        let stories = site.unseen_keys(all, at(view, unread_only(&[]))).await;
        view_set_subtasks(site.db(), view, SubtaskMode::Hierarchy)
            .await
            .unwrap();
        let with_descendants = site.unseen_keys(all, at(view, unread_only(&[]))).await;

        assert_eq!(everything, ["NW-1042", "NW-1044", "NW-1059"]);
        assert_eq!(stories, ["NW-1042"]);
        assert_eq!(with_descendants, ["NW-1042", "NW-1059"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn marking_all_as_read_in_a_view_leaves_the_other_unread_issues_unread() {
        let site = Site::new().await;
        site.unread_story_subtask_and_task().await;
        let nw = site.nw_list().await;
        let view = site.view_in(nw, 0, SubtaskMode::Hidden).await;
        site.filter_view(view, of_type("Story")).await;

        let marked = mark_all_seen(
            site.db(),
            move |conn| unseen_ids(conn, &viewer(), nw, at(view, Narrowing::default()).as_ref()),
            LATER.into(),
        )
        .await
        .unwrap();

        assert_eq!(marked, [site.id_of("NW-1042").await]);
        assert_eq!(
            sorted_keys(&site.recent_data().await),
            ["NW-1044", "NW-1059"]
        );
    }
}

mod unread_only_mode {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_unread_subtask_type_issue_is_listed_and_counted() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1059").await;

        let data = site.recent_data().await;

        assert_eq!(sorted_keys(&data), ["NW-1059"]);
        assert_eq!(site.badge().await, Some(1));
    }

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
        assert_eq!(site.badge().await, Some(3));
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
        assert_eq!(site.badge().await, Some(1));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn unread_counts_in_the_sidebar_cover_every_list_open_or_done_and_leave_the_bookmark_count()
     {
        let site = Site::new().await;
        let done = site
            .text(format!(
                "SELECT key FROM cache.issue
                 WHERE project_id = '{NW}' AND status_category = 'done' AND gone = 0 ORDER BY key LIMIT 1"
            ))
            .await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen(&done).await;
        site.comment_after_seen("LUM-318").await;
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

        let unread = site.tree(true).await;
        let open = site.tree(false).await;

        let nw_unread = |tree: &SidebarTree| {
            tree.spaces
                .iter()
                .find(|s| s.key == "NW")
                .unwrap()
                .open_count
        };
        let lum_unread = unread.spaces.iter().find(|s| s.key == "LUM").unwrap();
        assert_eq!(unread.everything.count, Some(3));
        assert_eq!(nw_unread(&unread), 2);
        assert_eq!(lum_unread.open_count, 1);
        assert!(nw_unread(&open) > nw_unread(&unread));
        let bookmarks = |tree: &SidebarTree| {
            tree.home
                .iter()
                .find(|n| n.kind == Kind::Pinned)
                .unwrap()
                .count
        };
        assert_eq!(bookmarks(&unread), Some(1));
        assert_eq!(bookmarks(&open), Some(1));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_view_shows_the_unseen_issues_and_those_read_here_in_any_list() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("NW-1044").await;
        site.comment_after_seen("LUM-318").await;
        site.mark(&["NW-1042"], LATER).await;
        let read_here = [site.id_of("NW-1042").await];
        let everything = site.list_of_kind("everything").await;
        let nw = site.nw_list().await;

        for (list, kept, expected) in [
            (
                everything,
                &read_here[..],
                vec!["LUM-318", "NW-1042", "NW-1044"],
            ),
            (everything, &[][..], vec!["LUM-318", "NW-1044"]),
            (nw, &read_here[..], vec!["NW-1042", "NW-1044"]),
            (nw, &[][..], vec!["NW-1044"]),
        ] {
            let view = views_of(site.db(), list).await.unwrap()[0].id;
            let data = view_rows(site.db(), viewer(), list, view, unread_only(kept))
                .await
                .unwrap();

            assert_eq!(sorted_keys(&data), expected, "list {list}, kept {kept:?}");
        }
    }
}

mod marking_unread {
    use super::*;

    async fn unread_flag(site: &Site, key: &str) -> bool {
        site.detail_unread(key).await
    }

    async fn badge(site: &Site) -> Option<i64> {
        site.badge().await
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_without_any_change_becomes_unread_everywhere() {
        let site = Site::new().await;
        let id = site.id_of("NW-1042").await;
        assert!(!unread_flag(&site, "NW-1042").await);

        mark_unread(site.db(), id, None).await.unwrap();

        let nw = site.nw_data().await;
        assert!(row_of(&nw, "NW-1042").changed);
        assert!(!row_of(&nw, "NW-1044").changed);
        assert_eq!(sorted_keys(&site.recent_data().await), ["NW-1042"]);
        assert_eq!(badge(&site).await, Some(1));
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

        assert_eq!(sorted_keys(&site.recent_data().await), ["NW-1042"]);
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
        assert!(site.recent_data().await.rows.is_empty());
        assert!(!row_of(&site.nw_data().await, "NW-1042").changed);
        assert_eq!(badge(&site).await, Some(0));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn mark_all_seen_clears_unread_flags_of_the_list() {
        let site = Site::new().await;
        let id = site.id_of("NW-1042").await;
        mark_unread(site.db(), id.clone(), None).await.unwrap();
        let nw = site.nw_list().await;

        let marked = mark_all_seen(
            site.db(),
            move |conn| unseen_ids(conn, &viewer(), nw, None),
            LATER.into(),
        )
        .await
        .unwrap();

        assert_eq!(marked, [id]);
        assert!(site.recent_data().await.rows.is_empty());
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
        assert_eq!(sorted_keys(&site.recent_data().await), ["NW-1042"]);
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

mod read_here {
    use super::*;

    async fn data_keeping(site: &Site, kind: &str, keys: &[&str]) -> ViewData {
        let list = site.list_of_kind(kind).await;
        let view = views_of(site.db(), list).await.unwrap()[0].id;
        let mut read_here = Vec::new();
        for key in keys {
            read_here.push(site.id_of(key).await);
        }
        view_rows(site.db(), viewer(), list, view, unread_only(&read_here))
            .await
            .unwrap()
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn unread_keeps_listing_an_issue_read_here_as_not_changed() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.comment_after_seen("NW-1044").await;
        site.mark(&["NW-1042"], LATER).await;

        let data = data_keeping(&site, "everything", &["NW-1042"]).await;

        assert_eq!(sorted_keys(&data), ["NW-1042", "NW-1044"]);
        assert!(!row_of(&data, "NW-1042").changed);
        assert!(row_of(&data, "NW-1044").changed);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn unread_without_read_here_drops_the_read_issue() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1042").await;
        site.mark(&["NW-1042"], LATER).await;

        let data = data_keeping(&site, "everything", &[]).await;

        assert!(data.rows.is_empty());
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn read_here_never_adds_issues_outside_the_list() {
        let site = Site::new().await;

        let bookmarks = data_keeping(&site, "pinned", &["NW-1042"]).await;

        assert!(bookmarks.rows.is_empty());
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
    async fn what_the_site_did_shows_as_chips_on_the_edited_issues() {
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
    async fn a_comment_mentioning_me_shows_mentioned_you_first_on_the_row() {
        let site = Site::new().await;

        mention_then_delta(
            &site,
            "NW-1049",
            "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03",
            ME_ACCOUNT_ID,
        )
        .await;

        let data = site.nw_data().await;
        assert_eq!(
            state(row_of(&data, "NW-1049")),
            (true, Some("Mentioned you"))
        );
        assert_eq!(state(row_of(&data, "NW-1042")), (false, None));
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

        let data = site.nw_data().await;
        assert_eq!(
            state(row_of(&data, "NW-1049")),
            (true, Some("1 new comment"))
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

        let data = site.nw_data().await;
        assert_eq!(
            state(row_of(&data, "NW-1044")),
            (true, Some("Moved to Done"))
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

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1044")), (true, Some("Reassigned")));
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
            vec![changelog_entry(ME_ACCOUNT_ID, "status", DELTA_AT)],
        );

        site.delta_sync().await;

        let items = issue_activity(site.db(), viewer(), site.id_of("NW-1044").await)
            .await
            .unwrap();
        let history: Vec<(&str, Option<String>)> = items
            .iter()
            .filter_map(|item| match item {
                ActivityItem::Change { field, author, .. } => Some((
                    field.as_str(),
                    author.as_ref().map(|a| a.account_id.clone()),
                )),
                ActivityItem::Comment { .. } => None,
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

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, &mine)), (false, None));
        assert_eq!(state(row_of(&data, &theirs)), (true, Some("New")));
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

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1049")), (false, None));
        assert_eq!(
            state(row_of(&data, "NW-1042")),
            (true, Some("1 new comment"))
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

        let data = site.nw_data().await;
        assert_eq!(
            state(row_of(&data, "NW-1049")),
            (true, Some("2 new comments"))
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
        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1049")), (false, None));
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

        let data = site.nw_data().await;
        assert_eq!(
            state(row_of(&data, "NW-1044")),
            (true, Some("Moved to Done"))
        );
        assert_eq!(state(row_of(&data, &mine)), (true, Some("New")));
        assert_eq!(
            state(row_of(&data, "NW-1042")),
            (true, Some("1 new comment"))
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
            site.fake
                .set_changelog(key, vec![changelog_entry(ME_ACCOUNT_ID, "duedate", &stamp)]);
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

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1042")), (true, Some("Updated")));
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

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1044")), (false, None));
        assert_eq!(state(row_of(&data, "NW-1049")), (false, None));
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

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1042")), (false, None));
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

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1042")), (true, Some("Updated")));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_issue_never_seen_in_a_known_project_is_unread_as_new() {
        let site = Site::new().await;
        all_read(&site).await;
        site.forget_seen("NW-1049").await;

        site.refill_cache().await;

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1049")), (true, Some("New")));
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

        let data = site.nw_data().await;
        assert_eq!(state(row_of(&data, "NW-1060")), (false, None));
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
