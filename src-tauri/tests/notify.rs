//! What Taskboard would announce (`notify::notices`) and how the `Notifier` hands changes out, on a synced demo
//! cache and changes the fake Jira makes before a delta sync.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;
use tempfile::TempDir;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::notify::{MAX_SINGLE, Notice, Notifier, notices};
use taskboard_lib::store::settings::{NotifyAbout, Settings, SettingsStore};
use taskboard_lib::sync::{Emit, Job, SyncContext, full_sync, run_job};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};

const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const ALEX: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const MARTA: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03";
const DONE: &str = "10001";

fn later() -> DateTime<Utc> {
    DEMO_NOW.parse::<DateTime<Utc>>().unwrap() + chrono::Duration::hours(2)
}

fn demo_user(account_id: &str) -> Value {
    demo::demo()
        .users
        .into_iter()
        .find(|user| user["accountId"] == account_id)
        .unwrap()
}

fn demo_status(id: &str) -> Value {
    demo::demo()
        .statuses
        .into_iter()
        .find(|status| status["id"] == id)
        .unwrap()
}

struct Rig {
    fake: FakeJira,
    _dir: TempDir,
    ctx: SyncContext,
}

impl Rig {
    /// Connected to the demo site and synced; the clock stands two hours later for what the test changes.
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
            NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(),
        )
        .await
        .unwrap();
        refresh_projects(&ctx.db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();
        for id in PROJECT_IDS {
            track_project(&ctx.db, id.into(), true).await.unwrap();
        }
        let emit: Emit = Arc::new(|_| {});
        full_sync(&ctx, &*emit).await.unwrap();
        fake.set_now(later());
        Self {
            fake,
            _dir: dir,
            ctx,
        }
    }

    async fn delta(&self) {
        let emit: Emit = Arc::new(|_| {});
        run_job(&self.ctx, Job::Delta, &*emit, None).await;
    }

    async fn newest_change(&self) -> i64 {
        self.ctx
            .db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT COALESCE(MAX(rowid), 0) FROM cache.issue_change",
                    [],
                    |row| row.get(0),
                )?)
            })
            .await
            .unwrap()
    }

    async fn notices_after(&self, after: i64, about: NotifyAbout) -> Vec<Notice> {
        self.ctx
            .db
            .read(move |conn| notices(conn, after, ALEX, about))
            .await
            .unwrap()
    }

    fn settings(&self, about: NotifyAbout) -> Settings {
        let mut settings = self.ctx.settings.get();
        settings.behaviour.notify_about = about;
        settings
    }

    fn assign_to_me(&self, key: &str) {
        self.fake
            .edit_issue(key, |fields| fields["assignee"] = demo_user(ALEX));
    }

    fn move_to_done(&self, key: &str) {
        self.fake
            .edit_issue(key, |fields| fields["status"] = demo_status(DONE));
    }
}

fn notice(key: &str, summary: &str, body: &str) -> Notice {
    Notice {
        title: format!("{key} {summary}"),
        body: body.into(),
        key: Some(key.into()),
    }
}

const NW_1042: (&str, &str) = ("NW-1042", "Checkout: address validation for DACH postcodes");
const NW_1044: (&str, &str) = ("NW-1044", "Lazy-load product gallery images");
const NW_1060: (&str, &str) = ("NW-1060", "Screen reader hint for the postcode field");

mod the_notices {
    use super::*;

    #[tokio::test]
    async fn a_mention_of_me_is_announced_with_the_issue_and_what_happened() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.fake.add_mention(
            "NW-1044",
            demo_user(MARTA),
            "Please check the Alpine",
            &demo_user(ALEX),
        );
        rig.delta().await;

        let found = rig
            .notices_after(before, NotifyAbout::MentionsAndAssignments)
            .await;

