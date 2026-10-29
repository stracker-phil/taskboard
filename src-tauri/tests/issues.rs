//! An issue as the peek shows it, on the demo site synced from the in-process fake Jira into a temp database.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, NaiveDate, Utc};
use tempfile::TempDir;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, fetch_versions, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::error::{AppResult, ErrorKind};
use taskboard_lib::issues::{
    ActivityItem, ChangeValue, CustomField, IssueDetail, IssueLink, RelatedIssue, WebLink,
    WebLinkKind, issue_activity, issue_forget, issue_get, issue_related, issue_sync_comments,
    issue_sync_web_links, issue_web_links,
};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::Viewer;
use taskboard_lib::space_config::{FieldVisibility, space_set_field};
use taskboard_lib::status_style::{Style, status_style_set};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::{Emit, SyncContext, full_sync, source_for};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL, REVIEWER};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::rows::{self, Cell, EpicChip, Person};
use taskboard_lib::views::{view_set_columns, views_of};

const LATER_UTC: &str = "2026-10-28T13:00:00.000Z";
const NW_PROJECT_ID: &str = "10001";
const ME_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const MARTA_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03";
const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];

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
    clock_minutes: AtomicI64,
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
            clock_minutes: AtomicI64::new(0),
        }
    }

    fn db(&self) -> &Db {
        &self.ctx.db
    }

    async fn id_of(&self, key: &str) -> String {
        let key = key.to_string();
        self.db()
            .read(move |conn| {
                Ok(
                    conn.query_row("SELECT id FROM cache.issue WHERE key = ?1", [&key], |r| {
                        r.get::<_, String>(0)
                    })?,
                )
            })
            .await
            .unwrap()
    }

    async fn status_of(&self, key: &str) -> String {
        let key = key.to_string();
        self.db()
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT status_id FROM cache.issue WHERE key = ?1",
                    [&key],
                    |r| r.get::<_, String>(0),
                )?)
            })
            .await
            .unwrap()
    }

    async fn detail(&self, key: &str) -> IssueDetail {
        let id = self.id_of(key).await;
        issue_get(self.db(), viewer(), id).await.unwrap()
    }

    async fn activity(&self, key: &str) -> Vec<ActivityItem> {
        let id = self.id_of(key).await;
        issue_activity(self.db(), viewer(), id).await.unwrap()
    }

    async fn related(&self, key: &str) -> taskboard_lib::issues::Related {
        let id = self.id_of(key).await;
        issue_related(self.db(), viewer(), id).await.unwrap()
    }

    async fn sync_comments(&self, key: &str) -> bool {
        let id = self.id_of(key).await;
        let connection = self.ctx.settings.get().connection;
        let source = source_for(&self.ctx.access, &connection).unwrap();
        issue_sync_comments(self.db(), source.as_ref(), id)
            .await
            .unwrap()
    }

    /// Syncs at `minutes` after the demo's now.
    async fn try_sync_web_links_at(&self, key: &str, minutes: i64) -> AppResult<bool> {
        let id = self.id_of(key).await;
        let connection = self.ctx.settings.get().connection;
        let source = source_for(&self.ctx.access, &connection).unwrap();
        let now = viewer().now + chrono::Duration::minutes(minutes);
        issue_sync_web_links(self.db(), source.as_ref(), id, now).await
    }

    /// Each call happens 6 minutes after the previous one, so none is answered from the freshness window.
    async fn try_sync_web_links(&self, key: &str) -> AppResult<bool> {
        let minutes = self.clock_minutes.fetch_add(6, Ordering::SeqCst);
        self.try_sync_web_links_at(key, minutes).await
    }

    async fn web_link_fetches(&self, key: &str) -> usize {
        let path = format!("/rest/api/3/issue/{}/remotelink", self.id_of(key).await);
        self.fake.paths().iter().filter(|p| **p == path).count()
    }

    async fn sync_web_links(&self, key: &str) -> bool {
        self.try_sync_web_links(key).await.unwrap()
    }

    async fn web_links(&self, key: &str) -> Vec<WebLink> {
        let id = self.id_of(key).await;
        issue_web_links(self.db(), id).await.unwrap()
    }

    async fn resync(&self) {
        let emit: Emit = Arc::new(|_| {});
        full_sync(&self.ctx, &*emit).await.unwrap();
    }

    async fn execute(&self, sql: &'static str) {
        self.db()
            .write(move |tx, _| {
                tx.execute(sql, [])?;
                Ok(())
            })
            .await
            .unwrap();
    }

    fn comment_fetches(&self, key: &str) -> usize {
        let path = format!("/rest/api/3/issue/{key}/comment");
        self.fake.paths().iter().filter(|p| **p == path).count()
    }

    async fn comment_count_in_cache(&self, key: &str) -> i64 {
        let key = key.to_string();
        self.db()
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM cache.comment c JOIN cache.issue i ON i.id = c.issue_id
                     WHERE i.key = ?1",
                    [&key],
                    |r| r.get::<_, i64>(0),
                )?)
            })
            .await
            .unwrap()
    }

    async fn count_rows(&self, sql: String) -> i64 {
        self.db()
            .read(move |conn| Ok(conn.query_row(&sql, [], |r| r.get::<_, i64>(0))?))
            .await
            .unwrap()
    }

    /// Rows of the issue in the tables `issue_forget` clears, as
    /// `(issue, comments, changes, seen, search)`.
    async fn rows_of(&self, id: &str) -> (i64, i64, i64, i64, i64) {
        (
            self.count_rows(format!(
                "SELECT COUNT(*) FROM cache.issue WHERE id = '{id}'"
            ))
            .await,
            self.count_rows(format!(
                "SELECT COUNT(*) FROM cache.comment WHERE issue_id = '{id}'"
            ))
            .await,
            self.count_rows(format!(
                "SELECT COUNT(*) FROM cache.issue_change WHERE issue_id = '{id}'"
            ))
            .await,
            self.count_rows(format!(
                "SELECT COUNT(*) FROM cache.issue_seen WHERE issue_id = '{id}'"
            ))
            .await,
            self.count_rows(format!(
                "SELECT COUNT(*) FROM cache.issue_fts WHERE rowid = {id}"
            ))
            .await,
        )
    }

    async fn show_reviewer_column(&self) {
        let space = self
            .db()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT id FROM filter_list WHERE kind = 'space' AND project_id = ?1",
                    [NW_PROJECT_ID],
                    |r| r.get::<_, i64>(0),
                )?)
            })
            .await
            .unwrap();
        let view = views_of(self.db(), space).await.unwrap()[0].id;
        view_set_columns(self.db(), view, vec!["assignee".into(), REVIEWER.into()])
            .await
            .unwrap();
        self.resync().await;
    }
}

