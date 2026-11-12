//! Changes made by automations (app accounts) on a synced demo cache: who each recorded change names as its
//! author, which changes make an issue unread or notify, and how the post-sync pass fills in authors a search's
//! partial changelog couldn't tell. Changes are made on the fake Jira before a delta sync.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use tempfile::TempDir;

use taskboard_lib::changes::UNSEEN;
use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::inbox;
use taskboard_lib::issues::{ActivityItem, issue_activity};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::Viewer;
use taskboard_lib::notify::rules::resettle;
use taskboard_lib::notify::{Notice, notices};
use taskboard_lib::store::settings::{Notifications, NotifyEvent, SettingsStore};
use taskboard_lib::sync::{Emit, Job, SyncContext, full_sync, run_job};
use taskboard_lib::test_support::demo::{self, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};

const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const ALEX: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const MARTA: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03";
/// The demo's test management app.
const APP: &str = "557058:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e90";
const DONE: &str = "10001";
const TEN: &str = "2026-10-28T10:00:00.000Z";
/// What the fake's search embeds of an issue's changelog, at most.
const EMBEDDED_HISTORIES: usize = 100;

fn viewer() -> Viewer {
    Viewer {
        me: Some(ALEX.into()),
        time_zone: Some("Europe/Berlin".into()),
        now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
    }
}

fn later() -> DateTime<Utc> {
    DEMO_NOW.parse::<DateTime<Utc>>().unwrap() + chrono::Duration::hours(2)
}

fn notifications(change: impl FnOnce(&mut Notifications)) -> Notifications {
    let mut settings = Notifications::default();
    change(&mut settings);
    settings
}