        assert_eq!(found, vec![notice(NW_1044.0, NW_1044.1, "Mentioned you.")]);
    }

    #[tokio::test]
    async fn an_assignment_to_me_is_announced() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.assign_to_me("NW-1044");
        rig.delta().await;

        let found = rig
            .notices_after(before, NotifyAbout::MentionsAndAssignments)
            .await;

        assert_eq!(
            found,
            vec![notice(NW_1044.0, NW_1044.1, "Assigned to you.")]
        );
    }

    #[tokio::test]
    async fn a_status_change_of_my_issue_alone_is_announced_only_for_my_issues() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.move_to_done("NW-1042");
        rig.delta().await;

        let narrow = rig
            .notices_after(before, NotifyAbout::MentionsAndAssignments)
            .await;
        let wide = rig.notices_after(before, NotifyAbout::MyIssues).await;

        assert_eq!(narrow, vec![]);
        assert_eq!(wide, vec![notice(NW_1042.0, NW_1042.1, "Moved to Done.")]);
    }

    #[tokio::test]
    async fn my_issues_include_the_issues_i_reported() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.move_to_done("NW-1060");
        rig.delta().await;

        let narrow = rig
            .notices_after(before, NotifyAbout::MentionsAndAssignments)
            .await;
        let wide = rig.notices_after(before, NotifyAbout::MyIssues).await;

        assert_eq!(narrow, vec![]);
        assert_eq!(wide, vec![notice(NW_1060.0, NW_1060.1, "Moved to Done.")]);
    }

    #[tokio::test]
    async fn mentions_of_others_and_changes_of_others_issues_are_never_announced() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.fake.add_mention(
            "NW-1044",
            demo_user(ALEX),
            "Marta knows the Danube",
            &demo_user(MARTA),
        );
        rig.move_to_done("NW-1046");
        rig.delta().await;

        for about in [NotifyAbout::MentionsAndAssignments, NotifyAbout::MyIssues] {
            assert_eq!(rig.notices_after(before, about).await, vec![], "{about:?}");
        }
    }

    #[tokio::test]
    async fn changes_at_or_before_the_given_row_are_ignored() {
        let rig = Rig::synced().await;
        rig.assign_to_me("NW-1044");
        rig.delta().await;
        let seen = rig.newest_change().await;

        let found = rig
            .notices_after(seen, NotifyAbout::MentionsAndAssignments)
            .await;

        assert_eq!(found, vec![]);
    }

    #[tokio::test]
    async fn created_and_gone_rows_never_notify() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.fake
            .add_issue("NW-1055", "NW", "Order confirmation email: shipping notice");
        rig.delta().await;
        rig.ctx
            .db
            .write(|tx, _| {
                tx.execute(
                    "INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
                     VALUES ('101042', 'gone', NULL, NULL, '2026-10-28T13:00:00.000Z')",
                    [],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        for about in [NotifyAbout::MentionsAndAssignments, NotifyAbout::MyIssues] {
            assert_eq!(rig.notices_after(before, about).await, vec![], "{about:?}");
        }
    }

    #[tokio::test]
    async fn up_to_the_limit_each_issue_gets_its_own_notice() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        let keys = ["NW-1044", "NW-1046", "NW-1050"];
        assert_eq!(keys.len(), MAX_SINGLE);
        for key in keys {
            rig.assign_to_me(key);
        }
        rig.delta().await;

        let found = rig
            .notices_after(before, NotifyAbout::MentionsAndAssignments)
            .await;

        let announced: Vec<Option<&str>> = found.iter().map(|n| n.key.as_deref()).collect();
        assert_eq!(
            announced,
            [Some("NW-1044"), Some("NW-1046"), Some("NW-1050")]
        );
    }

    #[tokio::test]
    async fn more_issues_than_the_limit_make_one_summary() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        for key in ["NW-1044", "NW-1046", "NW-1050", "NW-1053"] {
            rig.assign_to_me(key);
        }
        rig.delta().await;

        let found = rig
            .notices_after(before, NotifyAbout::MentionsAndAssignments)
            .await;

        assert_eq!(
            found,
            vec![Notice {
                title: "4 issues changed".into(),
                body: "NW-1044, NW-1046, NW-1050, NW-1053".into(),
                key: None,
            }]
        );
    }
}

mod the_notifier {
    use super::*;

    #[tokio::test]
    async fn starts_after_the_rows_that_exist_and_announces_only_new_ones_once() {
        let rig = Rig::synced().await;
        rig.assign_to_me("NW-1046");
        rig.delta().await;
        let notifier = Notifier::start(&rig.ctx.db).await.unwrap();
        let settings = rig.settings(NotifyAbout::MentionsAndAssignments);
        assert_eq!(
            notifier.take(&rig.ctx.db, &settings, true).await.unwrap(),
            vec![]
        );

        rig.assign_to_me("NW-1044");
        rig.delta().await;

        assert_eq!(
            notifier.take(&rig.ctx.db, &settings, true).await.unwrap(),
            vec![notice(NW_1044.0, NW_1044.1, "Assigned to you.")]
        );
        assert_eq!(
            notifier.take(&rig.ctx.db, &settings, true).await.unwrap(),
            vec![]
        );
    }

