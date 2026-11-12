//! What Taskboard would announce (`notify::notices`), how the `Notifier` hands changes out and which recorded
//! changes make an issue unread (Settings → Notifications), on a synced demo cache and changes the fake Jira
//! makes before a delta sync.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use tempfile::TempDir;

use taskboard_lib::changes::{UNSEEN, mark_unread, unseen_changes};
use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::field_choice::field_choice_set;
use taskboard_lib::inbox;
use taskboard_lib::issues::{ActivityItem, issue_activity};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::Viewer;
use taskboard_lib::notify::rules::resettle;
use taskboard_lib::notify::{MAX_SINGLE, Notice, Notifier, notices};
use taskboard_lib::store::settings::{
    Involvement, Notifications, NotifyEvent, NotifyScope, Settings, SettingsStore,
};
use taskboard_lib::sync::{Emit, Job, SyncContext, full_sync, run_job};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL, REVIEWER, STORY_POINTS};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};

const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const ALEX: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const MARTA: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03";
const DONE: &str = "10001";
const TEN: &str = "2026-10-28T10:00:00.000Z";

fn viewer() -> Viewer {
    Viewer {
        me: Some(ALEX.into()),
        time_zone: Some("Europe/Berlin".into()),
        now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
    }
}

/// The default settings with `change` applied.
fn notifications(change: impl FnOnce(&mut Notifications)) -> Notifications {
    let mut settings = Notifications::default();
    change(&mut settings);
    settings
}

fn every_event_but(event: NotifyEvent) -> Vec<NotifyEvent> {
    NotifyEvent::ALL
        .into_iter()
        .filter(|e| *e != event)
        .collect()
}