fn keys(found: &[&str]) -> Vec<String> {
    found.iter().map(|key| key.to_string()).collect()
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

/// A changelog item for `field`, which is all the sync reads of it.
fn changelog_item(field: &str) -> Value {
    json!({
        "field": field,
        "fieldId": field,
        "fieldtype": "jira",
        "from": null,
        "fromString": null,
        "to": null,
        "toString": null
    })
}

/// Old entries by someone else on another field; with the app's entry after them, the issue's changelog is
/// longer than a search embeds.
fn history_beyond_the_embedded_ones() -> Vec<Value> {
    let marta = demo_user(MARTA);
    (0..EMBEDDED_HISTORIES)
        .map(|n| {
            json!({
                "id": format!("older-{n}"),
                "author": marta,
                "created": "2026-10-20T08:00:00.000+0000",
                "items": [changelog_item("labels")],
            })
        })
        .collect()
}

fn notice(key: &str, summary: &str, body: &str) -> Notice {
    Notice {
        title: format!("{key} {summary}"),
        body: body.into(),
        key: Some(key.into()),
    }
}

struct Rig {
    fake: FakeJira,
    _dir: TempDir,
    ctx: SyncContext,
}

impl Rig {
    /// Connected to the demo site as the demo owner and synced; every issue counts as read at 10:00 and the
    /// fake's clock stands two hours later for what the test changes.
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

    async fn badge(&self) -> u32 {
        self.ctx.db.read(inbox::count).await.unwrap()
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

    /// The issue's recorded changes, oldest first, as `field>author`; `unknown` for a change without one.
    async fn authors(&self, key: &str) -> Vec<String> {
        self.strings(format!(
            "SELECT c.field || '>' || COALESCE(c.author_id, 'unknown') FROM cache.issue_change c
             JOIN cache.issue i ON i.id = c.issue_id WHERE i.key = '{key}' ORDER BY c.id"
        ))
        .await
    }

    /// The recorded changes of one kind, oldest first, as `key>author`.
    async fn authors_of_kind(&self, field: &str) -> Vec<String> {
        self.strings(format!(
            "SELECT i.key || '>' || COALESCE(c.author_id, 'unknown') FROM cache.issue_change c
             JOIN cache.issue i ON i.id = c.issue_id WHERE c.field = '{field}' ORDER BY c.id"
        ))
        .await
    }

    /// `1` for a change that doesn't count in the app, `0` for one that does, `unsettled` before it is judged.
    async fn quiet_of(&self, key: &str) -> Vec<String> {
        self.strings(format!(
            "SELECT COALESCE(CAST(c.quiet AS TEXT), 'unsettled') FROM cache.issue_change c
             JOIN cache.issue i ON i.id = c.issue_id WHERE i.key = '{key}' ORDER BY c.id"
        ))
        .await
    }

    async fn issues_with_history(&self) -> Vec<String> {
        self.strings(
            "SELECT key FROM cache.issue WHERE history_synced_at IS NOT NULL ORDER BY key".into(),
        )
        .await
    }

    async fn activity(&self, key: &str) -> Vec<ActivityItem> {
        let id = self.id_of(key).await;
        issue_activity(&self.ctx.db, viewer(), id).await.unwrap()
    }

    async fn delta(&self) {
        let emit: Emit = Arc::new(|_| {});
        run_job(&self.ctx, Job::Delta, &*emit, None).await;
    }

    /// Saves the page the way `notifications_save` does: the settings, then every recorded change judged again.
    async fn use_notifications(&self, notifications: Notifications) {
        self.ctx
            .settings
            .edit(move |settings| settings.notifications = notifications)
            .await
            .unwrap();
        resettle(&self.ctx.db, &self.ctx.settings.get())
            .await
            .unwrap();
    }

    fn app_moves_to_done(&self, key: &str) {
        self.moves_to_done_as(key, demo_user(APP));
    }

    fn moves_to_done_as(&self, key: &str, author: Value) {
        self.fake
            .edit_issue_as(key, author, vec![changelog_item("status")], |fields| {
                fields["status"] = demo_status(DONE);
            });
    }

    fn lowers_priority_as(&self, key: &str, author: Value) {
        self.fake
            .edit_issue_as(key, author, vec![changelog_item("priority")], |fields| {
                fields["priority"] = json!({ "id": "4", "name": "Low" });
            });
    }

    fn app_assigns_to_me(&self, key: &str) {
        self.fake.edit_issue_as(
            key,
            demo_user(APP),
            vec![changelog_item("assignee")],
            |fields| fields["assignee"] = demo_user(ALEX),
        );
    }

    /// The test management app moved NW-1044 to Done, lowered the priority of NW-1049 and commented on NW-1046.
    fn the_app_runs(&self) {
        self.app_moves_to_done("NW-1044");
        self.lowers_priority_as("NW-1049", demo_user(APP));
        self.fake
            .add_comment("NW-1046", demo_user(APP), "Run 12 passed on staging.");
    }
}

const EVERY_EVENT_ON_THE_MAC: fn(&mut Notifications) = |n| n.mac = NotifyEvent::ALL.to_vec();

mod the_cache {
    use super::*;

    #[tokio::test]
    async fn has_a_column_for_the_author_of_a_change_and_one_for_the_type_of_an_account() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();

        let (change_columns, user_columns) = db
            .read(|conn| {
                let columns = |table: &str| -> rusqlite::Result<Vec<String>> {
                    let mut query = conn.prepare(&format!(
                        "SELECT name FROM pragma_table_info('{table}', 'cache')"
                    ))?;
                    let names = query
                        .query_map([], |r| r.get::<_, String>(0))?
                        .collect::<Result<Vec<_>, _>>();
                    names
                };
                Ok((columns("issue_change")?, columns("user")?))
            })
            .await
            .unwrap();

        assert!(
            change_columns.contains(&"author_id".to_string()),
            "{change_columns:?}"
        );
        assert!(
            user_columns.contains(&"account_type".to_string()),
            "{user_columns:?}"
        );
    }

    #[tokio::test]
    async fn keeps_the_type_of_each_account_a_sync_has_seen() {
        let rig = Rig::synced().await;
        let type_of = |account: &'static str| {
            format!(
                "SELECT COALESCE(account_type, 'unknown') FROM cache.user WHERE account_id = '{account}'"
            )
        };
        assert_eq!(rig.strings(type_of(ALEX)).await, ["atlassian"]);

        rig.the_app_runs();
        rig.delta().await;

        assert_eq!(rig.strings(type_of(APP)).await, ["app"]);
        assert_eq!(rig.strings(type_of(ALEX)).await, ["atlassian"]);
    }
}

mod changes_by_an_automation {
    use super::*;

    #[tokio::test]
    async fn are_recorded_with_its_account_as_author() {
        let rig = Rig::synced().await;
        rig.the_app_runs();
        rig.delta().await;

        assert_eq!(rig.authors("NW-1044").await, [format!("status>{APP}")]);
        assert_eq!(rig.authors("NW-1049").await, [format!("priority>{APP}")]);
        assert_eq!(
            rig.authors("NW-1046").await,
            [format!("comment_added>{APP}")]
        );
    }

    #[tokio::test]
    async fn leave_the_issues_read_the_inbox_unchanged_and_announce_nothing() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        let badge = rig.badge().await;
        rig.the_app_runs();
        rig.delta().await;

