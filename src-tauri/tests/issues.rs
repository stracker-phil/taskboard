//! An issue as the peek shows it, on the demo site synced from the in-process fake Jira into a temp database.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use tempfile::TempDir;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, fetch_versions, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::error::ErrorKind;
use taskboard_lib::issues::{IssueDetail, issue_get};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::Viewer;
use taskboard_lib::status_style::{Style, status_style_set};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::{Emit, SyncContext, full_sync};
use taskboard_lib::test_support::demo::{DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::rows::{self, EpicChip};

const ME_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const MARTA_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03";
const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];

fn viewer() -> Viewer {
    Viewer {
        me: Some(ME_ACCOUNT_ID.into()),
        time_zone: Some("Europe/Vienna".into()),
        now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
    }
}

struct Site {
    _fake: FakeJira,
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
            _fake: fake,
            _dir: dir,
            ctx,
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
    status_style_set(site.db(), status.clone(), Some(Style::Blocked))
        .await
        .unwrap();

    assert_eq!(site.detail("NW-1038").await.style, Style::Blocked);

    status_style_set(site.db(), status, None).await.unwrap();

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
    ] {
        assert!(value.get(key).is_some(), "{key}");
    }
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