fn alex() -> Person {
    rows::person(ME_ACCOUNT_ID, "Alex Morgan", Some(ME_ACCOUNT_ID))
}

fn marta() -> Person {
    rows::person(MARTA_ACCOUNT_ID, "Marta Klein", Some(ME_ACCOUNT_ID))
}

fn keys(issues: &[RelatedIssue]) -> Vec<&str> {
    issues.iter().map(|i| i.key.as_str()).collect()
}

fn comment_times(items: &[ActivityItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            ActivityItem::Comment { at, .. } => Some(at.as_str()),
            ActivityItem::Change { .. } => None,
        })
        .collect()
}

fn done_status() -> serde_json::Value {
    serde_json::json!({
        "id": "10001",
        "name": "Done",
        "statusCategory": { "id": 3, "key": "done", "name": "Done" },
    })
}

fn later() -> DateTime<Utc> {
    DEMO_NOW.parse::<DateTime<Utc>>().unwrap() + chrono::Duration::hours(2)
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_issue_shows_its_key_type_summary_and_status() {
    let site = Site::new().await;

    let detail = site.detail("NW-1042").await;

    assert_eq!(detail.id, site.id_of("NW-1042").await);
    assert_eq!(detail.key, "NW-1042");
    assert_eq!(detail.issue_type, "Story");
    assert_eq!(
        detail.summary,
        "Checkout: address validation for DACH postcodes"
    );
    assert_eq!(detail.status_name, "In Progress");
    assert_eq!(detail.style, Style::Progress);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn statuses_in_review_and_done_get_their_styles() {
    let site = Site::new().await;

    for (key, status_name, style) in [
        ("NW-1038", "In Review", Style::Review),
        ("NW-1021", "Done", Style::Done),
    ] {
        let detail = site.detail(key).await;

        assert_eq!(detail.status_name, status_name, "{key}");
        assert_eq!(detail.style, style, "{key}");
    }
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_chosen_status_style_wins_over_the_detected_one() {
    let site = Site::new().await;
    let status = site.status_of("NW-1038").await;
    status_style_set(
        site.db(),
        NW_PROJECT_ID.into(),
        status.clone(),
        Some(Style::Blocked),
    )
    .await
    .unwrap();

    assert_eq!(site.detail("NW-1038").await.style, Style::Blocked);

    status_style_set(site.db(), NW_PROJECT_ID.into(), status, None)
        .await
        .unwrap();

    assert_eq!(site.detail("NW-1038").await.style, Style::Review);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_child_of_an_epic_shows_the_epic_chip() {
    let site = Site::new().await;

    let detail = site.detail("NW-1042").await;

    assert_eq!(
        detail.epic,
        Some(EpicChip {
            id: site.id_of("NW-1020").await,
            name: "Checkout".into(),
            color: 1,
        })
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_epic_has_no_epic_chip_and_is_marked_as_epic() {
    let site = Site::new().await;

    let epic = site.detail("NW-1020").await;
    let story = site.detail("NW-1042").await;

    assert!(epic.is_epic);
    assert_eq!(epic.issue_type, "Epic");
    assert_eq!(epic.epic, None);
    assert!(!story.is_epic);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn releases_are_listed_by_version_name() {
    let site = Site::new().await;

    assert_eq!(site.detail("NW-1042").await.releases, ["2.4.0"]);
    assert_eq!(site.detail("NW-1049").await.releases, ["2.5.0"]);
    assert!(site.detail("NW-1055").await.releases.is_empty());
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_demo_account_is_the_assignee_with_colour_one() {
    let site = Site::new().await;

    let detail = site.detail("NW-1042").await;

    assert_eq!(detail.assignees.len(), 1);
    let assignee = &detail.assignees[0];
    assert_eq!(assignee.account_id, ME_ACCOUNT_ID);
    assert_eq!(assignee.name, "Alex Morgan");
    assert_eq!(assignee.initials, "AM");
    assert_eq!(assignee.color, 1);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_reporter_is_a_person_other_than_me() {
    let site = Site::new().await;

    let reporter = site.detail("NW-1038").await.reporter.expect("a reporter");

    assert_eq!(
        reporter,
        rows::person(MARTA_ACCOUNT_ID, "Marta Klein", Some(ME_ACCOUNT_ID))
    );
    assert_eq!(reporter.initials, "MK");
    assert_ne!(reporter.color, 1);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn labels_come_with_their_tint() {
    let site = Site::new().await;

    let detail = site.detail("NW-1042").await;

    assert_eq!(detail.labels, [rows::label("frontend")]);
    assert!(site.detail("NW-1059").await.labels.is_empty());
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_due_date_is_formatted_like_the_list() {
    let site = Site::new().await;

    assert_eq!(site.detail("NW-1042").await.due.as_deref(), Some("Oct 30"));
    assert_eq!(site.detail("NW-1049").await.due, None);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_description_arrives_as_rendered_html() {
    let site = Site::new().await;

    let html = site
        .detail("NW-1042")
        .await
        .description_html
        .expect("a description");

    assert!(html.contains('<'), "{html}");
    assert!(
        html.contains("Validate the postcode against the selected country"),
        "{html}"
    );
    assert_eq!(site.detail("NW-1049").await.description_html, None);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_comment_count_matches_the_comments_of_the_issue() {
    let site = Site::new().await;

    assert_eq!(site.detail("NW-1042").await.comment_count, 4);
    assert_eq!(site.detail("NW-1049").await.comment_count, 0);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_issue_marked_gone_says_so() {
    let site = Site::new().await;
    assert!(!site.detail("NW-1042").await.gone);
    site.db()
        .write(|tx, _| {
            tx.execute("UPDATE cache.issue SET gone = 1 WHERE key = 'NW-1042'", [])?;
            Ok(())
        })
        .await
        .unwrap();

    assert!(site.detail("NW-1042").await.gone);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_detail_serializes_with_camel_case_keys() {
    let site = Site::new().await;

    let value = serde_json::to_value(site.detail("NW-1042").await).unwrap();

    for key in [
        "issueType",
        "statusName",
        "descriptionHtml",
        "commentCount",
        "isEpic",
        "createdAt",
        "updatedAt",
        "syncedAt",
    ] {
        assert!(value.get(key).is_some(), "{key}");
    }
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_detail_carries_the_time_the_sync_stored_the_issue() {
    let site = Site::new().await;
    site.execute(
        "UPDATE cache.issue SET synced_at = '2026-10-20T08:15:00.000Z' WHERE key = 'NW-1042'",
    )
    .await;

    let detail = site.detail("NW-1042").await;

    assert_eq!(detail.synced_at, "2026-10-20T08:15:00.000Z");
    assert_eq!(
        serde_json::to_value(&detail).unwrap()["syncedAt"],
        serde_json::json!("2026-10-20T08:15:00.000Z")
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn forgetting_a_gone_issue_removes_it_with_its_comments_changes_seen_row_and_search_row() {
    let site = Site::new().await;
    let id = site.id_of("NW-1042").await;
    site.execute(
        "INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
         VALUES ((SELECT id FROM cache.issue WHERE key = 'NW-1042'), 'summary', 'a', 'b',
                 '2026-10-28T09:00:00.000Z')",
    )
    .await;
    site.execute("UPDATE cache.issue SET gone = 1 WHERE key = 'NW-1042'")
        .await;
    let before = site.rows_of(&id).await;
    assert!(
        before.0 == 1 && before.1 > 0 && before.2 > 0 && before.3 == 1 && before.4 == 1,
        "{before:?}"
    );

    issue_forget(site.db(), id.clone()).await.unwrap();

    assert_eq!(site.rows_of(&id).await, (0, 0, 0, 0, 0));
    let err = issue_get(site.db(), viewer(), id).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn forgetting_a_gone_issue_keeps_the_other_issues() {
    let site = Site::new().await;
    let total = site
        .count_rows("SELECT COUNT(*) FROM cache.issue".into())
        .await;
    site.execute("UPDATE cache.issue SET gone = 1 WHERE key = 'NW-1042'")
        .await;

    issue_forget(site.db(), site.id_of("NW-1042").await)
        .await
        .unwrap();

    assert_eq!(
        site.count_rows("SELECT COUNT(*) FROM cache.issue".into())
            .await,
        total - 1
    );
    assert!(!site.detail("NW-1049").await.gone);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_live_issue_cannot_be_forgotten_and_stays() {
    let site = Site::new().await;
    let id = site.id_of("NW-1042").await;
    let before = site.rows_of(&id).await;

    let err = issue_forget(site.db(), id.clone()).await.unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert_eq!(site.rows_of(&id).await, before);
    assert_eq!(before.0, 1);
    assert_eq!(before.4, 1);
    assert!(!site.detail("NW-1042").await.gone);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn forgetting_an_unknown_issue_is_not_found() {
    let site = Site::new().await;

    let err = issue_forget(site.db(), "99999999".into())
        .await
        .unwrap_err();

    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_unknown_id_is_not_found() {
    let site = Site::new().await;

    let err = issue_get(site.db(), viewer(), "99999999".into())
        .await
        .unwrap_err();

    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_activity_lists_the_comments_in_time_order_with_their_authors() {
    let site = Site::new().await;

    let items = site.activity("NW-1042").await;

    let comments: Vec<(Option<Person>, bool)> = items
        .iter()
        .filter_map(|item| match item {
            ActivityItem::Comment { author, edited, .. } => Some((author.clone(), *edited)),
            ActivityItem::Change { .. } => None,
        })
        .collect();
    assert_eq!(
        comments,
        [
            (Some(marta()), false),
            (Some(alex()), false),
            (Some(marta()), false),
            (Some(marta()), false),
        ]
    );
    let times = comment_times(&items);
    assert_eq!(times.len(), 4);
    assert!(times.is_sorted(), "{times:?}");
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_first_sync_records_no_changes_so_only_comments_show() {
    let site = Site::new().await;

    let items = site.activity("NW-1042").await;

    assert_eq!(comment_times(&items).len(), items.len());
    assert!(site.activity("NW-1049").await.is_empty());
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_issue_created_after_the_first_sync_shows_a_created_change() {
    let site = Site::new().await;
    site.fake.set_now(later());
    let key = site
        .fake
        .add_issue("NW-1055", "NW", "Order confirmation email: shipping notice");

    site.resync().await;

    assert_eq!(
        site.activity(&key).await,
        [ActivityItem::Change {
            field: "created".into(),
            from: None,
            to: Some(ChangeValue::Text(key.clone())),
            at: LATER_UTC.into(),
            fresh: true,
        }]
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_status_edited_on_the_site_shows_as_a_change_with_names_and_styles() {
    let site = Site::new().await;
    site.execute("UPDATE cache.issue_seen SET seen_at = '2026-10-28T10:00:00.000Z'")
        .await;
    site.fake.set_now(later());
    site.fake
        .edit_issue("NW-1044", |fields| fields["status"] = done_status());

    site.resync().await;

    assert_eq!(
        site.activity("NW-1044").await,
        [ActivityItem::Change {
            field: "status".into(),
            from: Some(ChangeValue::Status {
                name: "In Review".into(),
                style: Style::Review,
            }),
            to: Some(ChangeValue::Status {
                name: "Done".into(),
                style: Style::Done,
            }),
            at: LATER_UTC.into(),
            fresh: true,
        }]
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_chosen_status_style_colours_the_status_change() {
    let site = Site::new().await;
    let status = site.status_of("NW-1044").await;
    status_style_set(
        site.db(),
        NW_PROJECT_ID.into(),
        status,
        Some(Style::Blocked),
    )
    .await
    .unwrap();
    site.fake.set_now(later());
    site.fake
        .edit_issue("NW-1044", |fields| fields["status"] = done_status());

    site.resync().await;

    let items = site.activity("NW-1044").await;
    let [ActivityItem::Change { from, .. }] = items.as_slice() else {
        panic!("expected one change, got {items:?}");
    };
    assert_eq!(
        *from,
        Some(ChangeValue::Status {
            name: "In Review".into(),
            style: Style::Blocked,
        })
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_assignee_edited_on_the_site_shows_as_a_change_between_people() {
    let site = Site::new().await;
    site.execute("UPDATE cache.issue_seen SET seen_at = '2026-10-28T10:00:00.000Z'")
        .await;
    site.fake.set_now(later());
    let marta_user = demo::demo()
        .users
        .iter()
        .find(|u| u["accountId"] == MARTA_ACCOUNT_ID)
        .cloned()
        .expect("Marta is a demo user");
    site.fake
        .edit_issue("NW-1049", |fields| fields["assignee"] = marta_user);

    site.resync().await;

    assert_eq!(
        site.activity("NW-1049").await,
        [ActivityItem::Change {
            field: "assignee".into(),
            from: Some(ChangeValue::Person(alex())),
            to: Some(ChangeValue::Person(marta())),
            at: LATER_UTC.into(),
            fresh: true,
        }]
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_epic_lists_its_issues_open_ones_by_status_then_key_and_done_ones_by_key() {
    let site = Site::new().await;

    let children = site.related("NW-1020").await.children;

    assert_eq!(
        keys(&children.open),
        ["NW-1042", "NW-1047", "NW-1038", "NW-1035", "NW-1051"]
    );
    assert_eq!(
        keys(&children.done),
        ["NW-1021", "NW-1024", "NW-1026", "NW-1031"]
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_task_lists_its_subtasks_open_and_done() {
    let site = Site::new().await;

    let children = site.related("NW-1042").await.children;

    assert_eq!(keys(&children.open), ["NW-1059"]);
    assert_eq!(keys(&children.done), ["NW-1043", "NW-1045"]);
    assert_eq!(children.open[0].issue_type, "Subtask");
    assert_eq!(children.open[0].style, Style::Todo);
    assert_eq!(children.open[0].assignee, Some(alex()));
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_issue_without_children_or_links_has_nothing_related() {
    let site = Site::new().await;

    let related = site.related("NW-1049").await;

    assert!(related.children.open.is_empty());
    assert!(related.children.done.is_empty());
    assert!(related.links.is_empty());
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_blocking_issue_links_to_the_blocked_one_with_its_cached_status() {
    let site = Site::new().await;

    let links = site.related("NW-1042").await.links;

    assert_eq!(
        links,
        [IssueLink {
            relation: "blocks".into(),
            issue: RelatedIssue {
                id: Some(site.id_of("NW-1051").await),
                key: "NW-1051".into(),
                summary: "Guest checkout: remember shipping country".into(),
                style: Style::Todo,
                issue_type: "Story".into(),
                assignee: Some(alex()),
                changed: false,
            },
        }]
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_blocked_issue_links_back_with_the_inward_words() {
    let site = Site::new().await;

    let links = site.related("NW-1051").await.links;

    assert_eq!(
        links,
        [IssueLink {
            relation: "is blocked by".into(),
            issue: RelatedIssue {
                id: Some(site.id_of("NW-1042").await),
                key: "NW-1042".into(),
                summary: "Checkout: address validation for DACH postcodes".into(),
                style: Style::Progress,
                issue_type: "Story".into(),
                assignee: Some(alex()),
                changed: false,
            },
        }]
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_linked_issue_the_cache_does_not_hold_shows_the_links_own_fields() {
    let site = Site::new().await;
    site.execute("UPDATE cache.issue SET gone = 1 WHERE key = 'NW-1051'")
        .await;

    let links = site.related("NW-1042").await.links;

    assert_eq!(
        links,
        [IssueLink {
            relation: "blocks".into(),
            issue: RelatedIssue {
                id: None,
                key: "NW-1051".into(),
                summary: "Guest checkout: remember shipping country".into(),
                style: Style::Todo,
                issue_type: "Story".into(),
                assignee: None,
                changed: false,
            },
        }]
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_related_issues_serialize_with_camel_case_keys() {
    let site = Site::new().await;

    let value = serde_json::to_value(site.related("NW-1042").await).unwrap();

    assert_eq!(
        value["links"][0]["issue"]["issueType"],
        serde_json::json!("Story")
    );
    assert!(value["children"]["open"].is_array());
    assert!(value["children"]["done"].is_array());
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn stale_comments_are_fetched_stored_and_then_fresh() {
    let site = Site::new().await;
    site.execute("UPDATE cache.issue SET updated_at = '2000-01-01T00:00:00.000Z'")
        .await;
    site.execute("UPDATE cache.issue SET comments_synced_at = NULL WHERE key = 'NW-1042'")
        .await;
    site.execute("DELETE FROM cache.comment WHERE issue_id = '101042'")
        .await;
    let fetches_before = site.comment_fetches("NW-1042");

    let first = site.sync_comments("NW-1042").await;
    let second = site.sync_comments("NW-1042").await;

    assert!(first);
    assert!(!second);
    assert_eq!(site.comment_fetches("NW-1042"), fetches_before + 1);
    assert_eq!(site.comment_count_in_cache("NW-1042").await, 4);
    assert_eq!(site.detail("NW-1042").await.comment_count, 4);
    assert_eq!(comment_times(&site.activity("NW-1042").await).len(), 4);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn comments_fetched_before_the_last_update_are_stale() {
    let site = Site::new().await;
    site.execute(
        "UPDATE cache.issue SET comments_synced_at = '2000-01-01T00:00:00.000Z'
         WHERE key = 'NW-1042'",
    )
    .await;

    assert!(site.sync_comments("NW-1042").await);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn fetching_replaces_the_cached_comments_and_the_count() {
    let site = Site::new().await;
    site.fake.add_comment(
        "NW-1042",
        demo::demo().users[0].clone(),
        "Liechtenstein rule is covered by a new test.",
    );
    site.execute("UPDATE cache.issue SET comments_synced_at = NULL WHERE key = 'NW-1042'")
        .await;

    assert!(site.sync_comments("NW-1042").await);

    assert_eq!(site.comment_count_in_cache("NW-1042").await, 5);
    assert_eq!(site.detail("NW-1042").await.comment_count, 5);
    assert_eq!(comment_times(&site.activity("NW-1042").await).len(), 5);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_issue_gone_from_the_site_is_not_fetched() {
    let site = Site::new().await;
    site.execute(
        "UPDATE cache.issue SET gone = 1, comments_synced_at = NULL WHERE key = 'NW-1042'",
    )
    .await;
    let fetches_before = site.comment_fetches("NW-1042");

    assert!(!site.sync_comments("NW-1042").await);

    assert_eq!(site.comment_fetches("NW-1042"), fetches_before);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_unknown_issue_is_not_fetched() {
    let site = Site::new().await;
    let connection = site.ctx.settings.get().connection;

    let source = source_for(&site.ctx.access, &connection).unwrap();

    let fetched = issue_sync_comments(site.db(), source.as_ref(), "99999999".into())
        .await
        .unwrap();

    assert!(!fetched);
}

fn remote_link(id: u64, url: &str, title: Option<&str>) -> serde_json::Value {
    let mut object = serde_json::json!({ "url": url });
    if let Some(title) = title {
        object["title"] = title.into();
    }
    serde_json::json!({ "id": id, "object": object })
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn syncing_web_links_stores_the_issues_links_in_the_trackers_order() {
    let site = Site::new().await;
    assert_eq!(site.web_links("NW-1042").await, []);

    assert!(site.sync_web_links("NW-1042").await);

    assert_eq!(
        site.web_links("NW-1042").await,
        [
            WebLink {
                id: "70001".into(),
                url: "https://github.com/example/shop/pull/481".into(),
                title: "Validate DACH postcodes".into(),
                host: Some("github.com".into()),
                kind: WebLinkKind::PullRequest,
                reference: Some("shop#481".into()),
                openable: true,
            },
            WebLink {
                id: "70002".into(),
                url: "https://docs.example.com/checkout/postcode-rules".into(),
                title: "Postcode rules for the checkout".into(),
                host: Some("docs.example.com".into()),
                kind: WebLinkKind::Link,
                reference: None,
                openable: true,
            },
        ]
    );
    assert_eq!(site.web_links("NW-1038").await, []);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn syncing_unchanged_web_links_again_reports_no_change() {
    let site = Site::new().await;
    assert!(site.sync_web_links("NW-1042").await);
    let before = site.web_links("NW-1042").await;

    assert!(!site.sync_web_links("NW-1042").await);

    assert_eq!(site.web_links("NW-1042").await, before);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn changed_web_links_replace_the_cached_ones() {
    let site = Site::new().await;
    assert!(site.sync_web_links("NW-1042").await);
    site.fake.set_remote_links(
        "NW-1042",
        vec![
            remote_link(
                70_010,
                "https://gitlab.example.com/shop/store-api/-/merge_requests/58",
                Some("Cart totals from the Store API"),
            ),
            remote_link(
                70_001,
                "https://github.com/example/shop/pull/481",
                Some("Validate DACH postcodes"),
            ),
        ],
    );

    assert!(site.sync_web_links("NW-1042").await);

    let links = site.web_links("NW-1042").await;
    let ids: Vec<&str> = links.iter().map(|link| link.id.as_str()).collect();
    assert_eq!(ids, ["70010", "70001"]);
    assert_eq!(links[0].reference.as_deref(), Some("store-api#58"));
    assert!(!site.sync_web_links("NW-1042").await);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn web_links_fetched_within_five_minutes_are_not_asked_for_again() {
    let site = Site::new().await;
    assert!(site.try_sync_web_links_at("NW-1042", 0).await.unwrap());
    let before = site.web_links("NW-1042").await;
    let fetches = site.web_link_fetches("NW-1042").await;
    site.fake.set_remote_links("NW-1042", vec![]);

    for minutes in [0, 1, 4] {
        assert!(
            !site
                .try_sync_web_links_at("NW-1042", minutes)
                .await
                .unwrap(),
            "{minutes} min"
        );
    }

    assert_eq!(site.web_link_fetches("NW-1042").await, fetches);
    assert_eq!(site.web_links("NW-1042").await, before);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn web_links_are_fetched_again_after_five_minutes_and_show_the_new_ones() {
    let site = Site::new().await;
    assert!(site.try_sync_web_links_at("NW-1042", 0).await.unwrap());
    let fetches = site.web_link_fetches("NW-1042").await;
    site.fake.set_remote_links(
        "NW-1042",
        vec![remote_link(
            70_010,
            "https://gitlab.example.com/shop/store-api/-/merge_requests/58",
            Some("Cart totals from the Store API"),
        )],
    );

    assert!(site.try_sync_web_links_at("NW-1042", 5).await.unwrap());

    assert_eq!(site.web_link_fetches("NW-1042").await, fetches + 1);
    let links = site.web_links("NW-1042").await;
    let ids: Vec<&str> = links.iter().map(|link| link.id.as_str()).collect();
    assert_eq!(ids, ["70010"]);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_failed_web_link_fetch_does_not_count_as_a_fetch() {
    let site = Site::new().await;
    site.fake.fail_next(1, 500, None);
    assert!(site.try_sync_web_links_at("NW-1042", 0).await.is_err());
    let fetches = site.web_link_fetches("NW-1042").await;

    assert!(site.try_sync_web_links_at("NW-1042", 1).await.unwrap());

    assert_eq!(site.web_link_fetches("NW-1042").await, fetches + 1);
    assert_eq!(site.web_links("NW-1042").await.len(), 2);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn web_links_removed_on_the_site_are_removed_from_the_cache() {
    let site = Site::new().await;
    assert!(site.sync_web_links("NW-1042").await);
    site.fake.set_remote_links("NW-1042", vec![]);

    assert!(site.sync_web_links("NW-1042").await);

    assert_eq!(site.web_links("NW-1042").await, []);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_failed_web_link_fetch_keeps_the_cached_links() {
    let site = Site::new().await;
    assert!(site.sync_web_links("NW-1042").await);
    let before = site.web_links("NW-1042").await;
    site.fake.set_remote_links("NW-1042", vec![]);
    site.fake.fail_next(1, 500, None);

    let err = site.try_sync_web_links("NW-1042").await.unwrap_err();

    assert_eq!(err.kind(), ErrorKind::Internal);
    assert_eq!(site.web_links("NW-1042").await, before);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_offline_site_fails_the_web_link_fetch_and_keeps_the_cached_links() {
    let site = Site::new().await;
    assert!(site.sync_web_links("NW-1042").await);
    let before = site.web_links("NW-1042").await;
    site.fake.go_offline().await;

    let err = site.try_sync_web_links("NW-1042").await.unwrap_err();

    assert_eq!(err.kind(), ErrorKind::Offline);
    assert_eq!(site.web_links("NW-1042").await, before);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn web_links_of_an_unknown_issue_are_not_fetched() {
    let site = Site::new().await;
    let connection = site.ctx.settings.get().connection;
    let source = source_for(&site.ctx.access, &connection).unwrap();
    let fetches_before = site.fake.paths().len();

    let changed = issue_sync_web_links(site.db(), source.as_ref(), "99999999".into(), viewer().now)
        .await
        .unwrap();

    assert!(!changed);
    assert_eq!(site.fake.paths().len(), fetches_before);
    assert_eq!(
        issue_web_links(site.db(), "99999999".into()).await.unwrap(),
        []
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn web_links_of_an_issue_gone_from_the_site_are_not_fetched() {
    let site = Site::new().await;
    site.execute("UPDATE cache.issue SET gone = 1 WHERE key = 'NW-1042'")
        .await;

    assert!(!site.sync_web_links("NW-1042").await);

    assert_eq!(site.web_links("NW-1042").await, []);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_web_link_without_a_title_shows_its_address_and_one_without_an_address_is_left_out() {
    let site = Site::new().await;
    site.fake.set_remote_links(
        "NW-1042",
        vec![
            remote_link(1, "https://example.com/untitled", None),
            remote_link(2, "https://example.com/blank", Some("  ")),
            remote_link(3, "", Some("No address")),
            remote_link(4, "https://example.com/named", Some("Named")),
        ],
    );

    assert!(site.sync_web_links("NW-1042").await);

    let shown: Vec<(&str, String)> = vec![
        ("1", "https://example.com/untitled".into()),
        ("2", "https://example.com/blank".into()),
        ("4", "Named".into()),
    ];
    let links = site.web_links("NW-1042").await;
    let found: Vec<(&str, String)> = links
        .iter()
        .map(|link| (link.id.as_str(), link.title.clone()))
        .collect();
    assert_eq!(found, shown);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn web_links_that_are_not_https_are_listed_but_not_openable() {
    let site = Site::new().await;
    site.fake.set_remote_links(
        "NW-1042",
        vec![
            remote_link(1, "http://wiki.example.com/postcodes", Some("Wiki")),
            remote_link(2, "javascript:alert(1)", Some("Script")),
        ],
    );

    assert!(site.sync_web_links("NW-1042").await);

    let links = site.web_links("NW-1042").await;
    let found: Vec<(Option<&str>, bool)> = links
        .iter()
        .map(|link| (link.host.as_deref(), link.openable))
        .collect();
    assert_eq!(found, [(Some("wiki.example.com"), false), (None, false)]);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn forgetting_an_issue_drops_its_web_links() {
    let site = Site::new().await;
    assert!(site.sync_web_links("NW-1042").await);
    let id = site.id_of("NW-1042").await;
    site.execute("UPDATE cache.issue SET gone = 1 WHERE key = 'NW-1042'")
        .await;
    let count = format!("SELECT COUNT(*) FROM cache.remote_link WHERE issue_id = '{id}'");
    assert_eq!(site.count_rows(count.clone()).await, 2);

    issue_forget(site.db(), id).await.unwrap();

    assert_eq!(site.count_rows(count).await, 0);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn the_custom_fields_an_issue_fills_are_listed_by_name_without_being_shown() {
    let site = Site::new().await;

    let detail = site.detail("NW-1042").await;

    let found: Vec<(&str, &str)> = detail
        .custom
        .iter()
        .map(|f| (f.id.as_str(), f.name.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            (REVIEWER, "Reviewer"),
            ("customfield_10016", "Story points")
        ]
    );
    assert_eq!(detail.custom[0].value, Some(Cell::Person(marta())));
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_field_the_space_hides_from_the_detail_is_left_out_of_its_issues() {
    let site = Site::new().await;

    for (visibility, listed) in [
        (FieldVisibility::HiddenInDetail, false),
        (FieldVisibility::Hidden, false),
        (FieldVisibility::HiddenInList, true),
        (FieldVisibility::Visible, true),
    ] {
        space_set_field(site.db(), NW_PROJECT_ID.into(), REVIEWER.into(), visibility)
            .await
            .unwrap();

        let detail = site.detail("NW-1042").await;

        assert_eq!(
            detail.custom.iter().any(|f| f.id == REVIEWER),
            listed,
            "{visibility:?}"
        );
        assert!(
            detail.custom.iter().any(|f| f.id == "customfield_10016"),
            "{visibility:?}"
        );
    }
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn an_issue_that_fills_no_custom_field_lists_none_until_one_is_shown() {
    let site = Site::new().await;

    assert_eq!(site.detail("NW-1056").await.custom, []);
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_bug_lists_its_template_rich_text_fields_with_their_rendered_html() {
    let site = Site::new().await;
    let raw: serde_json::Value = serde_json::from_str(
        &site
            .db()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT raw FROM cache.issue WHERE key = 'NW-1038'",
                    [],
                    |r| r.get::<_, String>(0),
                )?)
            })
            .await
            .unwrap(),
    )
    .unwrap();
    let html = |id: &str| raw["renderedFields"][id].as_str().unwrap().to_string();

    let detail = site.detail("NW-1038").await;

    let rich: Vec<&CustomField> = detail.custom.iter().filter(|f| f.html.is_some()).collect();
    assert_eq!(
        rich,
        [
            &CustomField {
                id: "customfield_10081".into(),
                name: "Acceptance criteria".into(),
                value: None,
                html: Some(html("customfield_10081")),
            },
            &CustomField {
                id: "customfield_10080".into(),
                name: "Steps to reproduce".into(),
                value: None,
                html: Some(html("customfield_10080")),
            },
        ]
    );
    assert!(
        site.detail("NW-1042")
            .await
            .custom
            .iter()
            .all(|f| f.html.is_none())
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_custom_column_shows_up_in_the_detail_with_its_name_and_value() {
    let site = Site::new().await;
    site.show_reviewer_column().await;

    let detail = site.detail("NW-1042").await;

    assert_eq!(
        detail.custom[0],
        CustomField {
            id: REVIEWER.into(),
            name: "Reviewer".into(),
            value: Some(Cell::Person(marta())),
            html: None,
        }
    );
}

#[tokio::test]
#[ignore = "syncs the demo site into a temp database"]
async fn a_custom_field_without_a_value_is_listed_with_no_value() {
    let site = Site::new().await;
    site.show_reviewer_column().await;

    let detail = site.detail("NW-1049").await;

    assert_eq!(
        detail.custom[0],
        CustomField {
            id: REVIEWER.into(),
            name: "Reviewer".into(),
            value: None,
            html: None,
        }
    );
}