        let announced = rig
            .notices_after(before, &notifications(EVERY_EVENT_ON_THE_MAC))
            .await;

        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
        assert_eq!(rig.badge().await, badge);
        assert_eq!(announced, vec![]);
        for key in ["NW-1044", "NW-1049", "NW-1046"] {
            assert_eq!(rig.quiet_of(key).await, ["1"], "{key}");
        }
    }

    #[tokio::test]
    async fn still_show_in_the_activity_of_the_issue_as_old_news() {
        let rig = Rig::synced().await;
        let comments = |items: &[ActivityItem]| {
            items
                .iter()
                .filter(|item| matches!(item, ActivityItem::Comment { .. }))
                .count()
        };
        let comments_before = comments(&rig.activity("NW-1046").await);
        rig.the_app_runs();
        rig.delta().await;

        let priority: Vec<bool> = rig
            .activity("NW-1049")
            .await
            .iter()
            .filter_map(|item| match item {
                ActivityItem::Change { field, fresh, .. } if field == "priority" => Some(*fresh),
                _ => None,
            })
            .collect();
        let comments_after = comments(&rig.activity("NW-1046").await);

        assert_eq!(priority, [false]);
        assert_eq!(comments_after, comments_before + 1);
    }

    #[tokio::test]
    async fn make_the_issues_unread_at_once_when_the_setting_is_turned_off_and_read_again_when_turned_on()
     {
        let rig = Rig::synced().await;
        rig.the_app_runs();
        rig.delta().await;
        assert_eq!(rig.unread_keys().await, Vec::<String>::new());

        rig.use_notifications(notifications(|n| n.ignore_automations = false))
            .await;

        assert_eq!(
            rig.unread_keys().await,
            keys(&["NW-1044", "NW-1046", "NW-1049"])
        );
        assert_eq!(rig.badge().await, 3);

        rig.use_notifications(Notifications::default()).await;

        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
        assert_eq!(rig.badge().await, 0);
    }

    #[tokio::test]
    async fn count_from_the_start_when_the_setting_is_off_for_the_sync() {
        let rig = Rig::synced().await;
        rig.use_notifications(notifications(|n| n.ignore_automations = false))
            .await;
        rig.the_app_runs();
        rig.delta().await;

        assert_eq!(
            rig.unread_keys().await,
            keys(&["NW-1044", "NW-1046", "NW-1049"])
        );
    }

    #[tokio::test]
    async fn that_assign_or_mention_the_owner_still_make_the_issue_unread_and_notify() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.app_assigns_to_me("NW-1044");
        rig.fake.add_mention(
            "NW-1046",
            demo_user(APP),
            "Please review run 12",
            &demo_user(ALEX),
        );
        rig.delta().await;

        let announced = rig.notices_after(before, &Notifications::default()).await;

        assert_eq!(rig.unread_keys().await, keys(&["NW-1044", "NW-1046"]));
        assert_eq!(
            announced,
            vec![
                notice(
                    "NW-1044",
                    "Lazy-load product gallery images",
                    "Assigned to you."
                ),
                notice(
                    "NW-1046",
                    "Preload critical fonts on landing pages",
                    "Mentioned you."
                ),
            ]
        );
        assert_eq!(rig.authors("NW-1044").await, [format!("assignee>{APP}")]);
    }

    #[tokio::test]
    async fn that_mention_someone_else_stay_quiet() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.fake.add_mention(
            "NW-1046",
            demo_user(APP),
            "Marta, run 12 failed",
            &demo_user(MARTA),
        );
        rig.delta().await;

        let announced = rig
            .notices_after(before, &notifications(EVERY_EVENT_ON_THE_MAC))
            .await;

        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
        assert_eq!(announced, vec![]);
    }
}

mod changes_by_a_person {
    use super::*;

    #[tokio::test]
    async fn still_make_the_issues_unread_and_are_recorded_with_their_author() {
        let rig = Rig::synced().await;
        let before = rig.newest_change().await;
        rig.moves_to_done_as("NW-1044", demo_user(MARTA));
        rig.lowers_priority_as("NW-1049", demo_user(MARTA));
        rig.fake
            .add_comment("NW-1046", demo_user(MARTA), "Run 12 passed on staging.");
        rig.delta().await;

        assert_eq!(rig.authors("NW-1044").await, [format!("status>{MARTA}")]);
        assert_eq!(rig.authors("NW-1049").await, [format!("priority>{MARTA}")]);
        assert_eq!(
            rig.authors("NW-1046").await,
            [format!("comment_added>{MARTA}")]
        );
        assert_eq!(
            rig.unread_keys().await,
            keys(&["NW-1044", "NW-1046", "NW-1049"])
        );
        assert_eq!(
            rig.notices_after(before, &notifications(EVERY_EVENT_ON_THE_MAC))
                .await
                .len(),
            3
        );
    }
}

mod the_author_of {
    use super::*;