    #[tokio::test]
    async fn after_the_cache_was_reset_it_reads_the_renumbered_changes_from_the_start() {
        let rig = Rig::synced().await;
        for key in ["NW-1046", "NW-1050", "NW-1053"] {
            rig.assign_to_me(key);
        }
        rig.delta().await;
        rig.ctx
            .db
            .write(|tx, _| {
                tx.execute(
                    "INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
                     SELECT id, 'priority', 'High', 'Low', '2026-10-28T09:00:00.000Z'
                     FROM cache.issue LIMIT 20",
                    [],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let notifier = Notifier::start(&rig.ctx.db).await.unwrap();
        let settings = rig.settings(NotifyAbout::MentionsAndAssignments);
        let before_reset = rig.newest_change().await;
        assert!(before_reset >= 20);

        rig.ctx.db.reset_cache().await.unwrap();
        refresh_projects(&rig.ctx.db, &rig.fake.client(TokenKind::Classic))
            .await
            .unwrap();
        for id in PROJECT_IDS {
            track_project(&rig.ctx.db, id.into(), true).await.unwrap();
        }
        let emit: Emit = Arc::new(|_| {});
        full_sync(&rig.ctx, &*emit).await.unwrap();
        assert!(rig.newest_change().await < before_reset);
        rig.fake.add_mention(
            "NW-1044",
            demo_user(MARTA),
            "Please check the Alpine",
            &demo_user(ALEX),
        );
        rig.delta().await;
        assert!(rig.newest_change().await < before_reset);

        let found = notifier.take(&rig.ctx.db, &settings, true).await.unwrap();

        assert_eq!(found, vec![notice(NW_1044.0, NW_1044.1, "Mentioned you.")]);
    }

    #[tokio::test]
    async fn changes_it_is_told_not_to_announce_are_passed_by_for_good() {
        let rig = Rig::synced().await;
        let notifier = Notifier::start(&rig.ctx.db).await.unwrap();
        let settings = rig.settings(NotifyAbout::MentionsAndAssignments);
        rig.assign_to_me("NW-1044");
        rig.delta().await;

        let quiet = notifier.take(&rig.ctx.db, &settings, false).await.unwrap();
        let after = notifier.take(&rig.ctx.db, &settings, true).await.unwrap();

        assert_eq!(quiet, vec![]);
        assert_eq!(after, vec![]);
    }

    #[tokio::test]
    async fn announces_nothing_when_the_owner_wants_nothing_and_still_passes_the_changes_by() {
        let rig = Rig::synced().await;
        let notifier = Notifier::start(&rig.ctx.db).await.unwrap();
        rig.assign_to_me("NW-1044");
        rig.delta().await;

        let nothing = notifier
            .take(&rig.ctx.db, &rig.settings(NotifyAbout::Nothing), true)
            .await
            .unwrap();
        let later_on = notifier
            .take(
                &rig.ctx.db,
                &rig.settings(NotifyAbout::MentionsAndAssignments),
                true,
            )
            .await
            .unwrap();

        assert_eq!(nothing, vec![]);
        assert_eq!(later_on, vec![]);
    }

    #[tokio::test]
    async fn announces_nothing_without_a_known_account_and_still_passes_the_changes_by() {
        let rig = Rig::synced().await;
        let notifier = Notifier::start(&rig.ctx.db).await.unwrap();
        rig.assign_to_me("NW-1044");
        rig.delta().await;
        let mut unknown = rig.settings(NotifyAbout::MentionsAndAssignments);
        unknown.connection.account_id = None;

        let without = notifier.take(&rig.ctx.db, &unknown, true).await.unwrap();
        let with = notifier
            .take(
                &rig.ctx.db,
                &rig.settings(NotifyAbout::MentionsAndAssignments),
                true,
            )
            .await
            .unwrap();

        assert_eq!(without, vec![]);
        assert_eq!(with, vec![]);
    }
}
