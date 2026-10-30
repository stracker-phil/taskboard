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
use taskboard_lib::error::ErrorKind;
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::store::settings::{Settings, SettingsStore};
use taskboard_lib::sync::{Emit, SyncContext, SyncEvent, full_sync};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::writes::{Change, Reverse, WriteOutcome, apply, forget_all, toggle_mine};

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
        let Change::Assign { account_id } = reverse;
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