fn keys(found: &[&str]) -> Vec<String> {
    found.iter().map(|key| key.to_string()).collect()
}

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
        Self::synced_with(&[]).await
    }

    /// Like `synced`, with the custom `fields` opted in and fetched; every issue counts as read at 10:00.
    async fn synced_with(fields: &[&str]) -> Self {
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
        if !fields.is_empty() {
            for id in fields {
                field_choice_set(&ctx.db, id.to_string(), true, true)
                    .await
                    .unwrap();
            }
            full_sync(&ctx, &*emit).await.unwrap();
        }
        ctx.db
            .write(|tx, _| {
                tx.execute("UPDATE issue_seen SET seen_at = ?1", [TEN])?;
                Ok(())
            })
            .await
            .unwrap();
        fake.set_now(later());
        Self {
            fake,
            _dir: dir,
            ctx,
        }
    }

    async fn strings(&self, sql: String) -> Vec<String> {
        self.ctx
            .db
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

    async fn id_of(&self, key: &str) -> String {
        self.strings(format!("SELECT id FROM cache.issue WHERE key = '{key}'"))
            .await
            .remove(0)
    }

    /// The keys of the issues that are unread, sorted.
    async fn unread_keys(&self) -> Vec<String> {
        let mut found = self
            .strings(format!(
                "SELECT i.key FROM cache.issue i WHERE i.gone = 0 AND {UNSEEN}"
            ))
            .await;
        found.sort();
        found
    }

    /// The bell's badge.
    async fn badge(&self) -> u32 {
        self.ctx.db.read(inbox::count).await.unwrap()
    }

    /// The inbox's note of the issue, none when the inbox doesn't list it.
    async fn note_of(&self, key: &str) -> Option<String> {
        let listed = self
            .ctx
            .db
            .read(|conn| {
                let taken = inbox::snapshot(conn, Some(ALEX))?;
                inbox::rows(conn, &taken)
            })
            .await
            .unwrap();
        listed
            .rows
            .into_iter()
            .find(|row| row.key == key)
            .map(|row| row.note)
    }

    /// The `people` changes recorded so far, as `field>account`.
    async fn people_changes(&self) -> Vec<String> {
        self.strings(
            "SELECT old_value || '>' || new_value FROM cache.issue_change
             WHERE field = 'people' ORDER BY id"
                .into(),
        )
        .await
    }

    /// Takes effect for the next sync; `resettle` applies it to what is recorded already.
    async fn use_notifications(&self, notifications: Notifications) {
        self.ctx
            .settings
            .edit(move |settings| settings.notifications = notifications)
            .await
            .unwrap();
    }

    async fn resettle(&self) {
        resettle(&self.ctx.db, &self.ctx.settings.get())
            .await
            .unwrap();
    }

    fn reviewer_is(&self, key: &str, account_id: &str) {
        self.fake
            .edit_issue(key, |fields| fields[REVIEWER] = demo_user(account_id));
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

    async fn notices_after(&self, after: i64, settings: &Notifications) -> Vec<Notice> {
        let settings = settings.clone();
        self.ctx
            .db
            .read(move |conn| notices(conn, after, ALEX, &settings))
            .await
            .unwrap()
    }

    fn settings(&self, notifications: Notifications) -> Settings {
        let mut settings = self.ctx.settings.get();
        settings.notifications = notifications;
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
const NW_1046: (&str, &str) = ("NW-1046", "Preload critical fonts on landing pages");
const NW_1050: (&str, &str) = ("NW-1050", "Skip link hidden behind sticky header");
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

        let found = rig.notices_after(before, &Notifications::default()).await;

        assert_eq!(found, vec![notice(NW_1044.0, NW_1044.1, "Mentioned you.")]);
    }

    #[tokio::test]
    async fn an_assignment_to_me_is_announced() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.assign_to_me("NW-1044");
        rig.delta().await;

        let found = rig.notices_after(before, &Notifications::default()).await;

        assert_eq!(
            found,
            vec![notice(NW_1044.0, NW_1044.1, "Assigned to you.")]
        );
    }

    #[tokio::test]
    async fn a_status_change_of_my_issue_is_announced_only_when_ticked() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.move_to_done("NW-1042");
        rig.delta().await;
        let ticked = notifications(|n| n.mac = vec![NotifyEvent::MovedToDone]);

        let unticked = rig.notices_after(before, &Notifications::default()).await;
        let found = rig.notices_after(before, &ticked).await;

        assert_eq!(unticked, vec![]);
        assert_eq!(found, vec![notice(NW_1042.0, NW_1042.1, "Moved to Done.")]);
    }

    #[tokio::test]
    async fn the_involved_scope_includes_the_issues_i_reported_unless_unticked() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.move_to_done("NW-1060");
        rig.delta().await;
        let reported = notifications(|n| {
            n.mac = vec![NotifyEvent::MovedToDone];
            n.scope = NotifyScope::Involved;
        });
        let not_reported = notifications(|n| {
            n.mac = vec![NotifyEvent::MovedToDone];
            n.scope = NotifyScope::Involved;
            n.involvement.reported = false;
        });

        let found = rig.notices_after(before, &reported).await;
        let left_out = rig.notices_after(before, &not_reported).await;

        assert_eq!(found, vec![notice(NW_1060.0, NW_1060.1, "Moved to Done.")]);
        assert_eq!(left_out, vec![]);
    }

    #[tokio::test]
    async fn mentions_of_others_and_changes_of_others_issues_are_not_announced_in_the_involved_scope()
     {
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

        for (name, settings) in [
            ("default", Notifications::default()),
            (
                "involved",
                notifications(|n| {
                    n.mac = NotifyEvent::ALL.to_vec();
                    n.scope = NotifyScope::Involved;
                }),
            ),
        ] {
            assert_eq!(rig.notices_after(before, &settings).await, vec![], "{name}");
        }
    }

    #[tokio::test]
    async fn changes_at_or_before_the_given_row_are_ignored() {
        let rig = Rig::synced().await;
        rig.assign_to_me("NW-1044");
        rig.delta().await;
        let seen = rig.newest_change().await;

        let found = rig.notices_after(seen, &Notifications::default()).await;

        assert_eq!(found, vec![]);
    }

    #[tokio::test]
    async fn a_created_issue_is_announced_only_when_issue_created_is_ticked() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        let added =
            rig.fake
                .add_issue("NW-1055", "NW", "Order confirmation email: shipping notice");
        rig.delta().await;
        let ticked = notifications(|n| n.mac = vec![NotifyEvent::Created]);

        let unticked = rig.notices_after(before, &Notifications::default()).await;
        let found = rig.notices_after(before, &ticked).await;

        assert_eq!(unticked, vec![]);
        assert_eq!(
            found,
            vec![notice(
                &added,
                "Order confirmation email: shipping notice",
                "Created."
            )]
        );
    }

    #[tokio::test]
    async fn a_gone_row_never_notifies() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
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

        let every_event = notifications(|n| n.mac = NotifyEvent::ALL.to_vec());
        assert_eq!(rig.notices_after(before, &every_event).await, vec![]);
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

        let found = rig.notices_after(before, &Notifications::default()).await;

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

        let found = rig.notices_after(before, &Notifications::default()).await;

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
        let settings = rig.settings(Notifications::default());
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
        let settings = rig.settings(Notifications::default());
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
        let settings = rig.settings(Notifications::default());
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

        let wants_nothing = notifications(|n| n.mac = Vec::new());
        let nothing = notifier
            .take(&rig.ctx.db, &rig.settings(wants_nothing), true)
            .await
            .unwrap();
        let later_on = notifier
            .take(&rig.ctx.db, &rig.settings(Notifications::default()), true)
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
        let mut unknown = rig.settings(Notifications::default());
        unknown.connection.account_id = None;

        let without = notifier.take(&rig.ctx.db, &unknown, true).await.unwrap();
        let with = notifier
            .take(&rig.ctx.db, &rig.settings(Notifications::default()), true)
            .await
            .unwrap();

        assert_eq!(without, vec![]);
        assert_eq!(with, vec![]);
    }
}

