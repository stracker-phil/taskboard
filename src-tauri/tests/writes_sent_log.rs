//! The record of changes sent to Jira (`writes::sent`) after the writes against the loopback fake Jira: which
//! changes leave a record, what it says, and which ones leave none.

use std::sync::Arc;

use serde_json::json;
use tempfile::TempDir;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::jira::capability::Capability;
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::{Emit, SyncContext, SyncEvent, full_sync};
use taskboard_lib::test_support::demo::ME_EMAIL;
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::writes::sent::{SentChange, sent_changes};
use taskboard_lib::writes::{Change, Offers, add_comment, add_web_link, apply};

const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const ALEX: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const MARTA: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03";
/// Assigned to Alex (me) on the demo site.
const NW_1042: &str = "101042";
/// Assigned to Marta Klein on the demo site.
const NW_1044: &str = "101044";
const DONE: &str = "10001";
const SPEC_URL: &str = "https://docs.example.com/checkout/spec";

struct Rig {
    fake: FakeJira,
    _dir: TempDir,
    ctx: SyncContext,
    offers: Offers,
}

impl Rig {
    /// Connected to the demo site, its five Spaces tracked and synced, changes to Jira allowed.
    async fn synced() -> Self {
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
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        save(
            &ctx.settings,
            &ctx.access,
            input,
            Some(DEMO_TOKEN.into()),
            today,
        )
        .await
        .unwrap();
        refresh_projects(&ctx.db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();
        for id in PROJECT_IDS {
            track_project(&ctx.db, id.into(), true).await.unwrap();
        }
        let emit: Emit = Arc::new(|_event: SyncEvent| {});
        full_sync(&ctx, &*emit).await.unwrap();
        ctx.settings
            .edit(|s| s.behaviour.allow_jira_changes = true)
            .await
            .unwrap();
        Self {
            fake,
            _dir: dir,
            ctx,
            offers: Offers::default(),
        }
    }

    async fn change(
        &self,
        issue_id: &str,
        change: Change,
    ) -> taskboard_lib::error::AppResult<taskboard_lib::writes::WriteOutcome> {
        let source = self.ctx.source().unwrap();
        apply(
            &self.ctx.db,
            &*source,
            &self.offers,
            &self.ctx.settings.get(),
            issue_id.into(),
            change,
        )
        .await
    }

    async fn assign(
        &self,
        issue_id: &str,
        account_id: Option<&str>,
    ) -> taskboard_lib::error::AppResult<taskboard_lib::writes::WriteOutcome> {
        self.change(
            issue_id,
            Change::Assign {
                account_id: account_id.map(String::from),
            },
        )
        .await
    }

    async fn comment(
        &self,
        issue_id: &str,
        text: &str,
    ) -> taskboard_lib::error::AppResult<taskboard_lib::writes::WriteOutcome> {
        let source = self.ctx.source().unwrap();
        add_comment(
            &self.ctx.db,
            &*source,
            &self.ctx.settings.get(),
            issue_id.into(),
            text.into(),
        )
        .await
    }

    async fn link(
        &self,
        issue_id: &str,
    ) -> taskboard_lib::error::AppResult<taskboard_lib::writes::WriteOutcome> {
        let source = self.ctx.source().unwrap();
        add_web_link(
            &self.ctx.db,
            &*source,
            &self.ctx.settings.get(),
            issue_id.into(),
            SPEC_URL.into(),
            Some("Spec".into()),
        )
        .await
    }

    async fn switch_off(&self, capability: Capability) {
        self.ctx
            .settings
            .edit(move |s| s.behaviour.disabled_changes = vec![capability])
            .await
            .unwrap();
    }

    /// What the record holds, newest first, as `(issue key, change, refused)`.
    async fn record(&self) -> Vec<(Option<String>, String, bool)> {
        let all: Vec<SentChange> = self
            .ctx
            .db
            .read(|conn| sent_changes(conn, None))
            .await
            .unwrap();
        all.into_iter()
            .map(|c| (c.issue_key, c.change, c.error.is_some()))
            .collect()
    }
}

fn entry(key: &str, change: &str, refused: bool) -> (Option<String>, String, bool) {
    (Some(key.to_string()), change.to_string(), refused)
}

#[tokio::test]
async fn assigning_someone_records_their_name() {
    let rig = Rig::synced().await;

    rig.assign(NW_1042, Some(MARTA)).await.unwrap();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", "Assignee: Marta Klein", false)]
    );
}

#[tokio::test]
async fn assigning_to_me_records_my_display_name() {
    let rig = Rig::synced().await;
    rig.ctx
        .settings
        .edit(|s| s.connection.display_name = Some("Alex Morgan".into()))
        .await
        .unwrap();

    rig.assign(NW_1044, Some(ALEX)).await.unwrap();

    assert_eq!(
        rig.record().await,
        [entry("NW-1044", "Assignee: Alex Morgan", false)]
    );
}

#[tokio::test]
async fn unassigning_records_unassigned() {
    let rig = Rig::synced().await;

    rig.assign(NW_1042, None).await.unwrap();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", "Assignee: Unassigned", false)]
    );
}

#[tokio::test]
async fn moving_an_issue_records_the_target_status() {
    let rig = Rig::synced().await;

    rig.change(
        NW_1042,
        Change::Transition {
            transition_id: DONE.into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", "Status: Done", false)]
    );
}

#[tokio::test]
async fn setting_the_priority_records_its_name() {
    let rig = Rig::synced().await;

    rig.change(NW_1042, Change::Priority { name: "Low".into() })
        .await
        .unwrap();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", "Priority: Low", false)]
    );
}

