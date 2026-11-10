//! Changes Taskboard makes in Jira (`writes/`) against the loopback fake Jira on temp folders: what the cache,
//! Jira and the toast hold afterwards, refusals, and how the sync treats an issue with a write on its way.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use tempfile::TempDir;

use taskboard_lib::changes::{UNSEEN, mark_unread};
use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::error::{AppError, AppResult, ErrorKind};
use taskboard_lib::issues::{
    WebLink, WebLinkKind, issue_sync_comments, issue_sync_web_links, issue_web_links,
};
use taskboard_lib::jira::capability::Capability;
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::status_style::{Style, status_style_set};
use taskboard_lib::store::settings::{Settings, SettingsStore};
use taskboard_lib::sync::{Emit, Job, SyncContext, SyncEvent, full_sync, refetch_issue, run_job};
use taskboard_lib::team::{Me, space_set_member};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::GroupBy;
use taskboard_lib::writes::{
    Assignees, Change, DropOutcome, DropTargets, MAX_COMMENT_CHARS, Reverse, TransitionOption,
    WriteOutcome, add_comment, add_web_link, apply, assignees, drop_on, drop_targets, forget_all,
    priorities, toggle_mine, transitions,
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

    /// The cached comments count as possibly behind, so the next comment sync fetches.
    async fn make_comments_stale(&self, issue_id: &'static str) {
        self.ctx
            .db
            .write(move |tx, _| {
                tx.execute(
                    "UPDATE cache.issue SET comments_synced_at = NULL WHERE id = ?1",
                    [issue_id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
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

impl Rig {
    /// What the cache shows as the priority.
    async fn cached_priority(&self, issue_id: &str) -> Option<String> {
        let id = issue_id.to_string();
        self.ctx
            .db
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT priority FROM cache.issue WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )?)
            })
            .await
            .unwrap()
    }

    /// The priority name Jira (the fake) holds.
    async fn jira_priority(&self, issue_id: &str) -> Option<String> {
        let source = self.ctx.source().unwrap();
        let found = source
            .fetch(&[issue_id.to_string()], &["priority".to_string()])
            .await
            .unwrap();
        found.issues[0].fields["priority"]["name"]
            .as_str()
            .map(String::from)
    }

    async fn try_set_priority(
        &self,
        issue_id: &str,
        name: &str,
    ) -> taskboard_lib::error::AppResult<WriteOutcome> {
        let source = self.ctx.source().unwrap();
        apply(
            &self.ctx.db,
            &*source,
            &self.settings(),
            issue_id.into(),
            Change::Priority { name: name.into() },
        )
        .await
    }

    async fn set_priority(&self, issue_id: &str, name: &str) -> WriteOutcome {
        self.try_set_priority(issue_id, name).await.unwrap()
    }

    /// Edits the fake received to change an issue's fields.
    fn priorities_sent(&self) -> usize {
        self.fake
            .requests()
            .iter()
            .filter(|r| {
                r.method == "PUT" && r.path.contains("/issue/") && !r.path.ends_with("/assignee")
            })
            .count()
    }

    /// A priority on its way: the row already shows `value`, as `writes::apply` leaves it.
    async fn start_pending_priority(&self, issue_id: &'static str, value: &'static str) {
        self.ctx
            .db
            .write(move |tx, _| {
                let base: Option<String> = tx.query_row(
                    "SELECT priority FROM cache.issue WHERE id = ?1",
                    [issue_id],
                    |row| row.get(0),
                )?;
                tx.execute(
                    "INSERT INTO cache.pending_write (issue_id, field, value, base, created_at)
                     VALUES (?1, 'priority', ?2, ?3, '2026-10-28T11:00:00.000Z')",
                    rusqlite::params![issue_id, value, base],
                )?;
                tx.execute(
                    "UPDATE cache.issue SET priority = ?2 WHERE id = ?1",
                    rusqlite::params![issue_id, value],
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
        status_id: id.into(),
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

mod assignee_menu {
    use super::*;

    async fn menu(rig: &Rig, issue_id: &str, me: Option<&str>) -> AppResult<Assignees> {
        let me = me.map(|account_id| Me {
            account_id: account_id.into(),
            name: "Alex Morgan".into(),
        });
        assignees(&rig.ctx.db, issue_id.into(), me).await
    }

    fn names(menu: &Assignees) -> Vec<&str> {
        menu.people.iter().map(|p| p.name.as_str()).collect()
    }

    #[tokio::test]
    async fn the_menu_offers_the_issues_spaces_team_with_me_first() {
        let rig = Rig::synced().await;

        let menu = menu(&rig, NW_1044, Some(ALEX)).await.unwrap();

        assert_eq!(
            names(&menu),
            ["Alex Morgan", "Anna Lutz", "Jonas Reiter", "Marta Klein"]
        );
        assert_eq!(menu.people[0].account_id, ALEX);
    }

    #[tokio::test]
    async fn without_me_the_team_is_sorted_by_name() {
        let rig = Rig::synced().await;

        let menu = menu(&rig, NW_1044, None).await.unwrap();

        assert_eq!(
            names(&menu),
            ["Alex Morgan", "Anna Lutz", "Jonas Reiter", "Marta Klein"]
        );
    }

    #[tokio::test]
    async fn the_current_assignee_is_the_one_the_issue_has() {
        let rig = Rig::synced().await;

        assert_eq!(
            menu(&rig, NW_1044, Some(ALEX)).await.unwrap().current,
            Some(MARTA.to_string())
        );
        assert_eq!(
            menu(&rig, NW_1042, Some(ALEX)).await.unwrap().current,
            Some(ALEX.to_string())
        );
    }

    #[tokio::test]
    async fn an_unassigned_issue_has_no_current_assignee() {
        let rig = Rig::synced().await;
        rig.apply(NW_1042, None).await;

        let menu = menu(&rig, NW_1042, Some(ALEX)).await.unwrap();

        assert_eq!(menu.current, None);
    }

    #[tokio::test]
    async fn a_person_switched_off_in_the_space_is_not_offered() {
        let rig = Rig::synced().await;
        space_set_member(&rig.ctx.db, NW_PROJECT.into(), JONAS.into(), false, None)
            .await
            .unwrap();

        let menu = menu(&rig, NW_1044, Some(ALEX)).await.unwrap();

        assert_eq!(names(&menu), ["Alex Morgan", "Anna Lutz", "Marta Klein"]);
    }

    #[tokio::test]
    async fn i_am_offered_first_even_when_switched_off_the_team() {
        let rig = Rig::synced().await;
        space_set_member(&rig.ctx.db, NW_PROJECT.into(), ALEX.into(), false, None)
            .await
            .unwrap();

        let menu = menu(&rig, NW_1044, Some(ALEX)).await.unwrap();

        assert_eq!(
            names(&menu),
            ["Alex Morgan", "Anna Lutz", "Jonas Reiter", "Marta Klein"]
        );
        assert_eq!(menu.people[0].account_id, ALEX);
    }

    #[tokio::test]
    async fn an_unknown_issue_is_not_found() {
        let rig = Rig::synced().await;

        let err = menu(&rig, "999999", Some(ALEX)).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn the_menu_serializes_with_camel_case_fields() {
        let rig = Rig::synced().await;

        let menu = menu(&rig, NW_1044, Some(ALEX)).await.unwrap();

        let json = serde_json::to_value(&menu).unwrap();
        assert_eq!(json["current"], MARTA);
        assert_eq!(json["people"][0]["accountId"], ALEX);
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
                "statusId": DONE,
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

    #[tokio::test]
    async fn fetching_an_issues_comments_keeps_the_pending_one_in_the_count() {
        let rig = Rig::synced().await;
        rig.start_pending_comment(NW_1042).await;
        rig.make_comments_stale(NW_1042).await;
        let source = rig.ctx.source().unwrap();

        issue_sync_comments(&rig.ctx.db, &*source, NW_1042.into())
            .await
            .unwrap();

        assert_eq!(
            rig.shown_comment_count(NW_1042).await,
            COMMENTS_ON_NW_1042 + 1
        );
    }

    #[tokio::test]
    async fn a_comment_that_settles_after_such_a_fetch_records_no_new_comment() {
        let rig = Rig::synced().await;
        rig.start_pending_comment(NW_1042).await;
        rig.make_comments_stale(NW_1042).await;
        let source = rig.ctx.source().unwrap();
        issue_sync_comments(&rig.ctx.db, &*source, NW_1042.into())
            .await
            .unwrap();
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake
            .add_comment("NW-1042", demo_user(ALEX), "On its way");

        refetch_issue(&rig.ctx.db, &*source, NW_1042, |tx| {
            tx.execute("DELETE FROM cache.comment WHERE id LIKE 'pending:%'", [])?;
            Ok(())
        })
        .await
        .unwrap();

        assert_eq!(rig.pending_comments(NW_1042).await, 0);
        assert_eq!(
            rig.shown_comment_count(NW_1042).await,
            COMMENTS_ON_NW_1042 + 1
        );
        assert_eq!(rig.recorded_changes().await, 0);
    }
}

mod my_own_writes_are_not_news {
    use super::*;

    const LONG_AGO: &str = "2026-10-28T08:00:00.000Z";

    impl Rig {
        /// Seen long ago, with no change recorded: `updated` is what the write moves it up to.
        async fn seen_long_ago(&self, issue_id: &'static str) {
            self.ctx
                .db
                .write(move |tx, _| {
                    tx.execute(
                        "INSERT INTO issue_seen (issue_id, seen_at) VALUES (?1, ?2)
                         ON CONFLICT (issue_id) DO UPDATE SET seen_at = excluded.seen_at",
                        rusqlite::params![issue_id, LONG_AGO],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }

        /// `(seen_at, updated_at)` of the issue.
        async fn seen_and_updated(&self, issue_id: &str) -> (String, String) {
            let id = issue_id.to_string();
            self.ctx
                .db
                .read(move |conn| {
                    Ok(conn.query_row(
                        "SELECT s.seen_at, i.updated_at FROM issue_seen s
                         JOIN cache.issue i ON i.id = s.issue_id WHERE i.id = ?1",
                        [id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )?)
                })
                .await
                .unwrap()
        }

        async fn is_unread(&self, issue_id: &str) -> bool {
            let id = issue_id.to_string();
            self.ctx
                .db
                .read(move |conn| {
                    Ok(conn.query_row(
                        &format!(
                            "SELECT EXISTS (SELECT 1 FROM cache.issue i WHERE i.id = ?1 AND {UNSEEN})"
                        ),
                        [id],
                        |row| row.get(0),
                    )?)
                })
                .await
                .unwrap()
        }

        async fn delta(&self) {
            let emit = emit();
            run_job(&self.ctx, Job::Delta, &*emit, None).await;
        }

        /// After the write the issue is read, seen up to Jira's `updated`, and stays read after a delta sync.
        async fn assert_read_up_to_updated(&self, issue_id: &str) {
            let (seen, updated) = self.seen_and_updated(issue_id).await;
            assert_eq!(seen, updated);
            assert_ne!(seen, LONG_AGO);
            assert!(!self.is_unread(issue_id).await);

            self.delta().await;

            assert!(!self.is_unread(issue_id).await);
        }
    }

    #[tokio::test]
    async fn a_status_change_leaves_the_issue_read() {
        let rig = Rig::synced().await;
        rig.seen_long_ago(NW_1042).await;

        rig.move_to(NW_1042, DONE).await;

        rig.assert_read_up_to_updated(NW_1042).await;
    }

    #[tokio::test]
    async fn an_assignment_leaves_the_issue_read() {
        let rig = Rig::synced().await;
        rig.seen_long_ago(NW_1044).await;

        rig.apply(NW_1044, Some(ALEX)).await;

        rig.assert_read_up_to_updated(NW_1044).await;
    }

    #[tokio::test]
    async fn a_comment_leaves_the_issue_read() {
        let rig = Rig::synced().await;
        rig.seen_long_ago(NW_1042).await;

        rig.comment(NW_1042, "Checked on staging").await.unwrap();

        rig.assert_read_up_to_updated(NW_1042).await;
    }

    #[tokio::test]
    async fn an_issue_marked_unread_by_hand_stays_unread_after_a_write() {
        let rig = Rig::synced().await;
        mark_unread(&rig.ctx.db, NW_1042.into(), None)
            .await
            .unwrap();

        rig.move_to(NW_1042, DONE).await;

        assert!(rig.is_unread(NW_1042).await);
    }

    #[tokio::test]
    async fn a_write_never_moves_the_seen_time_back() {
        let rig = Rig::synced().await;
        let far_ahead = "2999-01-01T00:00:00.000Z";
        rig.ctx
            .db
            .write(move |tx, _| {
                tx.execute(
                    "UPDATE issue_seen SET seen_at = ?2 WHERE issue_id = ?1",
                    rusqlite::params![NW_1042, far_ahead],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        rig.move_to(NW_1042, DONE).await;

        assert_eq!(rig.seen_and_updated(NW_1042).await.0, far_ahead);
    }
}

mod setting_the_priority {
    use super::*;

    fn priority_outcome(done: &str, back_to: Option<&str>) -> WriteOutcome {
        WriteOutcome {
            done: done.into(),
            reverse: back_to.map(|old| Reverse {
                label: format!("Change back to {old}"),
                change: Change::Priority { name: old.into() },
            }),
        }
    }

    #[tokio::test]
    async fn shows_in_the_cache_and_in_jira_and_offers_the_old_priority_back() {
        let rig = Rig::synced().await;

        let done = rig.set_priority(NW_1042, "Low").await;

        assert_eq!(
            done,
            priority_outcome("Set NW-1042 to Low priority", Some("High"))
        );
        assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("Low"));
        assert_eq!(rig.jira_priority(NW_1042).await.as_deref(), Some("Low"));
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.priorities_sent(), 1);
    }

    #[tokio::test]
    async fn is_still_shown_after_the_next_sync() {
        let rig = Rig::synced().await;
        rig.set_priority(NW_1042, "Lowest").await;

        rig.sync().await;

        assert_eq!(
            rig.cached_priority(NW_1042).await.as_deref(),
            Some("Lowest")
        );
    }

    #[tokio::test]
    async fn the_reverse_sets_the_old_priority_back() {
        let rig = Rig::synced().await;
        let done = rig.set_priority(NW_1042, "Low").await;

        let back = done.reverse.unwrap().change;
        let Change::Priority { name } = back else {
            panic!("expected a priority change, got {back:?}");
        };
        rig.set_priority(NW_1042, &name).await;

        assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("High"));
        assert_eq!(rig.jira_priority(NW_1042).await.as_deref(), Some("High"));
    }

    #[tokio::test]
    async fn an_issue_without_a_priority_gets_no_reverse() {
        let rig = Rig::synced().await;
        rig.ctx
            .db
            .write(|tx, _| {
                tx.execute(
                    "UPDATE cache.issue SET priority = NULL WHERE id = ?1",
                    [NW_1042],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        let done = rig.set_priority(NW_1042, "Low").await;

        assert_eq!(done, priority_outcome("Set NW-1042 to Low priority", None));
        assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("Low"));
    }

    #[tokio::test]
    async fn the_current_priority_sends_nothing_and_offers_no_reverse() {
        let rig = Rig::synced().await;

        let done = rig.set_priority(NW_1042, "High").await;

        assert_eq!(
            done,
            priority_outcome("NW-1042 already has High priority", None)
        );
        assert_eq!(rig.priorities_sent(), 0);
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn a_priority_the_edit_screen_does_not_offer_is_refused_and_nothing_changes() {
        let rig = Rig::synced().await;

        let err = rig.try_set_priority(NW_1042, "Urgent").await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.priorities_sent(), 0);
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("High"));
        assert_eq!(rig.jira_priority(NW_1042).await.as_deref(), Some("High"));
    }

    #[tokio::test]
    async fn with_changes_off_nothing_is_sent_or_changed() {
        let rig = Rig::synced().await;
        rig.ctx
            .settings
            .edit(|s| s.behaviour.allow_jira_changes = false)
            .await
            .unwrap();

        let err = rig.try_set_priority(NW_1042, "Low").await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.priorities_sent(), 0);
        assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("High"));
    }

    #[tokio::test]
    async fn a_write_jira_refuses_puts_the_old_priority_back_and_returns_the_error() {
        let rig = Rig::synced().await;
        rig.fake
            .answer_next_write(400, json!({ "errorMessages": ["Not allowed"] }));

        let err = rig.try_set_priority(NW_1042, "Low").await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("High"));
        assert_eq!(rig.jira_priority(NW_1042).await.as_deref(), Some("High"));
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.priorities_sent(), 1);
    }

    #[tokio::test]
    async fn a_second_change_while_one_is_on_its_way_is_refused_and_sends_nothing() {
        let rig = Rig::synced().await;
        rig.start_pending_priority(NW_1042, "Lowest").await;

        let err = rig.try_set_priority(NW_1042, "Low").await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.priorities_sent(), 0);
        assert_eq!(
            rig.cached_priority(NW_1042).await.as_deref(),
            Some("Lowest")
        );
    }

    #[tokio::test]
    async fn the_sync_keeps_the_pending_priority_over_what_jira_sent_and_remembers_jiras_as_the_base()
     {
        let rig = Rig::synced().await;
        rig.start_pending_priority(NW_1049, "Low").await;
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["priority"] = json!({ "id": "3", "name": "Medium" })
        });

        rig.sync().await;

        assert_eq!(rig.cached_priority(NW_1049).await.as_deref(), Some("Low"));
        let base: Option<String> = rig
            .ctx
            .db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT base FROM cache.pending_write WHERE issue_id = ?1 AND field = 'priority'",
                    [NW_1049],
                    |row| row.get(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(base.as_deref(), Some("Medium"));
    }

    #[tokio::test]
    async fn the_sync_takes_what_jira_sent_once_the_write_is_forgotten() {
        let rig = Rig::synced().await;
        rig.start_pending_priority(NW_1049, "Low").await;
        rig.ctx.db.write(|tx, _| forget_all(tx)).await.unwrap();
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["priority"] = json!({ "id": "3", "name": "Medium" })
        });

        rig.sync().await;

        assert_eq!(
            rig.cached_priority(NW_1049).await.as_deref(),
            Some("Medium")
        );
    }
}

mod the_priority_menu {
    use super::*;

    async fn menu(rig: &Rig) -> AppResult<Value> {
        let source = rig.ctx.source().unwrap();
        priorities(&rig.ctx.db, &*source, NW_1042.into())
            .await
            .map(|menu| serde_json::to_value(menu).unwrap())
    }

    #[tokio::test]
    async fn offers_the_priorities_of_the_edit_screen_in_order_with_their_levels_and_marks_the_current()
     {
        let rig = Rig::synced().await;

        assert_eq!(
            menu(&rig).await.unwrap(),
            json!({
                "options": [
                    { "name": "Highest", "level": 1 },
                    { "name": "High", "level": 2 },
                    { "name": "Medium", "level": 3 },
                    { "name": "Low", "level": 4 },
                    { "name": "Lowest", "level": 5 },
                ],
                "current": "High",
            })
        );
    }

    #[tokio::test]
    async fn shows_the_new_priority_as_current_after_a_priority_write() {
        let rig = Rig::synced().await;
        rig.set_priority(NW_1042, "Lowest").await;

        let menu = menu(&rig).await.unwrap();

        assert_eq!(menu["current"], "Lowest");
    }

    #[tokio::test]
    async fn has_no_current_for_an_issue_without_a_priority() {
        let rig = Rig::synced().await;
        rig.ctx
            .db
            .write(|tx, _| {
                tx.execute(
                    "UPDATE cache.issue SET priority = NULL WHERE id = ?1",
                    [NW_1042],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        let menu = menu(&rig).await.unwrap();

        assert_eq!(menu["current"], Value::Null);
        assert_eq!(menu["options"].as_array().unwrap().len(), 5);
    }

    #[tokio::test]
    async fn is_an_error_when_jira_refuses_the_edit_screen() {
        let rig = Rig::synced().await;
        rig.fake
            .answer_next(400, json!({ "errorMessages": ["Not allowed"] }));

        menu(&rig).await.unwrap_err();
    }
}

mod renaming {
    use super::*;

    const OLD_TITLE: &str = "Checkout: address validation for DACH postcodes";
    const NEW_TITLE: &str = "Checkout: postcode validation for DACH addresses";
    const REFUSAL: &str = "A title is one line of 1 to 255 characters";

    impl Rig {
        async fn cached_title(&self, issue_id: &str) -> String {
            let id = issue_id.to_string();
            self.ctx
                .db
                .read(move |conn| {
                    Ok(conn.query_row(
                        "SELECT summary FROM cache.issue WHERE id = ?1",
                        [id],
                        |row| row.get(0),
                    )?)
                })
                .await
                .unwrap()
        }

        async fn jira_title(&self, issue_id: &str) -> String {
            let source = self.ctx.source().unwrap();
            let found = source
                .fetch(&[issue_id.to_string()], &["summary".to_string()])
                .await
                .unwrap();
            found.issues[0].fields["summary"]
                .as_str()
                .unwrap()
                .to_string()
        }

        async fn try_rename(&self, issue_id: &str, text: &str) -> AppResult<WriteOutcome> {
            let source = self.ctx.source().unwrap();
            apply(
                &self.ctx.db,
                &*source,
                &self.settings(),
                issue_id.into(),
                Change::Title { text: text.into() },
            )
            .await
        }

        async fn rename(&self, issue_id: &str, text: &str) -> WriteOutcome {
            self.try_rename(issue_id, text).await.unwrap()
        }

        fn edits_sent(&self) -> usize {
            self.fake
                .requests()
                .iter()
                .filter(|r| {
                    r.method == "PUT"
                        && r.path.contains("/issue/")
                        && !r.path.ends_with("/assignee")
                })
                .count()
        }

        /// A rename on its way: the row already shows `value`, as `writes::apply` leaves it.
        async fn start_pending_title(&self, issue_id: &'static str, value: &'static str) {
            self.ctx
                .db
                .write(move |tx, _| {
                    let base: String = tx.query_row(
                        "SELECT summary FROM cache.issue WHERE id = ?1",
                        [issue_id],
                        |row| row.get(0),
                    )?;
                    tx.execute(
                        "INSERT INTO cache.pending_write (issue_id, field, value, base, created_at)
                         VALUES (?1, 'title', ?2, ?3, '2026-10-28T11:00:00.000Z')",
                        rusqlite::params![issue_id, value, base],
                    )?;
                    tx.execute(
                        "UPDATE cache.issue SET summary = ?2 WHERE id = ?1",
                        rusqlite::params![issue_id, value],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
    }

    fn title_outcome(done: &str, back_to: Option<&str>) -> WriteOutcome {
        WriteOutcome {
            done: done.into(),
            reverse: back_to.map(|old| Reverse {
                label: format!("Change back to “{old}”"),
                change: Change::Title { text: old.into() },
            }),
        }
    }

    fn invalid_input(result: AppResult<WriteOutcome>) -> String {
        match result {
            Err(AppError::InvalidInput(message)) => message,
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn shows_the_new_title_in_the_cache_and_in_jira_and_offers_the_old_one_back() {
        let rig = Rig::synced().await;
        assert_eq!(rig.cached_title(NW_1042).await, OLD_TITLE);

        let done = rig.rename(NW_1042, NEW_TITLE).await;

        assert_eq!(done, title_outcome("Renamed NW-1042", Some(OLD_TITLE)));
        assert_eq!(rig.cached_title(NW_1042).await, NEW_TITLE);
        assert_eq!(rig.jira_title(NW_1042).await, NEW_TITLE);
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.recorded_changes().await, 0);
        assert_eq!(rig.edits_sent(), 1);
    }

    #[tokio::test]
    async fn sends_only_the_summary_trimmed() {
        let rig = Rig::synced().await;

        rig.rename(NW_1042, &format!("  {NEW_TITLE}\n")).await;

        let edit = rig
            .fake
            .requests()
            .into_iter()
            .find(|r| r.method == "PUT" && r.path.ends_with("/issue/101042"))
            .unwrap();
        assert_eq!(edit.body, json!({ "fields": { "summary": NEW_TITLE } }));
        assert_eq!(rig.cached_title(NW_1042).await, NEW_TITLE);
    }

    #[tokio::test]
    async fn shows_the_new_title_at_once_while_it_is_on_its_way() {
        let rig = Rig::synced().await;
        rig.fake.set_latency(std::time::Duration::from_millis(100));

        let (result, seen) = tokio::join!(rig.try_rename(NW_1042, NEW_TITLE), async {
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                loop {
                    if rig.pending_writes().await == 1 {
                        return rig.cached_title(NW_1042).await;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
            })
            .await
            .expect("the pending title never showed")
        });

        result.unwrap();
        assert_eq!(seen, NEW_TITLE);
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn is_still_shown_after_the_next_sync() {
        let rig = Rig::synced().await;
        rig.rename(NW_1042, NEW_TITLE).await;

        rig.sync().await;

        assert_eq!(rig.cached_title(NW_1042).await, NEW_TITLE);
    }

    #[tokio::test]
    async fn the_reverse_sets_the_old_title_back() {
        let rig = Rig::synced().await;
        let done = rig.rename(NW_1042, NEW_TITLE).await;

        let back = done.reverse.unwrap().change;
        let Change::Title { text } = back else {
            panic!("expected a title change, got {back:?}");
        };
        let undone = rig.rename(NW_1042, &text).await;

        assert_eq!(undone, title_outcome("Renamed NW-1042", Some(NEW_TITLE)));
        assert_eq!(rig.cached_title(NW_1042).await, OLD_TITLE);
        assert_eq!(rig.jira_title(NW_1042).await, OLD_TITLE);
    }

    #[tokio::test]
    async fn the_outcome_serializes_for_the_page_in_camel_case() {
        let rig = Rig::synced().await;

        let done = rig.rename(NW_1042, NEW_TITLE).await;

        assert_eq!(
            serde_json::to_value(&done).unwrap(),
            json!({
                "done": "Renamed NW-1042",
                "reverse": {
                    "label": format!("Change back to “{OLD_TITLE}”"),
                    "change": { "kind": "title", "text": OLD_TITLE },
                },
            })
        );
    }

    #[tokio::test]
    async fn the_current_title_sends_nothing_and_offers_no_reverse() {
        let rig = Rig::synced().await;
        for text in [OLD_TITLE, &format!("  {OLD_TITLE} ")] {
            let done = rig.rename(NW_1042, text).await;

            assert_eq!(
                done,
                title_outcome("NW-1042 already has this title", None),
                "{text:?}"
            );
        }
        assert_eq!(rig.edits_sent(), 0);
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn a_blank_or_oversized_title_is_refused_and_nothing_is_sent() {
        let rig = Rig::synced().await;
        let requests_before = rig.fake.requests().len();
        for text in [
            String::new(),
            "   ".into(),
            "\n\t".into(),
            "Two\nlines".into(),
            "a".repeat(256),
        ] {
            let message = invalid_input(rig.try_rename(NW_1042, &text).await);

            assert_eq!(message, REFUSAL, "{text:?}");
        }
        assert_eq!(rig.fake.requests().len(), requests_before);
        assert_eq!(rig.cached_title(NW_1042).await, OLD_TITLE);
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn a_title_of_255_characters_is_sent() {
        let rig = Rig::synced().await;
        let longest = "ä".repeat(255);

        rig.rename(NW_1042, &longest).await;

        assert_eq!(rig.cached_title(NW_1042).await, longest);
        assert_eq!(rig.jira_title(NW_1042).await, longest);
    }

    #[tokio::test]
    async fn with_changes_off_nothing_is_sent_or_changed() {
        let rig = Rig::synced().await;
        rig.ctx
            .settings
            .edit(|s| s.behaviour.allow_jira_changes = false)
            .await
            .unwrap();

        let err = rig.try_rename(NW_1042, NEW_TITLE).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.edits_sent(), 0);
        assert_eq!(rig.cached_title(NW_1042).await, OLD_TITLE);
    }

    #[tokio::test]
    async fn with_the_change_switched_off_it_is_refused_naming_the_label_and_changes_nothing() {
        let rig = Rig::synced().await;
        rig.ctx
            .settings
            .edit(|s| s.behaviour.disabled_changes = vec![Capability::ChangeTitle])
            .await
            .unwrap();

        let message = invalid_input(rig.try_rename(NW_1042, NEW_TITLE).await);

        assert_eq!(
            message,
            "“Change the title” is switched off in Settings → Connection."
        );
        assert_eq!(rig.edits_sent(), 0);
        assert_eq!(rig.cached_title(NW_1042).await, OLD_TITLE);
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn an_issue_missing_from_the_cache_is_not_found() {
        let rig = Rig::synced().await;

        let err = rig.try_rename("999999", NEW_TITLE).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(rig.edits_sent(), 0);
    }

    #[tokio::test]
    async fn a_write_jira_refuses_puts_the_old_title_back_and_returns_the_error() {
        let rig = Rig::synced().await;
        rig.fake
            .answer_next_write(400, json!({ "errorMessages": ["Not allowed"] }));

        let err = rig.try_rename(NW_1042, NEW_TITLE).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.cached_title(NW_1042).await, OLD_TITLE);
        assert_eq!(rig.jira_title(NW_1042).await, OLD_TITLE);
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(rig.edits_sent(), 1);
    }

    #[tokio::test]
    async fn a_second_rename_while_one_is_on_its_way_is_refused_and_sends_nothing() {
        let rig = Rig::synced().await;
        rig.start_pending_title(NW_1042, "Checkout: another name")
            .await;

        let err = rig.try_rename(NW_1042, NEW_TITLE).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.edits_sent(), 0);
        assert_eq!(rig.cached_title(NW_1042).await, "Checkout: another name");
    }

    #[tokio::test]
    async fn the_sync_keeps_the_pending_title_over_what_jira_sent_and_remembers_jiras_as_the_base()
    {
        let rig = Rig::synced().await;
        rig.start_pending_title(NW_1049, "Checkout: the name I chose")
            .await;
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["summary"] = json!("Checkout: the name someone else chose")
        });

        rig.sync().await;

        assert_eq!(
            rig.cached_title(NW_1049).await,
            "Checkout: the name I chose"
        );
        let base: Option<String> = rig
            .ctx
            .db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT base FROM cache.pending_write WHERE issue_id = ?1 AND field = 'title'",
                    [NW_1049],
                    |row| row.get(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(
            base.as_deref(),
            Some("Checkout: the name someone else chose")
        );
    }

    #[tokio::test]
    async fn the_sync_takes_what_jira_sent_once_the_write_is_forgotten() {
        let rig = Rig::synced().await;
        rig.start_pending_title(NW_1049, "Checkout: the name I chose")
            .await;
        rig.ctx.db.write(|tx, _| forget_all(tx)).await.unwrap();
        rig.fake.set_now(demo_now() + chrono::Duration::hours(2));
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["summary"] = json!("Checkout: the name someone else chose")
        });

        rig.sync().await;

        assert_eq!(
            rig.cached_title(NW_1049).await,
            "Checkout: the name someone else chose"
        );
    }
}

mod the_fake_jiras_edits {
    use super::*;

    async fn put(rig: &Rig, body: Value) -> u16 {
        reqwest::Client::new()
            .put(format!("{}rest/api/3/issue/{NW_1042}", rig.fake.url()))
            .header("authorization", "Basic ZGVtbzpkZW1v")
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .unwrap()
            .status()
            .as_u16()
    }

    async fn jira_fields(rig: &Rig) -> (String, Option<String>) {
        let source = rig.ctx.source().unwrap();
        let found = source
            .fetch(
                &[NW_1042.to_string()],
                &["summary".to_string(), "priority".to_string()],
            )
            .await
            .unwrap();
        let fields = &found.issues[0].fields;
        (
            fields["summary"].as_str().unwrap().to_string(),
            fields["priority"]["name"].as_str().map(String::from),
        )
    }

    #[tokio::test]
    async fn a_summary_edit_is_stored() {
        let rig = Rig::synced().await;

        let status = put(
            &rig,
            json!({ "fields": { "summary": "Checkout: new copy" } }),
        )
        .await;

        assert_eq!(status, 204);
        assert_eq!(
            jira_fields(&rig).await,
            ("Checkout: new copy".to_string(), Some("High".to_string()))
        );
    }

    #[tokio::test]
    async fn two_fields_in_one_request_or_a_bad_summary_are_a_400_that_changes_nothing() {
        let rig = Rig::synced().await;
        let before = jira_fields(&rig).await;
        for (label, body) in [
            (
                "two fields",
                json!({ "fields": { "summary": "Checkout: new copy", "priority": { "name": "Low" } } }),
            ),
            ("a number", json!({ "fields": { "summary": 5 } })),
            ("null", json!({ "fields": { "summary": null } })),
            ("blank", json!({ "fields": { "summary": "  " } })),
            ("no fields", json!({ "fields": {} })),
            ("another field", json!({ "fields": { "labels": ["x"] } })),
        ] {
            assert_eq!(put(&rig, body).await, 400, "{label}");
        }
        assert_eq!(jira_fields(&rig).await, before);
    }
}

mod adding_a_web_link {
    use super::*;

    const SPEC_URL: &str = "https://docs.example.com/checkout/spec";

    fn invalid_input(result: AppResult<WriteOutcome>) {
        match result {
            Err(AppError::InvalidInput(_)) => {}
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    impl Rig {
        async fn add_link(
            &self,
            issue_id: &str,
            url: &str,
            title: Option<&str>,
        ) -> AppResult<WriteOutcome> {
            let source = self.ctx.source().unwrap();
            add_web_link(
                &self.ctx.db,
                &*source,
                &self.settings(),
                issue_id.into(),
                url.into(),
                title.map(String::from),
            )
            .await
        }

        /// The issue with one known link, cached as a sync of its web links leaves it.
        async fn with_one_link(&self, issue_id: &str, issue_key: &str) {
            self.fake.set_remote_links(
                issue_key,
                vec![json!({
                    "id": 70_001,
                    "object": { "url": "https://example.com/brief", "title": "Brief" }
                })],
            );
            let source = self.ctx.source().unwrap();
            issue_sync_web_links(&self.ctx.db, &*source, issue_id.into(), demo_now())
                .await
                .unwrap();
        }

        async fn shown_links(&self, issue_id: &str) -> Vec<WebLink> {
            issue_web_links(&self.ctx.db, issue_id.into())
                .await
                .unwrap()
        }

        /// `(id, title)` of the links Jira (the fake) holds, in its order.
        async fn jira_links(&self, issue_key: &str) -> Vec<(String, String)> {
            let source = self.ctx.source().unwrap();
            source
                .remote_links(issue_key)
                .await
                .unwrap()
                .into_iter()
                .map(|link| (link.id.to_string(), link.object.title.unwrap_or_default()))
                .collect()
        }

        fn links_posted(&self) -> usize {
            self.fake
                .requests()
                .iter()
                .filter(|r| r.method == "POST" && r.path.ends_with("/remotelink"))
                .count()
        }
    }

    #[tokio::test]
    async fn puts_the_link_last_in_jira_and_in_the_cache_at_once_under_jiras_id() {
        let rig = Rig::synced().await;
        rig.with_one_link(NW_1042, "NW-1042").await;

        let done = rig
            .add_link(NW_1042, SPEC_URL, Some("  Checkout spec "))
            .await
            .unwrap();

        assert_eq!(
            done,
            WriteOutcome {
                done: "Added the link to NW-1042".into(),
                reverse: None
            }
        );
        let jira = rig.jira_links("NW-1042").await;
        assert_eq!(jira.len(), 2);
        assert_eq!(jira[0], ("70001".to_string(), "Brief".to_string()));
        assert_eq!(jira[1].1, "Checkout spec");
        let shown = rig.shown_links(NW_1042).await;
        assert_eq!(
            shown,
            [
                WebLink {
                    id: "70001".into(),
                    url: "https://example.com/brief".into(),
                    title: "Brief".into(),
                    host: Some("example.com".into()),
                    kind: WebLinkKind::Link,
                    reference: None,
                    openable: true,
                },
                WebLink {
                    id: jira[1].0.clone(),
                    url: SPEC_URL.into(),
                    title: "Checkout spec".into(),
                    host: Some("docs.example.com".into()),
                    kind: WebLinkKind::Link,
                    reference: None,
                    openable: true,
                },
            ]
        );
        assert_eq!(rig.links_posted(), 1);
        assert_eq!(rig.recorded_changes().await, 0);
    }

    #[tokio::test]
    async fn a_blank_title_takes_the_pull_request_reference_and_keeps_the_host_beside_it() {
        for title in [None, Some(""), Some("  \t ")] {
            let rig = Rig::synced().await;

            rig.add_link(NW_1042, "https://github.com/example/shop/pull/512", title)
                .await
                .unwrap();

            let shown = rig.shown_links(NW_1042).await;
            let added = shown.last().unwrap();
            assert_eq!(added.title, "shop#512", "{title:?}");
            assert_eq!(added.reference, None, "{title:?}");
            assert_eq!(added.host.as_deref(), Some("github.com"), "{title:?}");
            assert_eq!(added.kind, WebLinkKind::PullRequest, "{title:?}");
            let jira = rig.jira_links("NW-1042").await;
            assert_eq!(jira.last().unwrap().1, "shop#512", "{title:?}");
        }
    }

    #[tokio::test]
    async fn a_blank_title_on_another_address_takes_the_address() {
        let rig = Rig::synced().await;

        rig.add_link(NW_1042, SPEC_URL, None).await.unwrap();

        let shown = rig.shown_links(NW_1042).await;
        assert_eq!(shown.last().unwrap().title, SPEC_URL);
        assert_eq!(rig.jira_links("NW-1042").await.last().unwrap().1, SPEC_URL);
    }

    #[tokio::test]
    async fn an_address_longer_than_a_title_may_be_is_cut_to_255_characters_as_the_title() {
        let rig = Rig::synced().await;
        let url = format!("https://docs.example.com/{}", "a".repeat(300));

        rig.add_link(NW_1042, &url, None).await.unwrap();

        let expected: String = url.chars().take(255).collect();
        let shown = rig.shown_links(NW_1042).await;
        assert_eq!(shown.last().unwrap().title, expected);
        assert_eq!(shown.last().unwrap().url, url);
        assert_eq!(rig.jira_links("NW-1042").await.last().unwrap().1, expected);
    }

    #[tokio::test]
    async fn an_address_that_is_not_web_is_refused_and_nothing_is_sent() {
        let rig = Rig::synced().await;
        let requests_before = rig.fake.requests().len();
        for url in [
            "",
            "   ",
            "docs.example.com/spec",
            "ftp://docs.example.com/spec",
            "javascript:alert(1)",
            "https://",
        ] {
            invalid_input(rig.add_link(NW_1042, url, Some("Spec")).await);
            assert_eq!(rig.shown_links(NW_1042).await, [], "{url:?}");
        }
        assert_eq!(rig.fake.requests().len(), requests_before);
    }

    #[tokio::test]
    async fn a_title_over_255_characters_or_with_control_characters_is_refused_and_nothing_is_sent()
    {
        let rig = Rig::synced().await;
        let requests_before = rig.fake.requests().len();
        let too_long = "a".repeat(256);
        for title in [too_long.as_str(), "Spec\nsheet", "Spec\u{1b}[31m"] {
            invalid_input(rig.add_link(NW_1042, SPEC_URL, Some(title)).await);
        }
        assert_eq!(rig.fake.requests().len(), requests_before);
        assert_eq!(rig.shown_links(NW_1042).await, []);
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

        let result = rig.add_link(NW_1042, SPEC_URL, Some("Spec")).await;

        invalid_input(result);
        assert_eq!(rig.fake.requests().len(), requests_before);
        assert_eq!(rig.shown_links(NW_1042).await, []);
    }

    #[tokio::test]
    async fn an_issue_missing_from_the_cache_is_not_found_and_nothing_is_sent() {
        let rig = Rig::synced().await;

        let err = rig
            .add_link("999999", SPEC_URL, Some("Spec"))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(rig.links_posted(), 0);
    }

    #[tokio::test]
    async fn a_link_jira_refuses_returns_the_error_and_stores_nothing() {
        let rig = Rig::synced().await;
        rig.with_one_link(NW_1042, "NW-1042").await;
        let before = rig.shown_links(NW_1042).await;
        rig.fake
            .answer_next_write(400, json!({ "errorMessages": ["Not allowed"] }));

        let err = rig
            .add_link(NW_1042, SPEC_URL, Some("Spec"))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(rig.shown_links(NW_1042).await, before);
        assert_eq!(rig.links_posted(), 1);
        assert_eq!(rig.jira_links("NW-1042").await.len(), 1);
    }
}

mod dropping {
    use super::*;

    const NOT_ALLOWED: &str = "Jira's workflow doesn't allow this move for the issue";

    use taskboard_lib::views::groups::GroupMark;
    use taskboard_lib::views::rows::person;
    use taskboard_lib::writes::DropTarget;

    const CHANGES_OFF: &str = "Changes to Jira are off. Turn them on in Settings → Connection.";
    const NO_FIELD: &str =
        "Dragging changes the status, the assignee or the priority; group the view by one of them.";
    const ANNA: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e04";

    fn noted(note: &str) -> DropTargets {
        DropTargets {
            targets: Vec::new(),
            note: Some(note.into()),
        }
    }

    fn offering(targets: Vec<DropTarget>) -> DropTargets {
        DropTargets {
            targets,
            note: None,
        }
    }

    fn status_target(id: &str, name: &str, style: Style) -> DropTarget {
        DropTarget {
            key: id.into(),
            label: name.into(),
            action: format!("Set status to {name}"),
            mark: Some(GroupMark::Status { style }),
        }
    }

    fn type_target(key: &str, style: Style, action: &str) -> DropTarget {
        DropTarget {
            key: key.into(),
            label: style.name().into(),
            action: action.into(),
            mark: Some(GroupMark::Status { style }),
        }
    }

    fn person_target(account_id: &str, name: &str, action: &str) -> DropTarget {
        DropTarget {
            key: account_id.into(),
            label: name.into(),
            action: action.into(),
            mark: Some(GroupMark::Person {
                person: person(account_id, name, Some(ALEX)),
            }),
        }
    }

    fn unassigned_target() -> DropTarget {
        DropTarget {
            key: String::new(),
            label: "Unassigned".into(),
            action: "Unassign".into(),
            mark: None,
        }
    }

    fn priority_target(name: &str, level: u8) -> DropTarget {
        DropTarget {
            key: name.into(),
            label: name.into(),
            action: format!("Set priority to {name}"),
            mark: Some(GroupMark::Priority { level }),
        }
    }

    async fn targets(rig: &Rig, group_by: GroupBy) -> DropTargets {
        let source = rig.ctx.source().unwrap();
        drop_targets(
            &rig.ctx.db,
            &*source,
            &rig.settings(),
            NW_1042.into(),
            group_by,
        )
        .await
        .unwrap()
    }

    async fn try_drop(
        rig: &Rig,
        group_by: GroupBy,
        key: &str,
    ) -> taskboard_lib::error::AppResult<DropOutcome> {
        let source = rig.ctx.source().unwrap();
        drop_on(
            &rig.ctx.db,
            &*source,
            &rig.settings(),
            NW_1042.into(),
            group_by,
            key.into(),
        )
        .await
    }

    async fn drop_done(rig: &Rig, group_by: GroupBy, key: &str) -> WriteOutcome {
        match try_drop(rig, group_by, key).await.unwrap() {
            DropOutcome::Done { outcome } => outcome,
            other => panic!("expected a done drop, got {other:?}"),
        }
    }

    fn refused(result: taskboard_lib::error::AppResult<DropOutcome>) -> String {
        match result {
            Err(AppError::InvalidInput(message)) => message,
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    async fn nothing_written(rig: &Rig) {
        assert_eq!(rig.transitions_posted(), 0);
        assert_eq!(rig.assignments_sent(), 0);
        assert_eq!(rig.priorities_sent(), 0);
        assert_eq!(rig.pending_writes().await, 0);
        assert_eq!(
            rig.cached_status(NW_1042).await,
            (IN_PROGRESS.to_string(), "indeterminate".to_string())
        );
        assert_eq!(rig.cached_assignee(NW_1042).await.as_deref(), Some(ALEX));
        assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("High"));
    }

    #[tokio::test]
    async fn with_changes_off_no_group_takes_the_issue_and_jira_is_not_asked() {
        let rig = Rig::synced().await;
        rig.ctx
            .settings
            .edit(|s| s.behaviour.allow_jira_changes = false)
            .await
            .unwrap();
        let requests_before = rig.fake.requests().len();

        for group_by in [
            GroupBy::Assignee,
            GroupBy::Status,
            GroupBy::StatusType,
            GroupBy::Priority,
        ] {
            assert_eq!(
                targets(&rig, group_by).await,
                noted(CHANGES_OFF),
                "{group_by:?}"
            );
        }

        assert_eq!(rig.fake.requests().len(), requests_before);
    }

    #[tokio::test]
    async fn grouped_by_assignee_the_targets_are_the_spaces_team_with_me_first_then_unassigned() {
        let rig = Rig::synced().await;
        let requests_before = rig.fake.requests().len();

        let found = targets(&rig, GroupBy::Assignee).await;

        assert_eq!(
            found,
            offering(vec![
                person_target(ALEX, "Alex Morgan", "Assign to me"),
                person_target(ANNA, "Anna Lutz", "Assign to Anna Lutz"),
                person_target(JONAS, "Jonas Reiter", "Assign to Jonas Reiter"),
                person_target(MARTA, "Marta Klein", "Assign to Marta Klein"),
                unassigned_target(),
            ])
        );
        assert_eq!(rig.fake.requests().len(), requests_before);
    }

    #[tokio::test]
    async fn grouped_by_assignee_people_switched_off_in_the_space_are_left_out() {
        let rig = Rig::synced().await;
        taskboard_lib::team::space_set_member(
            &rig.ctx.db,
            NW_PROJECT.into(),
            JONAS.into(),
            false,
            Some(ALEX.into()),
        )
        .await
        .unwrap();

        let found = targets(&rig, GroupBy::Assignee).await;

        let keys: Vec<&str> = found.targets.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(keys, [ALEX, ANNA, MARTA, ""]);
    }

    #[tokio::test]
    async fn grouped_by_status_the_targets_are_the_statuses_of_the_moves_without_a_screen() {
        let rig = Rig::synced().await;

        assert_eq!(
            targets(&rig, GroupBy::Status).await,
            offering(vec![
                status_target(TO_DO, "To Do", Style::Todo),
                status_target(IN_REVIEW, "In Review", Style::Review),
                status_target(BLOCKED, "Blocked", Style::Blocked),
                status_target(DONE, "Done", Style::Done),
            ])
        );

        rig.fake.require_screen(BLOCKED);

        assert_eq!(
            targets(&rig, GroupBy::Status).await,
            offering(vec![
                status_target(TO_DO, "To Do", Style::Todo),
                status_target(IN_REVIEW, "In Review", Style::Review),
                status_target(DONE, "Done", Style::Done),
            ])
        );
    }

    #[tokio::test]
    async fn grouped_by_status_type_the_targets_are_the_distinct_styles_of_those_moves_in_style_order()
     {
        let rig = Rig::synced().await;

        assert_eq!(
            targets(&rig, GroupBy::StatusType).await,
            offering(vec![
                type_target("review", Style::Review, "Set status to In Review"),
                type_target("blocked", Style::Blocked, "Set status to Blocked"),
                type_target("todo", Style::Todo, "Set status to To Do"),
                type_target("done", Style::Done, "Set status to Done"),
            ])
        );

        rig.fake.require_screen(BLOCKED);
        assert_eq!(
            targets(&rig, GroupBy::StatusType).await,
            offering(vec![
                type_target("review", Style::Review, "Set status to In Review"),
                type_target("todo", Style::Todo, "Set status to To Do"),
                type_target("done", Style::Done, "Set status to Done"),
            ])
        );
    }

    #[tokio::test]
    async fn a_status_type_with_several_statuses_names_them_all_in_its_action() {
        let rig = Rig::synced().await;
        for status in [DONE, BLOCKED] {
            status_style_set(
                &rig.ctx.db,
                NW_PROJECT.into(),
                status.into(),
                Some(Style::Review),
            )
            .await
            .unwrap();
        }

        assert_eq!(
            targets(&rig, GroupBy::StatusType).await,
            offering(vec![
                type_target(
                    "review",
                    Style::Review,
                    "Set status to In Review, Blocked or Done"
                ),
                type_target("todo", Style::Todo, "Set status to To Do"),
            ])
        );

        status_style_set(
            &rig.ctx.db,
            NW_PROJECT.into(),
            BLOCKED.into(),
            Some(Style::Blocked),
        )
        .await
        .unwrap();

        assert_eq!(
            targets(&rig, GroupBy::StatusType).await,
            offering(vec![
                type_target("review", Style::Review, "Set status to In Review or Done"),
                type_target("blocked", Style::Blocked, "Set status to Blocked"),
                type_target("todo", Style::Todo, "Set status to To Do"),
            ])
        );
    }

    #[tokio::test]
    async fn grouped_by_priority_the_targets_are_the_priorities_of_the_edit_screen() {
        let rig = Rig::synced().await;

        assert_eq!(
            targets(&rig, GroupBy::Priority).await,
            offering(vec![
                priority_target("Highest", 1),
                priority_target("High", 2),
                priority_target("Medium", 3),
                priority_target("Low", 4),
                priority_target("Lowest", 5),
            ])
        );
    }

    #[tokio::test]
    async fn other_groupings_take_no_drop_and_say_what_to_group_by() {
        let rig = Rig::synced().await;
        for group_by in [
            GroupBy::Project,
            GroupBy::Client,
            GroupBy::Epic,
            GroupBy::Release,
            GroupBy::Due,
            GroupBy::None,
        ] {
            assert_eq!(
                targets(&rig, group_by).await,
                noted(NO_FIELD),
                "{group_by:?}"
            );
        }
    }

    #[tokio::test]
    async fn the_targets_serialize_for_the_page_in_camel_case() {
        let rig = Rig::synced().await;

        let status = serde_json::to_value(targets(&rig, GroupBy::Status).await).unwrap();
        let none = serde_json::to_value(targets(&rig, GroupBy::Epic).await).unwrap();
        let assignee = serde_json::to_value(targets(&rig, GroupBy::Assignee).await).unwrap();
        let priority = serde_json::to_value(targets(&rig, GroupBy::Priority).await).unwrap();

        assert_eq!(
            status,
            json!({
                "targets": [
                    {
                        "key": TO_DO, "label": "To Do", "action": "Set status to To Do",
                        "mark": { "kind": "status", "style": "todo" },
                    },
                    {
                        "key": IN_REVIEW, "label": "In Review", "action": "Set status to In Review",
                        "mark": { "kind": "status", "style": "review" },
                    },
                    {
                        "key": BLOCKED, "label": "Blocked", "action": "Set status to Blocked",
                        "mark": { "kind": "status", "style": "blocked" },
                    },
                    {
                        "key": DONE, "label": "Done", "action": "Set status to Done",
                        "mark": { "kind": "status", "style": "done" },
                    },
                ],
                "note": null,
            })
        );
        assert_eq!(none, json!({ "targets": [], "note": NO_FIELD }));
        assert_eq!(assignee["targets"][0]["mark"]["kind"], "person");
        assert_eq!(
            assignee["targets"][0]["mark"]["person"]["accountId"],
            json!(ALEX)
        );
        assert_eq!(
            assignee["targets"][4],
            json!({ "key": "", "label": "Unassigned", "action": "Unassign", "mark": null })
        );
        assert_eq!(
            priority["targets"][0],
            json!({
                "key": "Highest", "label": "Highest", "action": "Set priority to Highest",
                "mark": { "kind": "priority", "level": 1 },
            })
        );
    }

    #[tokio::test]
    async fn on_an_assignee_group_assigns_to_that_account() {
        let rig = Rig::synced().await;

        let outcome = drop_done(&rig, GroupBy::Assignee, MARTA).await;

        assert_eq!(
            outcome,
            outcome_of(
                "Assigned NW-1042 to Marta Klein",
                "Change back to you",
                ALEX
            )
        );
        assert_eq!(rig.cached_assignee(NW_1042).await.as_deref(), Some(MARTA));
        assert_eq!(rig.jira_assignee(NW_1042).await.as_deref(), Some(MARTA));
    }

    fn outcome_of(done: &str, label: &str, back_to: &str) -> WriteOutcome {
        super::outcome(done, label, Some(back_to))
    }

    #[tokio::test]
    async fn on_the_unassigned_group_unassigns() {
        let rig = Rig::synced().await;

        let outcome = drop_done(&rig, GroupBy::Assignee, "").await;

        assert_eq!(
            outcome,
            outcome_of("Unassigned NW-1042", "Change back to you", ALEX)
        );
        assert_eq!(rig.cached_assignee(NW_1042).await, None);
        assert_eq!(rig.jira_assignee(NW_1042).await, None);
    }

    #[tokio::test]
    async fn on_a_priority_group_sets_that_priority() {
        let rig = Rig::synced().await;

        let outcome = drop_done(&rig, GroupBy::Priority, "Low").await;

        assert_eq!(
            outcome,
            WriteOutcome {
                done: "Set NW-1042 to Low priority".into(),
                reverse: Some(Reverse {
                    label: "Change back to High".into(),
                    change: Change::Priority {
                        name: "High".into()
                    },
                }),
            }
        );
        assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("Low"));
        assert_eq!(rig.jira_priority(NW_1042).await.as_deref(), Some("Low"));
    }

    #[tokio::test]
    async fn on_a_status_group_makes_the_move_into_that_status() {
        let rig = Rig::synced().await;

        let outcome = drop_done(&rig, GroupBy::Status, DONE).await;

        assert_eq!(outcome.done, "Moved NW-1042 to Done");
        assert_eq!(
            rig.cached_status(NW_1042).await,
            (DONE.to_string(), "done".to_string())
        );
        assert_eq!(rig.jira_status(NW_1042).await, DONE);
    }

    #[tokio::test]
    async fn on_a_status_jira_does_not_offer_or_only_with_a_screen_nothing_is_written() {
        let rig = Rig::synced().await;
        rig.fake.require_screen(BLOCKED);

        for (label, key) in [
            ("an unknown status", "99999"),
            ("the current status", IN_PROGRESS),
            ("a status behind a screen", BLOCKED),
        ] {
            let message = refused(try_drop(&rig, GroupBy::Status, key).await);

            assert_eq!(message, NOT_ALLOWED, "{label}");
        }
        nothing_written(&rig).await;
    }

    #[tokio::test]
    async fn on_a_status_type_with_one_status_makes_that_move() {
        let rig = Rig::synced().await;

        let outcome = drop_done(&rig, GroupBy::StatusType, "done").await;

        assert_eq!(outcome.done, "Moved NW-1042 to Done");
        assert_eq!(rig.jira_status(NW_1042).await, DONE);
    }

    #[tokio::test]
    async fn on_a_status_type_without_a_move_nothing_is_written() {
        let rig = Rig::synced().await;
        rig.fake.require_screen(BLOCKED);

        for (label, key) in [
            ("the issue's own type", "progress"),
            ("a type behind a screen", "blocked"),
            ("an unknown type", "nope"),
        ] {
            let message = refused(try_drop(&rig, GroupBy::StatusType, key).await);

            assert_eq!(message, NOT_ALLOWED, "{label}");
        }
        nothing_written(&rig).await;
    }

    #[tokio::test]
    async fn on_a_status_type_with_several_moves_the_choice_is_returned_and_nothing_is_written() {
        let rig = Rig::synced().await;
        status_style_set(
            &rig.ctx.db,
            NW_PROJECT.into(),
            BLOCKED.into(),
            Some(Style::Review),
        )
        .await
        .unwrap();

        let result = try_drop(&rig, GroupBy::StatusType, "review").await.unwrap();

        assert_eq!(
            result,
            DropOutcome::Choose {
                moves: vec![
                    option(IN_REVIEW, "In Review", Style::Review, false),
                    option(BLOCKED, "Blocked", Style::Review, false),
                ]
            }
        );
        nothing_written(&rig).await;
    }

    #[tokio::test]
    async fn the_outcomes_serialize_with_a_kind_tag() {
        let rig = Rig::synced().await;
        status_style_set(
            &rig.ctx.db,
            NW_PROJECT.into(),
            BLOCKED.into(),
            Some(Style::Review),
        )
        .await
        .unwrap();

        let choose = try_drop(&rig, GroupBy::StatusType, "review").await.unwrap();
        let done = try_drop(&rig, GroupBy::StatusType, "done").await.unwrap();

        let choose = serde_json::to_value(choose).unwrap();
        assert_eq!(choose["kind"], "choose");
        assert_eq!(choose["moves"][0]["statusName"], "In Review");
        let done = serde_json::to_value(done).unwrap();
        assert_eq!(done["kind"], "done");
        assert_eq!(done["outcome"]["done"], "Moved NW-1042 to Done");
    }

    #[tokio::test]
    async fn on_other_groupings_nothing_is_written() {
        let rig = Rig::synced().await;
        for group_by in [
            GroupBy::Project,
            GroupBy::Client,
            GroupBy::Epic,
            GroupBy::Release,
            GroupBy::Due,
            GroupBy::None,
        ] {
            let message = refused(try_drop(&rig, group_by, "x").await);

            assert_eq!(
                message, "Issues can't be moved between these groups",
                "{group_by:?}"
            );
        }
        nothing_written(&rig).await;
    }

    #[tokio::test]
    async fn with_changes_off_a_drop_writes_nothing() {
        let rig = Rig::synced().await;
        rig.ctx
            .settings
            .edit(|s| s.behaviour.allow_jira_changes = false)
            .await
            .unwrap();

        for (group_by, key) in [
            (GroupBy::Assignee, MARTA),
            (GroupBy::Status, DONE),
            (GroupBy::StatusType, "done"),
            (GroupBy::Priority, "Low"),
        ] {
            let err = try_drop(&rig, group_by, key).await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{group_by:?}");
        }
        nothing_written(&rig).await;
    }
}

mod a_kind_of_change_switched_off {
    use super::*;

    const SPEC_URL: &str = "https://docs.example.com/checkout/spec";

    async fn switch_off(rig: &Rig, capability: Capability) {
        rig.ctx
            .settings
            .edit(move |s| s.behaviour.disabled_changes = vec![capability])
            .await
            .unwrap();
    }

    fn off_message(label: &str) -> String {
        format!("“{label}” is switched off in Settings → Connection.")
    }

    fn refused<T: std::fmt::Debug>(result: AppResult<T>) -> String {
        match result {
            Err(AppError::InvalidInput(message)) => message,
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    async fn add_link(rig: &Rig) -> AppResult<WriteOutcome> {
        let source = rig.ctx.source().unwrap();
        add_web_link(
            &rig.ctx.db,
            &*source,
            &rig.settings(),
            NW_1042.into(),
            SPEC_URL.into(),
            Some("Spec".into()),
        )
        .await
    }

    fn links_posted(rig: &Rig) -> usize {
        rig.fake
            .requests()
            .iter()
            .filter(|r| r.method == "POST" && r.path.ends_with("/remotelink"))
            .count()
    }

    async fn drop_on_group(rig: &Rig, group_by: GroupBy, key: &str) -> AppResult<DropOutcome> {
        let source = rig.ctx.source().unwrap();
        drop_on(
            &rig.ctx.db,
            &*source,
            &rig.settings(),
            NW_1042.into(),
            group_by,
            key.into(),
        )
        .await
    }

    async fn targets(rig: &Rig, group_by: GroupBy) -> DropTargets {
        let source = rig.ctx.source().unwrap();
        drop_targets(
            &rig.ctx.db,
            &*source,
            &rig.settings(),
            NW_1042.into(),
            group_by,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn assigning_is_refused_naming_the_label_and_changes_nothing() {
        let rig = Rig::synced().await;
        switch_off(&rig, Capability::Assign).await;

        let message = refused(rig.try_apply(NW_1044, Some(ALEX)).await);

        assert_eq!(message, off_message("Change the assignee"));
        assert_eq!(rig.assignments_sent(), 0);
        assert_eq!(rig.cached_assignee(NW_1044).await.as_deref(), Some(MARTA));
        assert_eq!(rig.jira_assignee(NW_1044).await.as_deref(), Some(MARTA));
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn toggling_mine_is_refused_naming_the_label_and_changes_nothing() {
        let rig = Rig::synced().await;
        switch_off(&rig, Capability::Assign).await;
        let source = rig.ctx.source().unwrap();

        let message =
            refused(toggle_mine(&rig.ctx.db, &*source, &rig.settings(), NW_1044.into()).await);

        assert_eq!(message, off_message("Change the assignee"));
        assert_eq!(rig.assignments_sent(), 0);
        assert_eq!(rig.cached_assignee(NW_1044).await.as_deref(), Some(MARTA));
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn moving_is_refused_naming_the_label_and_changes_nothing() {
        let rig = Rig::synced().await;
        switch_off(&rig, Capability::ChangeStatus).await;

        let message = refused(rig.try_move_to(NW_1042, DONE).await);

        assert_eq!(message, off_message("Change the status"));
        assert_eq!(rig.transitions_posted(), 0);
        assert_eq!(
            rig.cached_status(NW_1042).await,
            (IN_PROGRESS.to_string(), "indeterminate".to_string())
        );
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn setting_the_priority_is_refused_naming_the_label_and_changes_nothing() {
        let rig = Rig::synced().await;
        switch_off(&rig, Capability::ChangePriority).await;

        let message = refused(rig.try_set_priority(NW_1042, "Low").await);

        assert_eq!(message, off_message("Change the priority"));
        assert_eq!(rig.priorities_sent(), 0);
        assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("High"));
        assert_eq!(rig.pending_writes().await, 0);
    }

    #[tokio::test]
    async fn commenting_is_refused_naming_the_label_and_adds_nothing() {
        let rig = Rig::synced().await;
        switch_off(&rig, Capability::Comment).await;

        let message = refused(rig.comment(NW_1042, "Checked").await);

        assert_eq!(message, off_message("Add comments"));
        assert_eq!(rig.comments_posted(), 0);
        assert_eq!(rig.pending_comments(NW_1042).await, 0);
        assert_eq!(rig.shown_comment_count(NW_1042).await, COMMENTS_ON_NW_1042);
    }

    #[tokio::test]
    async fn adding_a_web_link_is_refused_naming_the_label_and_adds_nothing() {
        let rig = Rig::synced().await;
        switch_off(&rig, Capability::AddWebLink).await;

        let message = refused(add_link(&rig).await);

        assert_eq!(message, off_message("Add web links"));
        assert_eq!(links_posted(&rig), 0);
        assert_eq!(
            issue_web_links(&rig.ctx.db, NW_1042.into()).await.unwrap(),
            []
        );
    }

    #[tokio::test]
    async fn the_other_kinds_of_change_still_go_through() {
        let rig = Rig::synced().await;
        switch_off(&rig, Capability::Comment).await;

        rig.apply(NW_1044, Some(ALEX)).await;
        rig.move_to(NW_1042, DONE).await;
        rig.set_priority(NW_1042, "Low").await;
        add_link(&rig).await.unwrap();

        assert_eq!(rig.jira_assignee(NW_1044).await.as_deref(), Some(ALEX));
        assert_eq!(rig.jira_status(NW_1042).await, DONE);
        assert_eq!(rig.jira_priority(NW_1042).await.as_deref(), Some("Low"));
        assert_eq!(links_posted(&rig), 1);
        assert_eq!(rig.comments_posted(), 0);
    }

    #[tokio::test]
    async fn a_drop_on_a_grouping_whose_change_is_off_is_refused_and_writes_nothing() {
        for (capability, group_by, key, label) in [
            (
                Capability::Assign,
                GroupBy::Assignee,
                MARTA,
                "Change the assignee",
            ),
            (
                Capability::ChangeStatus,
                GroupBy::Status,
                DONE,
                "Change the status",
            ),
            (
                Capability::ChangeStatus,
                GroupBy::StatusType,
                "done",
                "Change the status",
            ),
            (
                Capability::ChangePriority,
                GroupBy::Priority,
                "Low",
                "Change the priority",
            ),
        ] {
            let rig = Rig::synced().await;
            switch_off(&rig, capability).await;

            let message = refused(drop_on_group(&rig, group_by, key).await);

            assert_eq!(message, off_message(label), "{group_by:?}");
            assert_eq!(rig.transitions_posted(), 0, "{group_by:?}");
            assert_eq!(rig.assignments_sent(), 0, "{group_by:?}");
            assert_eq!(rig.priorities_sent(), 0, "{group_by:?}");
            assert_eq!(rig.pending_writes().await, 0, "{group_by:?}");
        }
    }

    #[tokio::test]
    async fn a_grouping_whose_change_is_off_offers_no_drop_targets_and_says_why() {
        for (capability, group_by, label) in [
            (Capability::Assign, GroupBy::Assignee, "Change the assignee"),
            (
                Capability::ChangeStatus,
                GroupBy::Status,
                "Change the status",
            ),
            (
                Capability::ChangeStatus,
                GroupBy::StatusType,
                "Change the status",
            ),
            (
                Capability::ChangePriority,
                GroupBy::Priority,
                "Change the priority",
            ),
        ] {
            let rig = Rig::synced().await;
            switch_off(&rig, capability).await;
            let requests_before = rig.fake.requests().len();

            let found = targets(&rig, group_by).await;

            assert_eq!(
                found,
                DropTargets {
                    targets: Vec::new(),
                    note: Some(off_message(label)),
                },
                "{group_by:?}"
            );
            assert_eq!(rig.fake.requests().len(), requests_before, "{group_by:?}");
        }
    }

    #[tokio::test]
    async fn the_groupings_of_the_other_changes_still_offer_their_targets() {
        let rig = Rig::synced().await;
        switch_off(&rig, Capability::ChangeStatus).await;

        let assignee = targets(&rig, GroupBy::Assignee).await;
        let priority = targets(&rig, GroupBy::Priority).await;

        assert_eq!(assignee.note, None);
        assert_eq!(assignee.targets.len(), 5);
        assert_eq!(priority.note, None);
        assert_eq!(priority.targets.len(), 5);
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

mod bulk {
    use super::*;
    use taskboard_lib::writes::{
        BulkOutcome, BulkStatus, IssueChange, apply_many, bulk_assignees, bulk_priorities,
        bulk_statuses,
    };

    const NWS_PROJECT: &str = "10002";

    fn transition(issue_id: &str, transition_id: &str) -> IssueChange {
        IssueChange {
            issue_id: issue_id.into(),
            change: Change::Transition {
                transition_id: transition_id.into(),
            },
        }
    }

    fn priority_of(issue_id: &str, name: &str) -> IssueChange {
        IssueChange {
            issue_id: issue_id.into(),
            change: Change::Priority { name: name.into() },
        }
    }

    /// A status offered with a move per issue; a move's id is its target status's id.
    fn offered(name: &str, style: Style, status_id: &str, issues: &[&str]) -> BulkStatus {
        BulkStatus {
            status_name: name.into(),
            style,
            changes: issues
                .iter()
                .map(|issue| transition(issue, status_id))
                .collect(),
        }
    }

    fn ids(issues: &[&str]) -> Vec<String> {
        issues.iter().map(|id| id.to_string()).collect()
    }

    async fn statuses(rig: &Rig, issues: &[&str]) -> Vec<BulkStatus> {
        let source = rig.ctx.source().unwrap();
        bulk_statuses(&rig.ctx.db, &*source, ids(issues))
            .await
            .unwrap()
    }

    async fn priorities_of(rig: &Rig, issues: &[&str]) -> Value {
        let source = rig.ctx.source().unwrap();
        let found = bulk_priorities(&*source, ids(issues)).await.unwrap();
        serde_json::to_value(found).unwrap()
    }

    async fn people(rig: &Rig, issues: &[&str]) -> AppResult<Vec<String>> {
        let found = bulk_assignees(&rig.ctx.db, &rig.settings(), ids(issues)).await?;
        Ok(found.into_iter().map(|person| person.name).collect())
    }

    async fn run(rig: &Rig, changes: Vec<IssueChange>, what: &str, total: usize) -> BulkOutcome {
        let source = rig.ctx.source().unwrap();
        apply_many(
            &rig.ctx.db,
            &*source,
            &rig.settings(),
            changes,
            what.into(),
            total,
        )
        .await
        .unwrap()
    }

    /// The changes the bulk menu offers for the status, applied as the bar does.
    async fn move_all(rig: &Rig, status_name: &str, issues: &[&str]) -> BulkOutcome {
        let offered = statuses(rig, issues).await;
        let status = offered
            .into_iter()
            .find(|s| s.status_name == status_name)
            .unwrap();
        run(
            rig,
            status.changes,
            &format!("Moved to {status_name}"),
            issues.len(),
        )
        .await
    }

    fn account_of(name: &str) -> String {
        demo::demo()
            .users
            .iter()
            .find(|user| user["displayName"] == name)
            .and_then(|user| user["accountId"].as_str())
            .unwrap()
            .to_string()
    }

    async fn first_issue_of(rig: &Rig, project_id: &'static str) -> String {
        rig.ctx
            .db
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT id FROM cache.issue WHERE project_id = ?1 AND gone = 0 ORDER BY id LIMIT 1",
                    [project_id],
                    |row| row.get(0),
                )?)
            })
            .await
            .unwrap()
    }

    async fn set_member(rig: &Rig, project_id: &str, account_id: &str, active: bool) {
        space_set_member(
            &rig.ctx.db,
            project_id.into(),
            account_id.into(),
            active,
            None,
        )
        .await
        .unwrap();
    }

    mod statuses_offered {
        use super::*;

        #[tokio::test]
        async fn for_issues_in_the_same_status_are_those_both_can_reach_with_a_move_each() {
            let rig = Rig::synced().await;

            let found = statuses(&rig, &[NW_1042, NW_1049]).await;

            assert_eq!(
                found,
                [
                    offered("To Do", Style::Todo, TO_DO, &[NW_1042, NW_1049]),
                    offered("In Review", Style::Review, IN_REVIEW, &[NW_1042, NW_1049]),
                    offered("Blocked", Style::Blocked, BLOCKED, &[NW_1042, NW_1049]),
                    offered("Done", Style::Done, DONE, &[NW_1042, NW_1049]),
                ]
            );
        }

        #[tokio::test]
        async fn include_a_status_one_issue_is_in_with_only_the_other_issues_move() {
            let rig = Rig::synced().await;

            let found = statuses(&rig, &[NW_1042, NW_1044]).await;

            assert_eq!(
                found,
                [
                    offered("To Do", Style::Todo, TO_DO, &[NW_1042, NW_1044]),
                    offered("In Review", Style::Review, IN_REVIEW, &[NW_1042]),
                    offered("Blocked", Style::Blocked, BLOCKED, &[NW_1042, NW_1044]),
                    offered("Done", Style::Done, DONE, &[NW_1042, NW_1044]),
                    offered("In Progress", Style::Progress, IN_PROGRESS, &[NW_1044]),
                ]
            );
        }

        #[tokio::test]
        async fn leave_out_a_status_all_the_issues_are_in() {
            let rig = Rig::synced().await;
            rig.move_to(NW_1044, IN_PROGRESS).await;

            let found = statuses(&rig, &[NW_1042, NW_1044, NW_1049]).await;

            let names: Vec<&str> = found.iter().map(|s| s.status_name.as_str()).collect();
            assert_eq!(names, ["To Do", "In Review", "Blocked", "Done"]);
            assert!(found.iter().all(|s| s.changes.len() == 3));
        }

        #[tokio::test]
        async fn leave_out_a_status_whose_move_needs_jiras_screen() {
            let rig = Rig::synced().await;
            rig.fake.require_screen(BLOCKED);

            let found = statuses(&rig, &[NW_1042, NW_1049]).await;

            let names: Vec<&str> = found.iter().map(|s| s.status_name.as_str()).collect();
            assert_eq!(names, ["To Do", "In Review", "Done"]);
        }

        #[tokio::test]
        async fn serialize_for_the_page_in_camel_case() {
            let rig = Rig::synced().await;

            let found = statuses(&rig, &[NW_1042]).await;

            assert_eq!(
                serde_json::to_value(&found[3]).unwrap(),
                json!({
                    "statusName": "Done",
                    "style": "done",
                    "changes": [{
                        "issueId": NW_1042,
                        "change": { "kind": "transition", "transitionId": DONE },
                    }],
                })
            );
        }
    }

    mod priorities_offered {
        use super::*;

        #[tokio::test]
        async fn are_the_edit_screens_priorities_with_their_levels() {
            let rig = Rig::synced().await;

            let found = priorities_of(&rig, &[NW_1042, NW_1049]).await;

            assert_eq!(
                found,
                json!([
                    { "name": "Highest", "level": 1 },
                    { "name": "High", "level": 2 },
                    { "name": "Medium", "level": 3 },
                    { "name": "Low", "level": 4 },
                    { "name": "Lowest", "level": 5 },
                ])
            );
        }

        #[tokio::test]
        async fn keep_only_what_every_issue_offers_in_the_first_issues_order() {
            let rig = Rig::synced().await;
            let edit_screen = |names: &[&str]| {
                let allowed: Vec<Value> =
                    names.iter().map(|name| json!({ "name": name })).collect();
                json!({ "fields": { "priority": { "allowedValues": allowed } } })
            };
            rig.fake
                .answer_next(200, edit_screen(&["Highest", "High", "Medium", "Low"]));
            rig.fake
                .answer_next(200, edit_screen(&["Lowest", "Low", "High"]));

            let found = priorities_of(&rig, &[NW_1042, NW_1049]).await;

            assert_eq!(
                found,
                json!([
                    { "name": "High", "level": 2 },
                    { "name": "Low", "level": 4 },
                ])
            );
        }

        #[tokio::test]
        async fn are_none_without_issues() {
            let rig = Rig::synced().await;

            assert_eq!(priorities_of(&rig, &[]).await, json!([]));
        }
    }

    mod assignees_offered {
        use super::*;

        #[tokio::test]
        async fn are_the_spaces_team_with_me_first_once_for_issues_of_one_space() {
            let rig = Rig::synced().await;

            let found = people(&rig, &[NW_1042, NW_1044]).await.unwrap();

            assert_eq!(
                found,
                ["Alex Morgan", "Anna Lutz", "Jonas Reiter", "Marta Klein"]
            );
        }

        #[tokio::test]
        async fn always_include_me_first_even_when_switched_off_the_team() {
            let rig = Rig::synced().await;
            set_member(&rig, NW_PROJECT, ALEX, false).await;

            let found = bulk_assignees(&rig.ctx.db, &rig.settings(), ids(&[NW_1044]))
                .await
                .unwrap();

            assert_eq!(found[0].account_id, ALEX);
            assert_eq!(found.len(), 4);
        }

        #[tokio::test]
        async fn for_issues_of_two_spaces_are_the_people_on_both_teams() {
            let rig = Rig::synced().await;
            let other = first_issue_of(&rig, NWS_PROJECT).await;
            set_member(&rig, NWS_PROJECT, &account_of("Anna Lutz"), false).await;
            set_member(&rig, NWS_PROJECT, MARTA, false).await;
            set_member(&rig, NWS_PROJECT, JONAS, true).await;

            let found = people(&rig, &[NW_1044, other.as_str()]).await.unwrap();

            assert_eq!(found, ["Alex Morgan", "Jonas Reiter"]);
        }

        #[tokio::test]
        async fn leave_out_someone_switched_off_in_one_of_the_spaces() {
            let rig = Rig::synced().await;
            let other = first_issue_of(&rig, NWS_PROJECT).await;
            set_member(&rig, NWS_PROJECT, &account_of("Anna Lutz"), false).await;
            set_member(&rig, NWS_PROJECT, MARTA, false).await;
            set_member(&rig, NWS_PROJECT, JONAS, true).await;
            set_member(&rig, NW_PROJECT, JONAS, false).await;

            let found = people(&rig, &[NW_1044, other.as_str()]).await.unwrap();

            assert_eq!(found, ["Alex Morgan"]);
        }

        #[tokio::test]
        async fn fail_as_not_found_for_an_issue_missing_from_the_cache() {
            let rig = Rig::synced().await;

            let err = people(&rig, &[NW_1044, "999999"]).await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::NotFound);
        }
    }

    mod applying_many {
        use super::*;

        #[tokio::test]
        async fn lands_every_change_in_the_cache_and_in_jira() {
            let rig = Rig::synced().await;

            let outcome = move_all(&rig, "Done", &[NW_1042, NW_1049]).await;

            assert_eq!(
                outcome,
                BulkOutcome {
                    done: "Moved to Done: 2 issues".into(),
                    changed: 2,
                    refused: 0,
                    reverse: Some(vec![
                        transition(NW_1042, IN_PROGRESS),
                        transition(NW_1049, IN_PROGRESS),
                    ]),
                }
            );
            for id in [NW_1042, NW_1049] {
                assert_eq!(
                    rig.cached_status(id).await,
                    (DONE.to_string(), "done".to_string()),
                    "{id}"
                );
                assert_eq!(rig.jira_status(id).await, DONE, "{id}");
            }
            assert_eq!(rig.pending_writes().await, 0);
            assert_eq!(rig.transitions_posted(), 2);
        }

        #[tokio::test]
        async fn the_reverse_changes_applied_the_same_way_restore_the_old_values() {
            let rig = Rig::synced().await;
            let outcome = move_all(&rig, "Done", &[NW_1042, NW_1049]).await;

            let back = run(&rig, outcome.reverse.unwrap(), "Changed back", 2).await;

            assert_eq!(back.done, "Changed back: 2 issues");
            assert_eq!((back.changed, back.refused), (2, 0));
            for id in [NW_1042, NW_1049] {
                assert_eq!(
                    rig.cached_status(id).await,
                    (IN_PROGRESS.to_string(), "indeterminate".to_string()),
                    "{id}"
                );
                assert_eq!(rig.jira_status(id).await, IN_PROGRESS, "{id}");
            }
            assert_eq!(rig.pending_writes().await, 0);
        }

        #[tokio::test]
        async fn a_refusal_names_its_issue_and_the_rest_still_apply() {
            let rig = Rig::synced().await;
            rig.fake
                .answer_next_write(400, json!({ "errorMessages": ["Not allowed"] }));

            let outcome = move_all(&rig, "Done", &[NW_1042, NW_1049]).await;

            assert_eq!(
                outcome,
                BulkOutcome {
                    done: "Moved to Done: 1 of 2 issues. NW-1042: Jira refused the change: Not allowed"
                        .into(),
                    changed: 1,
                    refused: 1,
                    reverse: Some(vec![transition(NW_1049, IN_PROGRESS)]),
                }
            );
            assert_eq!(rig.jira_status(NW_1042).await, IN_PROGRESS);
            assert_eq!(rig.jira_status(NW_1049).await, DONE);
            assert_eq!(
                rig.cached_status(NW_1042).await,
                (IN_PROGRESS.to_string(), "indeterminate".to_string())
            );
            assert_eq!(
                rig.cached_status(NW_1049).await,
                (DONE.to_string(), "done".to_string())
            );
            assert_eq!(rig.pending_writes().await, 0);
            assert_eq!(rig.transitions_posted(), 2);
        }

        #[tokio::test]
        async fn a_kind_of_change_switched_off_refuses_every_change_and_sends_nothing() {
            let rig = Rig::synced().await;
            rig.ctx
                .settings
                .edit(|s| s.behaviour.disabled_changes = vec![Capability::ChangePriority])
                .await
                .unwrap();
            let requests_before = rig.fake.requests().len();

            let outcome = run(
                &rig,
                vec![priority_of(NW_1042, "Low"), priority_of(NW_1049, "Low")],
                "Set priority to Low",
                2,
            )
            .await;

            assert_eq!(
                outcome,
                BulkOutcome {
                    done: "Set priority to Low: 0 of 2 issues. NW-1042: “Change the priority” is switched off in Settings → Connection."
                        .into(),
                    changed: 0,
                    refused: 2,
                    reverse: None,
                }
            );
            assert_eq!(rig.fake.requests().len(), requests_before);
            assert_eq!(rig.cached_priority(NW_1042).await.as_deref(), Some("High"));
        }

        #[tokio::test]
        async fn words_a_single_issue_in_the_singular() {
            let rig = Rig::synced().await;

            let outcome = move_all(&rig, "Done", &[NW_1042]).await;

            assert_eq!(outcome.done, "Moved to Done: 1 issue");
            assert_eq!(outcome.changed, 1);
        }

        #[tokio::test]
        async fn counts_the_issues_that_needed_no_change_as_changed() {
            let rig = Rig::synced().await;

            let outcome = move_all(&rig, "In Progress", &[NW_1042, NW_1044]).await;

            assert_eq!(
                outcome,
                BulkOutcome {
                    done: "Moved to In Progress: 2 issues".into(),
                    changed: 2,
                    refused: 0,
                    reverse: Some(vec![transition(NW_1044, IN_REVIEW)]),
                }
            );
            assert_eq!(rig.transitions_posted(), 1);
        }

        #[tokio::test]
        async fn without_changes_offers_no_reverse() {
            let rig = Rig::synced().await;

            let outcome = run(&rig, Vec::new(), "Moved to Done", 0).await;

            assert_eq!(
                outcome,
                BulkOutcome {
                    done: "Moved to Done: 0 issues".into(),
                    changed: 0,
                    refused: 0,
                    reverse: None,
                }
            );
        }
    }

    mod for_the_page {
        use super::*;

        #[test]
        fn the_outcome_serializes_in_camel_case() {
            let outcome = BulkOutcome {
                done: "Moved to Done: 2 issues".into(),
                changed: 2,
                refused: 0,
                reverse: Some(vec![transition(NW_1042, IN_PROGRESS)]),
            };

            assert_eq!(
                serde_json::to_value(&outcome).unwrap(),
                json!({
                    "done": "Moved to Done: 2 issues",
                    "changed": 2,
                    "refused": 0,
                    "reverse": [{
                        "issueId": NW_1042,
                        "change": { "kind": "transition", "transitionId": IN_PROGRESS },
                    }],
                })
            );
        }

        #[test]
        fn an_outcome_without_a_reverse_serializes_it_as_null() {
            let outcome = BulkOutcome {
                done: "Moved to Done: 0 of 1 issue".into(),
                changed: 0,
                refused: 1,
                reverse: None,
            };

            assert_eq!(
                serde_json::to_value(&outcome).unwrap()["reverse"],
                Value::Null
            );
        }

        #[test]
        fn an_issue_change_reads_the_pages_json() {
            let change: IssueChange = serde_json::from_value(json!({
                "issueId": NW_1042,
                "change": { "kind": "priority", "name": "Low" },
            }))
            .unwrap();

            assert_eq!(change, priority_of(NW_1042, "Low"));
        }
    }
}
