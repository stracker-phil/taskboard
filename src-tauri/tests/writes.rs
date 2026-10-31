//! Changes Taskboard makes in Jira (`writes/`) against the loopback fake Jira on temp folders: what the cache,
//! Jira and the toast hold afterwards, refusals, and how the sync treats an issue with a write on its way.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use tempfile::TempDir;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::error::{AppError, ErrorKind};
use taskboard_lib::issues::issue_sync_comments;
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::status_style::{Style, status_style_set};
use taskboard_lib::store::settings::{Settings, SettingsStore};
use taskboard_lib::sync::{Emit, SyncContext, SyncEvent, full_sync};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::writes::{
    Change, MAX_COMMENT_CHARS, Reverse, TransitionOption, WriteOutcome, add_comment, apply,
    forget_all, toggle_mine, transitions,
};

const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const ALEX: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const JONAS: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e02";
const MARTA: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03";
/// Assigned to Alex (me) on the demo site.
const NW_1042: &str = "101042";
/// Assigned to Marta Klein on the demo site.
const NW_1044: &str = "101044";
const NW_1049: &str = "101049";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()
}

fn demo_now() -> DateTime<Utc> {
    DEMO_NOW.parse().unwrap()
}

fn demo_user(account_id: &str) -> Value {
    demo::demo()
        .users
        .into_iter()
        .find(|user| user["accountId"] == account_id)
        .unwrap()
}

fn emit() -> Emit {
    Arc::new(|_event: SyncEvent| {})
}