#[tokio::test]
async fn renaming_records_the_length_of_the_title_and_never_the_title() {
    let rig = Rig::synced().await;

    rig.change(
        NW_1042,
        Change::Title {
            text: "  Grüße aus Köln  ".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", "Title (14 characters)", false)]
    );
}

#[tokio::test]
async fn a_rename_jira_refuses_is_recorded_as_refused() {
    let rig = Rig::synced().await;
    rig.fake
        .answer_next_write(400, json!({ "errorMessages": ["Not allowed"] }));

    rig.change(
        NW_1042,
        Change::Title {
            text: "Checkout copy".into(),
        },
    )
    .await
    .unwrap_err();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", "Title (13 characters)", true)]
    );
}

#[tokio::test]
async fn a_rename_that_is_not_sent_leaves_no_record() {
    let rig = Rig::synced().await;
    rig.change(
        NW_1042,
        Change::Title {
            text: "Checkout: address validation for DACH postcodes".into(),
        },
    )
    .await
    .unwrap();
    for text in ["", "   ", &"a".repeat(256)] {
        rig.change(NW_1042, Change::Title { text: text.into() })
            .await
            .unwrap_err();
    }
    rig.switch_off(Capability::ChangeTitle).await;
    rig.change(
        NW_1042,
        Change::Title {
            text: "Checkout copy".into(),
        },
    )
    .await
    .unwrap_err();

    assert_eq!(rig.record().await, []);
}

#[tokio::test]
async fn a_comment_records_its_length_and_never_its_text() {
    let rig = Rig::synced().await;

    rig.comment(NW_1042, "Checked on staging").await.unwrap();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", "Comment (18 characters)", false)]
    );
}

#[tokio::test]
async fn a_comment_counts_characters_not_bytes() {
    let rig = Rig::synced().await;

    rig.comment(NW_1042, "  Grüße aus Köln  ").await.unwrap();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", "Comment (14 characters)", false)]
    );
}

#[tokio::test]
async fn a_web_link_records_its_address() {
    let rig = Rig::synced().await;

    rig.link(NW_1042).await.unwrap();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", &format!("Web link: {SPEC_URL}"), false)]
    );
}

#[tokio::test]
async fn the_record_lists_the_newest_change_first() {
    let rig = Rig::synced().await;

    rig.assign(NW_1042, None).await.unwrap();
    rig.change(NW_1042, Change::Priority { name: "Low".into() })
        .await
        .unwrap();

    assert_eq!(
        rig.record().await,
        [
            entry("NW-1042", "Priority: Low", false),
            entry("NW-1042", "Assignee: Unassigned", false),
        ]
    );
}

#[tokio::test]
async fn a_change_jira_refuses_is_recorded_with_its_refusal() {
    let rig = Rig::synced().await;
    rig.fake
        .answer_next(400, json!({ "errorMessages": ["Not allowed"] }));

    let err = rig.assign(NW_1042, Some(MARTA)).await.unwrap_err();

    let stored = rig
        .ctx
        .db
        .read(|conn| sent_changes(conn, None))
        .await
        .unwrap();
    assert_eq!(
        stored,
        [SentChange {
            at: stored[0].at.clone(),
            issue_key: Some("NW-1042".into()),
            change: "Assignee: Marta Klein".into(),
            error: Some(err.to_string()),
        }]
    );
}

#[tokio::test]
async fn a_comment_jira_refuses_is_recorded_as_refused() {
    let rig = Rig::synced().await;
    rig.fake
        .answer_next_write(400, json!({ "errorMessages": ["Not allowed"] }));

    rig.comment(NW_1042, "Checked").await.unwrap_err();

    assert_eq!(
        rig.record().await,
        [entry("NW-1042", "Comment (7 characters)", true)]
    );
}

#[tokio::test]
async fn changes_that_are_off_leave_no_record() {
    let rig = Rig::synced().await;
    rig.ctx
        .settings
        .edit(|s| s.behaviour.allow_jira_changes = false)
        .await
        .unwrap();

    rig.assign(NW_1044, Some(ALEX)).await.unwrap_err();
    rig.change(NW_1042, Change::Priority { name: "Low".into() })
        .await
        .unwrap_err();
    rig.comment(NW_1042, "Checked").await.unwrap_err();
    rig.link(NW_1042).await.unwrap_err();

    assert_eq!(rig.record().await, []);
}

#[tokio::test]
async fn a_kind_of_change_switched_off_leaves_no_record_while_the_others_do() {
    let rig = Rig::synced().await;
    rig.switch_off(Capability::Comment).await;

    rig.comment(NW_1042, "Checked").await.unwrap_err();
    assert_eq!(rig.record().await, []);

    rig.link(NW_1042).await.unwrap();
    assert_eq!(
        rig.record().await,
        [entry("NW-1042", &format!("Web link: {SPEC_URL}"), false)]
    );
}

#[tokio::test]
async fn a_change_that_is_already_there_leaves_no_record() {
    let rig = Rig::synced().await;

    rig.assign(NW_1042, Some(ALEX)).await.unwrap();
    rig.assign(NW_1044, Some(MARTA)).await.unwrap();

    assert_eq!(rig.record().await, []);
}

#[tokio::test]
async fn a_comment_that_is_refused_before_sending_leaves_no_record() {
    let rig = Rig::synced().await;

    rig.comment(NW_1042, "   ").await.unwrap_err();
    rig.comment("999999", "Checked").await.unwrap_err();

    assert_eq!(rig.record().await, []);
}