mod the_mac_column {
    use super::*;

    #[tokio::test]
    async fn announces_only_the_events_ticked_for_macos() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.assign_to_me("NW-1044");
        rig.move_to_done("NW-1042");
        rig.delta().await;
        let assigned = notice(NW_1044.0, NW_1044.1, "Assigned to you.");
        let done = notice(NW_1042.0, NW_1042.1, "Moved to Done.");

        for (mac, expected) in [
            (vec![NotifyEvent::AssignsMe], vec![assigned.clone()]),
            (vec![NotifyEvent::MovedToDone], vec![done.clone()]),
            (
                vec![NotifyEvent::AssignsMe, NotifyEvent::MovedToDone],
                vec![done, assigned],
            ),
            (vec![], vec![]),
        ] {
            let settings = notifications(|n| n.mac = mac.clone());
            assert_eq!(
                rig.notices_after(before, &settings).await,
                expected,
                "{mac:?}"
            );
        }
    }

    #[tokio::test]
    async fn the_filters_apply_to_the_on_these_issues_events() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.move_to_done("NW-1042");
        rig.move_to_done("NW-1050");
        rig.delta().await;
        let mine = notice(NW_1042.0, NW_1042.1, "Moved to Done.");
        let others = notice(NW_1050.0, NW_1050.1, "Moved to Done.");

        let cases: [(&str, fn(&mut Notifications), Vec<&Notice>); 4] = [
            ("every issue", |_| {}, vec![&mine, &others]),
            (
                "only involved",
                |n| n.scope = NotifyScope::Involved,
                vec![&mine],
            ),
            (
                "bugs left out",
                |n| n.excluded_types = vec!["Bug".into()],
                vec![&mine],
            ),
            (
                "stories left out",
                |n| n.excluded_types = vec!["Story".into()],
                vec![&others],
            ),
        ];
        for (name, filter, expected) in cases {
            let settings = notifications(|n| {
                n.mac = vec![NotifyEvent::MovedToDone];
                filter(n);
            });
            let expected: Vec<Notice> = expected.into_iter().cloned().collect();
            assert_eq!(
                rig.notices_after(before, &settings).await,
                expected,
                "{name}"
            );
        }
    }

    #[tokio::test]
    async fn events_about_me_pass_the_scope_and_type_filters() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.assign_to_me("NW-1044");
        rig.fake.add_mention(
            "NW-1046",
            demo_user(MARTA),
            "Please check the Alpine",
            &demo_user(ALEX),
        );
        rig.delta().await;
        let settings = notifications(|n| {
            n.scope = NotifyScope::Involved;
            n.involvement = Involvement {
                assigned: false,
                reported: false,
                people_fields: false,
            };
            n.excluded_types = vec!["Task".into()];
        });

        let found = rig.notices_after(before, &settings).await;

        assert_eq!(
            found,
            vec![
                notice(NW_1044.0, NW_1044.1, "Assigned to you."),
                notice(NW_1046.0, NW_1046.1, "Mentioned you."),
            ]
        );
    }
}