struct Rig {
    fake: FakeJira,
    _dir: TempDir,
    ctx: SyncContext,
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
        let rig = Self {
            fake,
            _dir: dir,
            ctx,
        };
        rig.sync().await;
        rig.ctx
            .settings
            .edit(|s| s.behaviour.allow_jira_changes = true)
            .await
            .unwrap();
        rig
    }

    async fn sync(&self) {
        let emit = emit();
        full_sync(&self.ctx, &*emit).await.unwrap();
    }

    fn settings(&self) -> Settings {
        self.ctx.settings.get()
    }

    async fn apply(&self, issue_id: &str, account_id: Option<&str>) -> WriteOutcome {
        self.try_apply(issue_id, account_id).await.unwrap()
    }

    async fn try_apply(
        &self,
        issue_id: &str,
        account_id: Option<&str>,
    ) -> taskboard_lib::error::AppResult<WriteOutcome> {
        let source = self.ctx.source().unwrap();
        let change = Change::Assign {
            account_id: account_id.map(String::from),
        };
        apply(
            &self.ctx.db,
            &*source,
            &self.settings(),
            issue_id.into(),
            change,
        )
        .await
    }

    async fn toggle(&self, issue_id: &str) -> WriteOutcome {
        let source = self.ctx.source().unwrap();
        toggle_mine(&self.ctx.db, &*source, &self.settings(), issue_id.into())
            .await
            .unwrap()
    }

    /// What the cache shows as the assignee.
    async fn cached_assignee(&self, issue_id: &str) -> Option<String> {
        let id = issue_id.to_string();
        self.ctx
            .db
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT assignee_id FROM cache.issue WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )?)
            })
            .await
            .unwrap()
    }

    /// What Jira (the fake) holds as the assignee.
    async fn jira_assignee(&self, issue_id: &str) -> Option<String> {
        let source = self.ctx.source().unwrap();
        let found = source
            .fetch(&[issue_id.to_string()], &["assignee".to_string()])
            .await
            .unwrap();
        found.issues[0].fields["assignee"]["accountId"]
            .as_str()
            .map(String::from)
    }

    async fn pending_writes(&self) -> i64 {
        self.ctx
            .db
            .read(|conn| {
                Ok(
                    conn.query_row("SELECT COUNT(*) FROM cache.pending_write", [], |row| {
                        row.get(0)
                    })?,
                )
            })
            .await
            .unwrap()
    }

    async fn recorded_changes(&self) -> i64 {
        self.ctx
            .db
            .read(|conn| {
                Ok(
                    conn.query_row("SELECT COUNT(*) FROM cache.issue_change", [], |row| {
                        row.get(0)
                    })?,
                )
            })
            .await
            .unwrap()
    }

    /// Writes the fake received, the way a failed or refused write is told from one never sent.
    fn assignments_sent(&self) -> usize {
        self.fake
            .requests()
            .iter()
            .filter(|request| request.path.ends_with("/assignee"))
            .count()
    }

    /// A write on its way: the row already shows `value`, as `writes::apply` leaves it.
    async fn start_pending_assignment(&self, issue_id: &'static str, value: &'static str) {
        self.ctx
            .db
            .write(move |tx, _| {
                let base: Option<String> = tx.query_row(
                    "SELECT assignee_id FROM cache.issue WHERE id = ?1",
                    [issue_id],
                    |row| row.get(0),
                )?;
                tx.execute(
                    "INSERT INTO cache.pending_write (issue_id, field, value, base, created_at)
                     VALUES (?1, 'assignee', ?2, ?3, '2026-10-28T11:00:00.000Z')",
                    rusqlite::params![issue_id, value, base],
                )?;
                tx.execute(
                    "UPDATE cache.issue SET assignee_id = ?2 WHERE id = ?1",
                    rusqlite::params![issue_id, value],
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }
}

/// Statuses of the demo site: NW-1042 and NW-1049 start In Progress. A move's id is its target status's id.
const IN_PROGRESS: &str = "3";
const TO_DO: &str = "10000";
const DONE: &str = "10001";
const IN_REVIEW: &str = "10002";
const BLOCKED: &str = "10003";
const NW_PROJECT: &str = "10001";

fn demo_status(id: &str) -> Value {
    demo::demo()
        .statuses
        .into_iter()
        .find(|status| status["id"] == id)
        .unwrap()
}

impl Rig {
    /// `(status id, status category)` the cache shows.
    async fn cached_status(&self, issue_id: &str) -> (String, String) {
        let id = issue_id.to_string();
        self.ctx
            .db
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT status_id, status_category FROM cache.issue WHERE id = ?1",
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?)
            })
            .await
            .unwrap()
    }

    /// The status id Jira (the fake) holds.
    async fn jira_status(&self, issue_id: &str) -> String {
        let source = self.ctx.source().unwrap();
        let found = source
            .fetch(&[issue_id.to_string()], &["status".to_string()])
            .await
            .unwrap();
        found.issues[0].fields["status"]["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    async fn move_to(&self, issue_id: &str, transition_id: &str) -> WriteOutcome {
        self.try_move_to(issue_id, transition_id).await.unwrap()
    }

    async fn try_move_to(
        &self,
        issue_id: &str,
        transition_id: &str,
    ) -> taskboard_lib::error::AppResult<WriteOutcome> {
        let source = self.ctx.source().unwrap();
        apply(
            &self.ctx.db,
            &*source,
            &self.settings(),
            issue_id.into(),
            Change::Transition {
                transition_id: transition_id.into(),
            },
        )
        .await
    }

    async fn moves(&self, issue_id: &str) -> Vec<TransitionOption> {
        let source = self.ctx.source().unwrap();
        transitions(&self.ctx.db, &*source, issue_id.into())
            .await
            .unwrap()
    }

    fn transitions_posted(&self) -> usize {
        self.fake
            .requests()
            .iter()
            .filter(|r| r.method == "POST" && r.path.ends_with("/transitions"))
            .count()
    }

    /// A move on its way: the row already shows the status, as `writes::apply` leaves it.
    async fn start_pending_status(&self, issue_id: &'static str, status: &'static str) {
        let category = demo_status(status)["statusCategory"]["key"]
            .as_str()
            .unwrap()
            .to_string();
        self.ctx
            .db
            .write(move |tx, _| {
                let base: String = tx.query_row(
                    "SELECT status_id FROM cache.issue WHERE id = ?1",
                    [issue_id],
                    |row| row.get(0),
                )?;
                tx.execute(
                    "INSERT INTO cache.pending_write (issue_id, field, value, base, created_at)
                     VALUES (?1, 'status', ?2, ?3, '2026-10-28T11:00:00.000Z')",
                    rusqlite::params![issue_id, status, base],
                )?;
                tx.execute(
                    "UPDATE cache.issue SET status_id = ?2, status_category = ?3 WHERE id = ?1",
                    rusqlite::params![issue_id, status, category],
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }
}

const COMMENTS_ON_NW_1042: i64 = 4;

impl Rig {
    async fn number(&self, sql: &'static str, issue_id: &str) -> i64 {
        let id = issue_id.to_string();
        self.ctx
            .db
            .read(move |conn| Ok(conn.query_row(sql, [id], |row| row.get(0))?))
            .await
            .unwrap()
    }

    /// The `comment_count` the issue row shows.
    async fn shown_comment_count(&self, issue_id: &str) -> i64 {
        self.number(
            "SELECT comment_count FROM cache.issue WHERE id = ?1",
            issue_id,
        )
        .await
    }

    async fn cached_comments(&self, issue_id: &str) -> i64 {
        self.number(
            "SELECT COUNT(*) FROM cache.comment WHERE issue_id = ?1 AND id NOT LIKE 'pending:%'",
            issue_id,
        )
        .await
    }

    async fn pending_comments(&self, issue_id: &str) -> i64 {
        self.number(
            "SELECT COUNT(*) FROM cache.comment WHERE issue_id = ?1 AND id LIKE 'pending:%'",
            issue_id,
        )
        .await
    }

    async fn jira_comments(&self, issue_key: &str) -> usize {
        let source = self.ctx.source().unwrap();
        source.comments(issue_key).await.unwrap().len()
    }

    async fn comment(
        &self,
        issue_id: &str,
        text: &str,
    ) -> taskboard_lib::error::AppResult<WriteOutcome> {
        self.comment_with(&self.settings(), issue_id, text).await
    }

    async fn comment_with(
        &self,
        settings: &Settings,
        issue_id: &str,
        text: &str,
    ) -> taskboard_lib::error::AppResult<WriteOutcome> {
        let source = self.ctx.source().unwrap();
        add_comment(
            &self.ctx.db,
            &*source,
            settings,
            issue_id.into(),
            text.into(),
        )
        .await
    }

    fn comments_posted(&self) -> usize {
        self.fake
            .requests()
            .iter()
            .filter(|r| r.method == "POST" && r.path.ends_with("/comment"))
            .count()
    }

    /// A comment on its way, as `writes::add_comment` leaves it: shown, counted, not yet in Jira.
    async fn start_pending_comment(&self, issue_id: &'static str) {
        self.ctx
            .db
            .write(move |tx, _| {
                tx.execute(
                    "INSERT INTO cache.comment (id, issue_id, author_id, body_html, body_text, created_at, updated_at)
                     VALUES ('pending:1', ?1, ?2, '<p>On its way</p>', 'On its way',
                             '2026-10-28T11:00:00.000Z', '2026-10-28T11:00:00.000Z')",
                    rusqlite::params![issue_id, ALEX],
                )?;
                tx.execute(
                    "UPDATE cache.issue SET comment_count = comment_count + 1 WHERE id = ?1",
                    [issue_id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }
}

fn option(id: &str, status_name: &str, style: Style, opens_jira: bool) -> TransitionOption {
    TransitionOption {
        id: id.into(),
        status_name: status_name.into(),
        style,
        opens_jira,
    }
}

fn outcome(done: &str, label: &str, back_to: Option<&str>) -> WriteOutcome {
    WriteOutcome {
        done: done.into(),
        reverse: Some(Reverse {
            label: label.into(),
            change: Change::Assign {
                account_id: back_to.map(String::from),
            },
        }),
    }
}

mod assigning {
    use super::*;

    #[tokio::test]
    async fn an_issue_of_someone_else_to_me_shows_in_the_cache_and_in_jira_without_a_new_change() {
        let rig = Rig::synced().await;

        let done = rig.apply(NW_1044, Some(ALEX)).await;

        assert_eq!(
            done,
            outcome(
                "Assigned NW-1044 to you",
                "Change back to Marta Klein",
                Some(MARTA)
            )
        );
        assert_eq!(rig.cached_assignee(NW_1044).await.as_deref(), Some(ALEX));
        assert_eq!(rig.jira_assignee(NW_1044).await.as_deref(), Some(ALEX));
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.recorded_changes().await, 0);
        assert_eq!(rig.assignments_sent(), 1);
    }

    #[tokio::test]
    async fn unassigning_my_issue_names_me_in_the_reverse() {
        let rig = Rig::synced().await;

        let done = rig.apply(NW_1042, None).await;

        assert_eq!(
            done,
            outcome("Unassigned NW-1042", "Change back to you", Some(ALEX))
        );
        assert_eq!(rig.cached_assignee(NW_1042).await, None);
        assert_eq!(rig.jira_assignee(NW_1042).await, None);
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.recorded_changes().await, 0);
    }

    #[tokio::test]
    async fn an_unassigned_issue_to_me_reverses_to_unassigned() {
        let rig = Rig::synced().await;
        rig.apply(NW_1042, None).await;

        let done = rig.apply(NW_1042, Some(ALEX)).await;

        assert_eq!(
            done,
            outcome("Assigned NW-1042 to you", "Change back to Unassigned", None)
        );
        assert_eq!(rig.cached_assignee(NW_1042).await.as_deref(), Some(ALEX));
        assert_eq!(rig.jira_assignee(NW_1042).await.as_deref(), Some(ALEX));
    }

    #[tokio::test]
    async fn to_another_person_names_them_and_the_reverse_sets_the_old_assignee_back() {
        let rig = Rig::synced().await;

        let done = rig.apply(NW_1042, Some(MARTA)).await;

        assert_eq!(
            done,
            outcome(
                "Assigned NW-1042 to Marta Klein",
                "Change back to you",
                Some(ALEX)
            )
        );
        assert_eq!(rig.cached_assignee(NW_1042).await.as_deref(), Some(MARTA));
        assert_eq!(rig.jira_assignee(NW_1042).await.as_deref(), Some(MARTA));

        let reverse = done.reverse.unwrap().change;
        let Change::Assign { account_id } = reverse else {
            panic!("expected an assignment, got {reverse:?}");
        };
        rig.apply(NW_1042, account_id.as_deref()).await;

        assert_eq!(rig.cached_assignee(NW_1042).await.as_deref(), Some(ALEX));
        assert_eq!(rig.jira_assignee(NW_1042).await.as_deref(), Some(ALEX));
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn to_the_current_assignee_sends_nothing_and_offers_no_reverse() {
        let rig = Rig::synced().await;
        for (id, account, done) in [
            (NW_1042, Some(ALEX), "NW-1042 is already assigned to you"),
            (
                NW_1044,
                Some(MARTA),
                "NW-1044 is already assigned to Marta Klein",
            ),
        ] {
            let outcome = rig.apply(id, account).await;

            assert_eq!(
                outcome,
                WriteOutcome {
                    done: done.into(),
                    reverse: None
                },
                "{id}"
            );
        }
        rig.apply(NW_1042, None).await;
        let sent = rig.assignments_sent();

        let again = rig.apply(NW_1042, None).await;

        assert_eq!(
            again,
            WriteOutcome {
                done: "NW-1042 is already unassigned".into(),
                reverse: None
            }
        );
        assert_eq!(rig.assignments_sent(), sent);
    }

    #[tokio::test]
    async fn the_outcome_serializes_for_the_page_in_camel_case() {
        let rig = Rig::synced().await;

        let done = rig.apply(NW_1044, Some(ALEX)).await;

        assert_eq!(
            serde_json::to_value(&done).unwrap(),
            json!({
                "done": "Assigned NW-1044 to you",
                "reverse": {
                    "label": "Change back to Marta Klein",
                    "change": { "kind": "assign", "accountId": MARTA },
                },
            })
        );
    }
}

mod toggling_mine {
    use super::*;

    #[tokio::test]
    async fn unassigns_an_issue_that_is_mine() {
        let rig = Rig::synced().await;

        let done = rig.toggle(NW_1042).await;

        assert_eq!(
            done,
            outcome("Unassigned NW-1042", "Change back to you", Some(ALEX))
        );
        assert_eq!(rig.cached_assignee(NW_1042).await, None);
        assert_eq!(rig.jira_assignee(NW_1042).await, None);
    }

    #[tokio::test]
    async fn assigns_me_an_issue_of_someone_else() {
        let rig = Rig::synced().await;

        let done = rig.toggle(NW_1044).await;

        assert_eq!(
            done,
            outcome(
                "Assigned NW-1044 to you",
                "Change back to Marta Klein",
                Some(MARTA)
            )
        );
        assert_eq!(rig.cached_assignee(NW_1044).await.as_deref(), Some(ALEX));
        assert_eq!(rig.jira_assignee(NW_1044).await.as_deref(), Some(ALEX));
    }

    #[tokio::test]
    async fn twice_in_a_row_ends_where_it_began() {
        let rig = Rig::synced().await;

        rig.toggle(NW_1044).await;
        rig.toggle(NW_1044).await;

        assert_eq!(rig.cached_assignee(NW_1044).await, None);
        assert_eq!(rig.jira_assignee(NW_1044).await, None);
        assert_eq!(rig.pending_writes().await, 0);
    }
}

mod refusals {
    use super::*;

    #[tokio::test]
    async fn changes_to_jira_that_are_off_send_nothing_and_change_nothing() {
        let rig = Rig::synced().await;
        rig.ctx
            .settings
            .edit(|s| s.behaviour.allow_jira_changes = false)
            .await
            .unwrap();

        let err = rig.try_apply(NW_1044, Some(ALEX)).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.assignments_sent(), 0);
        assert_eq!(rig.cached_assignee(NW_1044).await.as_deref(), Some(MARTA));
        assert_eq!(rig.jira_assignee(NW_1044).await.as_deref(), Some(MARTA));
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn without_knowing_who_i_am_nothing_is_sent() {
        let rig = Rig::synced().await;
        let mut settings = rig.settings();
        settings.connection.account_id = None;
        let source = rig.ctx.source().unwrap();

        let err = apply(
            &rig.ctx.db,
            &*source,
            &settings,
            NW_1044.into(),
            Change::Assign {
                account_id: Some(JONAS.into()),
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotConnected);
        assert_eq!(rig.assignments_sent(), 0);
        assert_eq!(rig.cached_assignee(NW_1044).await.as_deref(), Some(MARTA));
    }

    #[tokio::test]
    async fn an_issue_missing_from_the_cache_is_not_found() {
        let rig = Rig::synced().await;

        let err = rig.try_apply("999999", Some(ALEX)).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(rig.assignments_sent(), 0);
    }

    #[tokio::test]
    async fn a_write_jira_refuses_puts_the_old_assignee_back_and_returns_the_error() {
        for status in [400, 403] {
            let rig = Rig::synced().await;
            rig.fake
                .answer_next(status, json!({ "errorMessages": ["Not allowed"] }));

            let err = rig.try_apply(NW_1044, Some(ALEX)).await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{status}");
            assert_eq!(
                rig.cached_assignee(NW_1044).await.as_deref(),
                Some(MARTA),
                "{status}"
            );
            assert_eq!(rig.pending_writes().await, 0, "{status}");
            assert_eq!(rig.recorded_changes().await, 0, "{status}");
            assert_eq!(rig.assignments_sent(), 1, "{status}");
        }
    }

    #[tokio::test]
    async fn a_failed_write_is_not_sent_again_and_a_later_one_goes_through() {
        let rig = Rig::synced().await;
        rig.fake.fail_next(1, 400, None);

        rig.try_apply(NW_1044, Some(ALEX)).await.unwrap_err();
        let done = rig.apply(NW_1044, Some(ALEX)).await;

        assert_eq!(done.done, "Assigned NW-1044 to you");
        assert_eq!(rig.assignments_sent(), 2);
        assert_eq!(rig.jira_assignee(NW_1044).await.as_deref(), Some(ALEX));
    }

    #[tokio::test]
    async fn a_second_change_while_one_is_on_its_way_is_refused_and_sends_nothing() {
        let rig = Rig::synced().await;
        rig.start_pending_assignment(NW_1044, JONAS).await;

        let err = rig.try_apply(NW_1044, Some(ALEX)).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.assignments_sent(), 0);
        assert_eq!(rig.cached_assignee(NW_1044).await.as_deref(), Some(JONAS));
        assert_eq!(rig.pending_writes().await, 1);
    }
}

mod the_sync_with_a_write_on_its_way {
    use super::*;

    #[tokio::test]
    async fn keeps_the_pending_assignee_over_what_jira_sent_and_remembers_jiras_as_the_base() {
        let rig = Rig::synced().await;
        rig.start_pending_assignment(NW_1049, MARTA).await;
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake
            .edit_issue("NW-1049", |fields| fields["assignee"] = demo_user(JONAS));

        rig.sync().await;

        assert_eq!(rig.cached_assignee(NW_1049).await.as_deref(), Some(MARTA));
        let base: Option<String> = rig
            .ctx
            .db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT base FROM cache.pending_write WHERE issue_id = ?1",
                    [NW_1049],
                    |row| row.get(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(base.as_deref(), Some(JONAS));
    }

    #[tokio::test]
    async fn takes_what_jira_sent_once_the_write_is_forgotten() {
        let rig = Rig::synced().await;
        rig.start_pending_assignment(NW_1049, MARTA).await;
        rig.ctx.db.write(|tx, _| forget_all(tx)).await.unwrap();
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake
            .edit_issue("NW-1049", |fields| fields["assignee"] = demo_user(JONAS));

        rig.sync().await;

        assert_eq!(rig.cached_assignee(NW_1049).await.as_deref(), Some(JONAS));
    }

    #[tokio::test]
    async fn a_page_older_than_the_cached_issue_changes_nothing() {
        let rig = Rig::synced().await;
        rig.fake.set_now(demo_now() - chrono::Duration::days(3));
        rig.fake
            .edit_issue("NW-1049", |fields| fields["assignee"] = demo_user(JONAS));

        rig.sync().await;

        assert_eq!(rig.cached_assignee(NW_1049).await.as_deref(), Some(ALEX));
        assert_eq!(rig.recorded_changes().await, 0);
    }
}

mod the_moves_of_an_issue {
    use super::*;

    #[tokio::test]
    async fn are_listed_in_jiras_order_named_by_their_target_status_and_styled() {
        let rig = Rig::synced().await;
        rig.fake.require_screen(BLOCKED);

        let moves = rig.moves(NW_1042).await;

        assert_eq!(
            moves,
            [
                option(TO_DO, "To Do", Style::Todo, false),
                option(IN_REVIEW, "In Review", Style::Review, false),
                option(BLOCKED, "Blocked", Style::Blocked, true),
                option(DONE, "Done", Style::Done, false),
            ]
        );
    }

    #[tokio::test]
    async fn take_the_style_the_owner_chose_for_the_status_in_the_issues_space() {
        let rig = Rig::synced().await;
        status_style_set(
            &rig.ctx.db,
            NW_PROJECT.into(),
            IN_REVIEW.into(),
            Some(Style::Progress),
        )
        .await
        .unwrap();

        let moves = rig.moves(NW_1042).await;

        let review = moves.iter().find(|m| m.id == IN_REVIEW).unwrap();
        assert_eq!(review.style, Style::Progress);
    }

    #[tokio::test]
    async fn leave_out_the_status_the_issue_is_in() {
        let rig = Rig::synced().await;

        let ids: Vec<String> = rig.moves(NW_1044).await.into_iter().map(|m| m.id).collect();

        assert_eq!(ids, [TO_DO, IN_PROGRESS, BLOCKED, DONE]);
    }

    #[tokio::test]
    async fn serialize_for_the_page_in_camel_case() {
        let rig = Rig::synced().await;
        rig.fake.require_screen(DONE);

        let moves = rig.moves(NW_1042).await;

        assert_eq!(
            serde_json::to_value(&moves[3]).unwrap(),
            json!({
                "id": DONE,
                "statusName": "Done",
                "style": "done",
                "opensJira": true,
            })
        );
    }
}

mod moving {
    use super::*;

    fn invalid_input(result: taskboard_lib::error::AppResult<WriteOutcome>) -> String {
        match result {
            Err(taskboard_lib::error::AppError::InvalidInput(message)) => message,
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn shows_the_new_status_in_the_cache_and_in_jira_without_a_new_change() {
        let rig = Rig::synced().await;

        let done = rig.move_to(NW_1042, DONE).await;

        assert_eq!(
            done,
            WriteOutcome {
                done: "Moved NW-1042 to Done".into(),
                reverse: Some(Reverse {
                    label: "Change back to In Progress".into(),
                    change: Change::Transition {
                        transition_id: IN_PROGRESS.into()
                    },
                }),
            }
        );
        assert_eq!(
            rig.cached_status(NW_1042).await,
            (DONE.to_string(), "done".to_string())
        );
        assert_eq!(rig.jira_status(NW_1042).await, DONE);
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.recorded_changes().await, 0);
        assert_eq!(rig.transitions_posted(), 1);
    }

    #[tokio::test]
    async fn the_reverse_moves_the_issue_back_and_offers_the_move_forward_again() {
        let rig = Rig::synced().await;
        let done = rig.move_to(NW_1042, DONE).await;
        let Some(Reverse {
            change: Change::Transition { transition_id },
            ..
        }) = done.reverse
        else {
            panic!("expected a move back, got {done:?}");
        };

        let back = rig.move_to(NW_1042, &transition_id).await;

        assert_eq!(back.done, "Moved NW-1042 to In Progress");
        assert_eq!(
            back.reverse.map(|r| r.label),
            Some("Change back to Done".to_string())
        );
        assert_eq!(
            rig.cached_status(NW_1042).await,
            (IN_PROGRESS.to_string(), "indeterminate".to_string())
        );
        assert_eq!(rig.jira_status(NW_1042).await, IN_PROGRESS);
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn offers_no_reverse_when_the_way_back_needs_a_screen() {
        let rig = Rig::synced().await;
        rig.fake.require_screen(IN_PROGRESS);

        let done = rig.move_to(NW_1042, DONE).await;

        assert_eq!(
            done,
            WriteOutcome {
                done: "Moved NW-1042 to Done".into(),
                reverse: None
            }
        );
        assert_eq!(rig.jira_status(NW_1042).await, DONE);
    }

    #[tokio::test]
    async fn a_move_that_needs_jiras_screen_is_refused_and_nothing_is_sent() {
        let rig = Rig::synced().await;
        rig.fake.require_screen(BLOCKED);

        let message = invalid_input(rig.try_move_to(NW_1042, BLOCKED).await);

        assert_eq!(
            message,
            "Moving to Blocked needs Jira's own screen; open the issue in Jira"
        );
        assert_eq!(rig.transitions_posted(), 0);
        assert_eq!(
            rig.cached_status(NW_1042).await,
            (IN_PROGRESS.to_string(), "indeterminate".to_string())
        );
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn a_move_jira_does_not_list_is_refused_and_nothing_is_sent() {
        let rig = Rig::synced().await;
        for (label, id) in [
            ("an unknown move", "99999"),
            ("the current status", IN_PROGRESS),
        ] {
            let message = invalid_input(rig.try_move_to(NW_1042, id).await);

            assert_eq!(
                message, "Jira no longer offers this move; the issue may have changed",
                "{label}"
            );
        }
        assert_eq!(rig.transitions_posted(), 0);
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn an_issue_already_in_that_status_sends_nothing_and_offers_no_reverse() {
        let rig = Rig::synced().await;
        rig.ctx
            .db
            .write(|tx, _| {
                tx.execute(
                    "UPDATE cache.issue SET status_id = ?2, status_category = 'done' WHERE id = ?1",
                    rusqlite::params![NW_1042, DONE],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        let done = rig.move_to(NW_1042, DONE).await;

        assert_eq!(
            done,
            WriteOutcome {
                done: "NW-1042 is already Done".into(),
                reverse: None
            }
        );
        assert_eq!(rig.transitions_posted(), 0);
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn an_issue_missing_from_the_cache_is_not_found() {
        let rig = Rig::synced().await;
        rig.ctx
            .db
            .write(|tx, _| {
                tx.execute("UPDATE cache.issue SET gone = 1 WHERE id = ?1", [NW_1042])?;
                Ok(())
            })
            .await
            .unwrap();

        let err = rig.try_move_to(NW_1042, DONE).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(rig.transitions_posted(), 0);
    }

    #[tokio::test]
    async fn changes_to_jira_that_are_off_send_nothing() {
        let rig = Rig::synced().await;
        rig.ctx
            .settings
            .edit(|s| s.behaviour.allow_jira_changes = false)
            .await
            .unwrap();

        let requests_before = rig.fake.requests().len();

        let err = rig.try_move_to(NW_1042, DONE).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.fake.requests().len(), requests_before);
        assert_eq!(rig.jira_status(NW_1042).await, IN_PROGRESS);
    }

    #[tokio::test]
    async fn a_write_jira_refuses_puts_the_old_status_back_and_returns_the_error() {
        let rig = Rig::synced().await;
        rig.fake
            .answer_next_write(400, json!({ "errorMessages": ["Not allowed"] }));

        let err = rig.try_move_to(NW_1042, DONE).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(
            rig.cached_status(NW_1042).await,
            (IN_PROGRESS.to_string(), "indeterminate".to_string())
        );
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.recorded_changes().await, 0);
        assert_eq!(rig.transitions_posted(), 1);
    }

    #[tokio::test]
    async fn a_second_change_to_the_status_while_one_is_on_its_way_is_refused() {
        let rig = Rig::synced().await;
        rig.start_pending_status(NW_1049, IN_REVIEW).await;

        let err = rig.try_move_to(NW_1049, DONE).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.transitions_posted(), 0);
        assert_eq!(
            rig.cached_status(NW_1049).await,
            (IN_REVIEW.to_string(), "indeterminate".to_string())
        );
        assert_eq!(rig.pending_writes().await, 1);
    }
}

mod the_sync_with_a_move_on_its_way {
    use super::*;

    #[tokio::test]
    async fn keeps_the_pending_status_and_its_category_over_what_jira_sent() {
        let rig = Rig::synced().await;
        rig.start_pending_status(NW_1049, DONE).await;
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["status"] = demo_status(IN_REVIEW)
        });

        rig.sync().await;

        assert_eq!(
            rig.cached_status(NW_1049).await,
            (DONE.to_string(), "done".to_string())
        );
    }

    #[tokio::test]
    async fn takes_what_jira_sent_once_the_move_is_forgotten() {
        let rig = Rig::synced().await;
        rig.start_pending_status(NW_1049, DONE).await;
        rig.ctx.db.write(|tx, _| forget_all(tx)).await.unwrap();
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["status"] = demo_status(IN_REVIEW)
        });

        rig.sync().await;

        assert_eq!(
            rig.cached_status(NW_1049).await,
            (IN_REVIEW.to_string(), "indeterminate".to_string())
        );
    }
}

mod commenting {
    use super::*;

    fn invalid_input(result: taskboard_lib::error::AppResult<WriteOutcome>) -> String {
        match result {
            Err(AppError::InvalidInput(message)) => message,
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn stores_jiras_comment_in_place_of_the_pending_one_without_a_new_change() {
        let rig = Rig::synced().await;

        let done = rig.comment(NW_1042, "Checked on staging").await.unwrap();

        assert_eq!(
            done,
            WriteOutcome {
                done: "Commented on NW-1042".into(),
                reverse: None
            }
        );
        assert_eq!(rig.pending_comments(NW_1042).await, 0);
        assert_eq!(rig.cached_comments(NW_1042).await, COMMENTS_ON_NW_1042 + 1);
        assert_eq!(
            rig.shown_comment_count(NW_1042).await,
            rig.jira_comments("NW-1042").await as i64
        );
        let (author, text): (String, String) = rig
            .ctx
            .db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT author_id, body_text FROM cache.comment
                     WHERE issue_id = '101042' ORDER BY created_at DESC, id DESC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(
            (author.as_str(), text.as_str()),
            (ALEX, "Checked on staging")
        );
        assert_eq!(rig.recorded_changes().await, 0);
        assert_eq!(rig.comments_posted(), 1);
    }

    #[tokio::test]
    async fn shows_the_comment_and_its_count_at_once_while_it_is_on_its_way() {
        let rig = Rig::synced().await;
        rig.fake.set_latency(std::time::Duration::from_millis(100));

        let (result, seen) = tokio::join!(rig.comment(NW_1042, "Checked on staging"), async {
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                loop {
                    if rig.pending_comments(NW_1042).await == 1 {
                        return rig.shown_comment_count(NW_1042).await;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
            })
            .await
            .expect("the pending comment never showed")
        });

        result.unwrap();
        assert_eq!(seen, COMMENTS_ON_NW_1042 + 1);
        assert_eq!(rig.pending_comments(NW_1042).await, 0);
    }

    #[tokio::test]
    async fn text_that_is_empty_or_only_whitespace_is_refused_and_nothing_is_sent() {
        let rig = Rig::synced().await;
        let requests_before = rig.fake.requests().len();
        for text in ["", "   ", "\n\t \n"] {
            let message = invalid_input(rig.comment(NW_1042, text).await);

            assert_eq!(message, "Write something first", "{text:?}");
        }
        assert_eq!(rig.fake.requests().len(), requests_before);
        assert_eq!(rig.pending_comments(NW_1042).await, 0);
        assert_eq!(rig.shown_comment_count(NW_1042).await, COMMENTS_ON_NW_1042);
    }

    #[tokio::test]
    async fn text_over_the_limit_is_refused_and_nothing_is_sent() {
        let rig = Rig::synced().await;
        let requests_before = rig.fake.requests().len();

        let message = invalid_input(
            rig.comment(NW_1042, &"a".repeat(MAX_COMMENT_CHARS + 1))
                .await,
        );

        assert_eq!(
            message,
            format!("Jira takes comments of up to {MAX_COMMENT_CHARS} characters")
        );
        assert_eq!(MAX_COMMENT_CHARS, 32_767);
        assert_eq!(rig.fake.requests().len(), requests_before);
        assert_eq!(rig.pending_comments(NW_1042).await, 0);
        assert_eq!(rig.shown_comment_count(NW_1042).await, COMMENTS_ON_NW_1042);
    }

    #[tokio::test]
    async fn changes_to_jira_that_are_off_send_nothing_and_add_nothing() {
        let rig = Rig::synced().await;
        rig.ctx
            .settings
            .edit(|s| s.behaviour.allow_jira_changes = false)
            .await
            .unwrap();
        let requests_before = rig.fake.requests().len();

        let err = rig.comment(NW_1042, "Checked").await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.fake.requests().len(), requests_before);
        assert_eq!(rig.pending_comments(NW_1042).await, 0);
        assert_eq!(rig.shown_comment_count(NW_1042).await, COMMENTS_ON_NW_1042);
    }

    #[tokio::test]
    async fn an_issue_missing_from_the_cache_is_not_found_and_nothing_is_sent() {
        let rig = Rig::synced().await;

        let err = rig.comment("999999", "Checked").await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(rig.comments_posted(), 0);
    }

    #[tokio::test]
    async fn a_comment_jira_refuses_is_withdrawn_with_its_count_and_the_error_returned() {
        let rig = Rig::synced().await;
        rig.fake
            .answer_next_write(400, json!({ "errorMessages": ["Not allowed"] }));

        let err = rig.comment(NW_1042, "Checked").await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.pending_comments(NW_1042).await, 0);
        assert_eq!(rig.cached_comments(NW_1042).await, COMMENTS_ON_NW_1042);
        assert_eq!(rig.shown_comment_count(NW_1042).await, COMMENTS_ON_NW_1042);
        assert_eq!(rig.recorded_changes().await, 0);
        assert_eq!(rig.comments_posted(), 1);
        assert_eq!(
            rig.jira_comments("NW-1042").await as i64,
            COMMENTS_ON_NW_1042
        );
    }
}

mod the_comment_syncs_with_a_comment_on_its_way {
    use super::*;

    #[tokio::test]
    async fn a_sync_page_keeps_the_pending_comment_and_stores_jiras_comments() {
        let rig = Rig::synced().await;
        rig.start_pending_comment(NW_1042).await;
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake.add_comment(
            "NW-1042",
            demo_user(JONAS),
            "Merged into the release branch",
        );

        rig.sync().await;

        assert_eq!(rig.pending_comments(NW_1042).await, 1);
        assert_eq!(rig.cached_comments(NW_1042).await, COMMENTS_ON_NW_1042 + 1);
    }

    #[tokio::test]
    async fn fetching_an_issues_comments_keeps_the_pending_comment() {
        let rig = Rig::synced().await;
        rig.start_pending_comment(NW_1042).await;
        rig.ctx
            .db
            .write(|tx, _| {
                tx.execute(
                    "UPDATE cache.issue SET comments_synced_at = NULL WHERE id = ?1",
                    [NW_1042],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let source = rig.ctx.source().unwrap();

        let fetched = issue_sync_comments(&rig.ctx.db, &*source, NW_1042.into())
            .await
            .unwrap();

        assert!(fetched);
        assert_eq!(rig.pending_comments(NW_1042).await, 1);
        assert_eq!(rig.cached_comments(NW_1042).await, COMMENTS_ON_NW_1042);
    }
}

mod forgetting_writes {
    use super::*;

    #[tokio::test]
    async fn drops_every_pending_write_and_leaves_the_values_the_issues_show() {
        let rig = Rig::synced().await;
        rig.start_pending_assignment(NW_1049, MARTA).await;
        rig.start_pending_assignment(NW_1044, JONAS).await;

        let dropped = rig.ctx.db.write(|tx, _| forget_all(tx)).await.unwrap();

        assert_eq!(dropped, 2);
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.cached_assignee(NW_1049).await.as_deref(), Some(MARTA));
        assert_eq!(rig.cached_assignee(NW_1044).await.as_deref(), Some(JONAS));
    }
}