    #[tokio::test]
    async fn a_created_issue_is_its_reporter() {
        let rig = Rig::synced().await;
        let by_person =
            rig.fake
                .add_issue("NW-1055", "NW", "Order confirmation email: shipping notice");
        let by_app = rig.fake.add_issue("NW-1055", "NW", "Packing slip layout");
        rig.fake.edit_issue(&by_person, |fields| {
            fields["reporter"] = demo_user(MARTA);
        });
        rig.fake.edit_issue(&by_app, |fields| {
            fields["reporter"] = demo_user(APP);
        });
        rig.delta().await;

        assert_eq!(
            rig.authors_of_kind("created").await,
            [format!("{by_person}>{MARTA}"), format!("{by_app}>{APP}")]
        );
        assert_eq!(rig.unread_keys().await, vec![by_person]);
    }

    #[tokio::test]
    async fn a_mention_is_the_author_of_the_comment() {
        let rig = Rig::synced().await;
        rig.fake.add_mention(
            "NW-1044",
            demo_user(MARTA),
            "Please check the Alpine",
            &demo_user(ALEX),
        );
        rig.fake.add_mention(
            "NW-1046",
            demo_user(APP),
            "Please review run 12",
            &demo_user(ALEX),
        );
        rig.delta().await;

        assert_eq!(
            rig.authors_of_kind("mention").await,
            [format!("NW-1044>{MARTA}"), format!("NW-1046>{APP}")]
        );
    }
}

mod a_changelog_the_search_could_not_embed_whole {
    use super::*;

    #[tokio::test]
    async fn is_fetched_after_the_sync_which_names_the_author_and_settles_the_change_quiet() {
        let rig = Rig::synced().await;
        rig.fake
            .set_changelog("NW-1049", history_beyond_the_embedded_ones());
        rig.lowers_priority_as("NW-1049", demo_user(APP));
        rig.app_moves_to_done("NW-1044");
        let with_history = rig.issues_with_history().await;
        rig.delta().await;

        assert_eq!(rig.authors("NW-1049").await, [format!("priority>{APP}")]);
        assert_eq!(rig.quiet_of("NW-1049").await, ["1"]);
        assert_eq!(rig.unread_keys().await, Vec::<String>::new());
        assert_eq!(rig.badge().await, 0);
        let mut expected = with_history;
        expected.push("NW-1049".into());
        expected.sort();
        assert_eq!(rig.issues_with_history().await, expected);
    }

    #[tokio::test]
    async fn skips_an_issue_deleted_since_the_search_and_still_names_the_authors_of_the_others() {
        let rig = Rig::synced().await;
        for key in ["NW-1049", "NW-1044"] {
            rig.fake
                .set_changelog(key, history_beyond_the_embedded_ones());
        }
        rig.lowers_priority_as("NW-1049", demo_user(APP));
        rig.fake.advance(std::time::Duration::from_secs(60));
        rig.app_moves_to_done("NW-1044");
        rig.fake.delete_issue_after_search("NW-1044");
        let with_history = rig.issues_with_history().await;
        rig.delta().await;

        assert_eq!(rig.authors("NW-1044").await, ["status>unknown"]);
        assert_eq!(rig.authors("NW-1049").await, [format!("priority>{APP}")]);
        assert_eq!(rig.quiet_of("NW-1049").await, ["1"]);
        assert!(!rig.unread_keys().await.contains(&"NW-1049".to_string()));
        let mut expected = with_history;
        expected.push("NW-1049".into());
        expected.sort();
        assert_eq!(rig.issues_with_history().await, expected);
    }

    #[tokio::test]
    async fn names_a_person_who_made_the_change_so_it_counts() {
        let rig = Rig::synced().await;
        rig.fake
            .set_changelog("NW-1049", history_beyond_the_embedded_ones());
        rig.lowers_priority_as("NW-1049", demo_user(MARTA));
        rig.delta().await;

        assert_eq!(rig.authors("NW-1049").await, [format!("priority>{MARTA}")]);
        assert_eq!(rig.unread_keys().await, keys(&["NW-1049"]));
    }
}

mod the_owners_own_field_change {
    use super::*;

    #[tokio::test]
    async fn is_dropped_when_the_search_embeds_the_whole_changelog() {
        let rig = Rig::synced().await;
        let with_history = rig.issues_with_history().await;
        rig.moves_to_done_as("NW-1044", demo_user(ALEX));
        rig.lowers_priority_as("NW-1049", demo_user(MARTA));
        rig.delta().await;

        assert_eq!(rig.authors("NW-1044").await, Vec::<String>::new());
        assert_eq!(rig.authors("NW-1049").await, [format!("priority>{MARTA}")]);
        assert_eq!(rig.unread_keys().await, keys(&["NW-1049"]));
        assert_eq!(rig.issues_with_history().await, with_history);
    }
}