mod unread_by_the_in_app_column {
    use super::*;

    #[tokio::test]
    async fn an_unticked_event_leaves_the_issue_read_and_the_change_recorded() {
        let rig = Rig::synced().await;
        rig.use_notifications(notifications(|n| {
            n.in_app = every_event_but(NotifyEvent::NewComment)
        }))
        .await;
        rig.fake.add_comment(
            "NW-1042",
            demo_user(MARTA),
            "Postcode rules for Liechtenstein are merged.",
        );
        rig.delta().await;

        let id = rig.id_of("NW-1042").await;
        let unseen = rig
            .ctx
            .db
            .read({
                let id = id.clone();
                move |conn| unseen_changes(conn, &[id.as_str()])
            })
            .await
            .unwrap();
        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
        assert_eq!(rig.badge().await, 0);
        assert_eq!(rig.note_of("NW-1042").await, None);
        assert!(unseen.is_empty());
        assert_eq!(
            rig.strings(format!(
                "SELECT COALESCE(CAST(quiet AS TEXT), 'unsettled') FROM cache.issue_change
                 WHERE issue_id = '{id}' AND field = 'comment_added'"
            ))
            .await,
            ["1"]
        );
    }

    #[tokio::test]
    async fn a_ticked_event_makes_the_issue_unread() {
        let rig = Rig::synced().await;
        rig.fake.add_comment(
            "NW-1042",
            demo_user(MARTA),
            "Postcode rules for Liechtenstein are merged.",
        );
        rig.delta().await;

        assert_eq!(rig.unread_keys().await, keys(&["NW-1042"]));
        assert_eq!(rig.badge().await, 1);
        assert_eq!(
            rig.note_of("NW-1042").await.as_deref(),
            Some("1 new comment.")
        );
    }

