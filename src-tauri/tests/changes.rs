//! Unseen changes on the demo site synced from the in-process fake Jira into a temp database: the highlight and
//! chip on rows, Unread and its badge, Mark seen and Mark all seen, and the Since banner of an issue.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use tempfile::TempDir;

use taskboard_lib::changes::{Since, mark_all_seen, mark_seen, mark_unread};
use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, fetch_versions, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::error::ErrorKind;
use taskboard_lib::filter::{Condition, Field, Filter};
use taskboard_lib::issues::{ActivityItem, issue_activity, issue_get, issue_related};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::{Kind, Viewer, list_create, sidebar_tree};
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

    async fn detail_unread(&self, key: &str) -> bool {
        let id = self.id_of(key).await;
        issue_get(self.db(), viewer(), id).await.unwrap().unread
    }

    async fn detail_since(&self, key: &str) -> Option<Since> {
        let id = self.id_of(key).await;
        issue_get(self.db(), viewer(), id).await.unwrap().since
    }

    async fn badge(&self, kind: Kind) -> Option<i64> {
        let tree = sidebar_tree(self.db(), viewer()).await.unwrap();
        tree.home.iter().find(|n| n.kind == kind).unwrap().count
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
        let recent = site.list_of_kind("recent").await;
        let view = site.view_in(recent, 0, SubtaskMode::Hidden).await;

        let everything = site
            .unseen_keys(recent, at(view, Narrowing::default()))
            .await;
        site.filter_view(view, of_type("Story")).await;
        let stories = site
            .unseen_keys(recent, at(view, Narrowing::default()))
            .await;
        view_set_subtasks(site.db(), view, SubtaskMode::Hierarchy)
            .await
            .unwrap();
        let with_descendants = site
            .unseen_keys(recent, at(view, Narrowing::default()))
            .await;

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

mod recently_changed {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_unread_subtask_type_issue_is_listed_and_counted() {
        let site = Site::new().await;
        site.comment_after_seen("NW-1059").await;

        let data = site.recent_data().await;

        assert_eq!(sorted_keys(&data), ["NW-1059"]);
        assert_eq!(site.badge(Kind::Recent).await, Some(1));
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

mod marking_unread {
    use super::*;

    async fn unread_flag(site: &Site, key: &str) -> bool {
        site.detail_unread(key).await
    }

    async fn badge(site: &Site) -> Option<i64> {
        let tree = sidebar_tree(site.db(), viewer()).await.unwrap();
        tree.home
            .iter()
            .find(|n| n.kind == Kind::Recent)
            .unwrap()
            .count
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
        let narrowing = Narrowing {
            read_here,
            ..Narrowing::default()
        };
        view_rows(site.db(), viewer(), list, view, narrowing)
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

        let data = data_keeping(&site, "recent", &["NW-1042"]).await;

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

        let data = data_keeping(&site, "recent", &[]).await;

        assert!(data.rows.is_empty());
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn other_levels_ignore_read_here() {
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