    #[tokio::test]
    async fn a_quiet_change_still_shows_in_the_activity_and_is_not_fresh() {
        let rig = Rig::synced().await;
        rig.use_notifications(notifications(|n| {
            n.in_app = every_event_but(NotifyEvent::DueOrPriority)
        }))
        .await;
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["priority"] = json!({ "id": "4", "name": "Low" });
        });
        rig.delta().await;

        let id = rig.id_of("NW-1049").await;
        let items = issue_activity(&rig.ctx.db, viewer(), id).await.unwrap();

        let priority: Vec<bool> = items
            .iter()
            .filter_map(|item| match item {
                ActivityItem::Change { field, fresh, .. } if field == "priority" => Some(*fresh),
                _ => None,
            })
            .collect();
        assert_eq!(priority, [false]);
        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
    }

    #[tokio::test]
    async fn an_excluded_type_keeps_its_on_these_issues_events_quiet_but_not_a_mention_of_me() {
        let rig = Rig::synced().await;
        rig.use_notifications(notifications(|n| n.excluded_types = vec!["Task".into()]))
            .await;
        rig.move_to_done("NW-1044");
        rig.fake.add_mention(
            "NW-1046",
            demo_user(MARTA),
            "Please check the Alpine",
            &demo_user(ALEX),
        );
        rig.move_to_done("NW-1050");
        rig.delta().await;

        assert_eq!(rig.unread_keys().await, keys(&["NW-1046", "NW-1050"]));
        assert_eq!(
            rig.note_of("NW-1046").await.as_deref(),
            Some("Mentioned you.")
        );
        assert_eq!(rig.note_of("NW-1044").await, None);
    }

    #[tokio::test]
    async fn the_involved_scope_counts_only_issues_assigned_to_or_reported_by_me() {
        let rig = Rig::synced().await;
        rig.use_notifications(notifications(|n| n.scope = NotifyScope::Involved))
            .await;
        for key in ["NW-1042", "NW-1050", "NW-1060"] {
            rig.move_to_done(key);
        }
        rig.delta().await;

        assert_eq!(rig.unread_keys().await, keys(&["NW-1042", "NW-1060"]));
    }

    #[tokio::test]
    async fn each_kind_of_involvement_counts_on_its_own() {
        let rig = Rig::synced().await;
        for key in ["NW-1042", "NW-1050", "NW-1060"] {
            rig.move_to_done(key);
        }
        rig.delta().await;
        let involved_by = |assigned, reported| {
            notifications(|n| {
                n.scope = NotifyScope::Involved;
                n.involvement = Involvement {
                    assigned,
                    reported,
                    people_fields: false,
                };
            })
        };

        for (name, settings, expected) in [
            ("assigned", involved_by(true, false), keys(&["NW-1042"])),
            ("reported", involved_by(false, true), keys(&["NW-1060"])),
            ("neither", involved_by(false, false), keys(&[])),
        ] {
            rig.use_notifications(settings).await;
            rig.resettle().await;

            assert_eq!(rig.unread_keys().await, expected, "{name}");
        }
    }

    #[tokio::test]
    async fn being_named_in_an_opted_in_people_field_counts_as_involved() {
        let rig = Rig::synced_with(&[REVIEWER]).await;
        rig.move_to_done("NW-1046");
        rig.move_to_done("NW-1050");
        rig.delta().await;
        let involved_by_people_field = |people_fields| {
            notifications(|n| {
                n.scope = NotifyScope::Involved;
                n.involvement = Involvement {
                    assigned: false,
                    reported: false,
                    people_fields,
                };
            })
        };

        for (people_fields, expected) in [(true, keys(&["NW-1046"])), (false, keys(&[]))] {
            rig.use_notifications(involved_by_people_field(people_fields))
                .await;
            rig.resettle().await;

            assert_eq!(rig.unread_keys().await, expected, "{people_fields}");
        }
    }

    #[tokio::test]
    async fn a_people_field_that_is_not_opted_in_does_not_make_me_involved() {
        let rig = Rig::synced().await;
        rig.use_notifications(notifications(|n| {
            n.scope = NotifyScope::Involved;
            n.involvement = Involvement {
                assigned: false,
                reported: false,
                people_fields: true,
            };
        }))
        .await;
        rig.move_to_done("NW-1046");
        rig.delta().await;

        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
    }

    #[tokio::test]
    async fn saving_settings_settles_what_is_recorded_at_once_in_both_directions() {
        let rig = Rig::synced().await;
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["priority"] = json!({ "id": "4", "name": "Low" });
        });
        rig.fake.add_comment(
            "NW-1042",
            demo_user(MARTA),
            "Postcode rules for Liechtenstein are merged.",
        );
        rig.delta().await;
        assert_eq!(rig.unread_keys().await, keys(&["NW-1042", "NW-1049"]));

        rig.use_notifications(notifications(|n| {
            n.in_app = every_event_but(NotifyEvent::DueOrPriority)
        }))
        .await;
        rig.resettle().await;

        assert_eq!(rig.unread_keys().await, keys(&["NW-1042"]));
        assert_eq!(rig.badge().await, 1);

        rig.use_notifications(Notifications::default()).await;
        rig.resettle().await;

        assert_eq!(rig.unread_keys().await, keys(&["NW-1042", "NW-1049"]));
        assert_eq!(rig.badge().await, 2);
    }

    #[tokio::test]
    async fn an_issue_marked_unread_stays_unread_when_all_its_changes_are_quiet() {
        let rig = Rig::synced().await;
        rig.use_notifications(notifications(|n| n.in_app = Vec::new()))
            .await;
        rig.fake.edit_issue("NW-1049", |fields| {
            fields["priority"] = json!({ "id": "4", "name": "Low" });
        });
        rig.fake.add_comment(
            "NW-1042",
            demo_user(MARTA),
            "Postcode rules for Liechtenstein are merged.",
        );
        rig.delta().await;
        assert_eq!(rig.unread_keys().await, Vec::<String>::new());

        let id = rig.id_of("NW-1049").await;
        mark_unread(&rig.ctx.db, id, None).await.unwrap();
        rig.resettle().await;

        assert_eq!(rig.unread_keys().await, keys(&["NW-1049"]));
        assert_eq!(
            rig.note_of("NW-1049").await.as_deref(),
            Some("Marked unread.")
        );
    }
}

mod being_added_to_a_people_field {
    use super::*;

    #[tokio::test]
    async fn is_recorded_makes_the_issue_unread_and_is_announced() {
        let rig = Rig::synced_with(&[REVIEWER]).await;
        let before = rig.newest_change().await;
        rig.reviewer_is("NW-1050", ALEX);
        rig.delta().await;

        assert_eq!(rig.people_changes().await, [format!("{REVIEWER}>{ALEX}")]);
        assert_eq!(rig.unread_keys().await, keys(&["NW-1050"]));
        assert_eq!(
            rig.note_of("NW-1050").await.as_deref(),
            Some("Added you as Reviewer.")
        );
        assert_eq!(
            rig.notices_after(before, &Notifications::default()).await,
            vec![notice(NW_1050.0, NW_1050.1, "Added you as Reviewer.")]
        );
    }

    #[tokio::test]
    async fn stays_recorded_but_quiet_when_its_event_is_unticked_in_app() {
        let rig = Rig::synced_with(&[REVIEWER]).await;
        rig.use_notifications(notifications(|n| {
            n.in_app = every_event_but(NotifyEvent::AddsMeToPeopleField)
        }))
        .await;
        rig.reviewer_is("NW-1050", ALEX);
        rig.delta().await;

        assert_eq!(rig.people_changes().await, [format!("{REVIEWER}>{ALEX}")]);
        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
    }

    #[tokio::test]
    async fn records_nothing_for_a_field_that_is_not_opted_in() {
        let rig = Rig::synced().await;
        rig.reviewer_is("NW-1050", ALEX);
        rig.delta().await;

        assert_eq!(rig.people_changes().await, Vec::<String>::new());
        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
    }

    #[tokio::test]
    async fn records_nothing_for_an_opted_in_field_that_is_not_a_user_field() {
        let rig = Rig::synced_with(&[STORY_POINTS]).await;
        rig.fake.edit_issue("NW-1050", |fields| {
            fields[STORY_POINTS] = json!({ "accountId": ALEX });
        });
        rig.delta().await;

        assert_eq!(rig.people_changes().await, Vec::<String>::new());
        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
    }

    #[tokio::test]
    async fn records_nothing_when_i_am_removed_from_the_field() {
        let rig = Rig::synced_with(&[REVIEWER]).await;
        rig.reviewer_is("NW-1046", MARTA);
        rig.delta().await;

        assert_eq!(rig.people_changes().await, Vec::<String>::new());
        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
    }
}
