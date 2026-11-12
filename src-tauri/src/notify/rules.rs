//! Which recorded changes count (Settings → Notifications): each `issue_change` row is one `NotifyEvent`, and the
//! settings decide per channel whether it counts. In app, a change that doesn't count is stored `quiet`, so it
//! never makes its issue unread (`changes::UNSEEN`); macOS asks `counts` directly (`notices`).

use rusqlite::{Connection, params};

use super::people::{named_in, people_field_ids};
use crate::changes::Change;
use crate::db::{Changes, Db};
use crate::error::AppResult;
use crate::store::settings::{Channel, Notifications, NotifyEvent, NotifyScope, Settings};

/// What a change's issue is to the owner.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Subject {
    pub issue_type: String,
    pub assigned_to_me: bool,
    pub reported_by_me: bool,
    pub in_my_people_field: bool,
}

/// Status categories around a `status` change: Jira's `new`, `indeterminate` or `done`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Categories {
    pub from: Option<String>,
    pub to: Option<String>,
}

/// The row of the Notifications page a change belongs to; none for a change that never counts (a mention of
/// someone else, a people field naming someone else, an issue gone).
pub fn event_of(change: &Change, me: Option<&str>, categories: &Categories) -> Option<NotifyEvent> {
    let is_me = |value: &Option<String>| me.is_some() && value.as_deref() == me;
    Some(match change.field.as_str() {
        "mention" if is_me(&change.new_value) => NotifyEvent::MentionsMe,
        "people" if is_me(&change.new_value) => NotifyEvent::AddsMeToPeopleField,
        "mention" | "people" | "gone" => return None,
        "assignee" if is_me(&change.new_value) => NotifyEvent::AssignsMe,
        "assignee" if is_me(&change.old_value) => NotifyEvent::UnassignsMe,
        "comment_added" => NotifyEvent::NewComment,
        "status" => status_event(categories),
        "due_date" | "priority" => NotifyEvent::DueOrPriority,
        "created" => NotifyEvent::Created,
        _ => NotifyEvent::OtherChange,
    })
}

fn status_event(categories: &Categories) -> NotifyEvent {
    let (from, to) = (categories.from.as_deref(), categories.to.as_deref());
    match to {
        Some("done") if from != Some("done") => NotifyEvent::MovedToDone,
        Some("new") if from != Some("new") => NotifyEvent::Reopened,
        _ => NotifyEvent::OtherChange,
    }
}

/// Whether `event` on `subject` shows in `channel`: events about the owner pass the scope and type filters.
pub fn counts(
    settings: &Notifications,
    channel: Channel,
    event: NotifyEvent,
    subject: &Subject,
) -> bool {
    if !settings.events(channel).contains(&event) {
        return false;
    }
    if event.is_about_me() {
        return true;
    }
    if settings.excluded_types.contains(&subject.issue_type) {
        return false;
    }
    match settings.scope {
        NotifyScope::AllIssues => true,
        NotifyScope::Involved => {
            let how = &settings.involvement;
            (how.assigned && subject.assigned_to_me)
                || (how.reported && subject.reported_by_me)
                || (how.people_fields && subject.in_my_people_field)
        }
    }
}

/// A recorded change with what it takes to judge it.
pub struct Judged {
    pub id: i64,
    pub issue_id: String,
    /// As stored; none when not settled yet.
    pub quiet: Option<bool>,
    pub change: Change,
    pub subject: Subject,
    pub categories: Categories,
}

impl Judged {
    pub fn counts(&self, settings: &Notifications, channel: Channel, me: Option<&str>) -> bool {
        event_of(&self.change, me, &self.categories)
            .is_some_and(|event| counts(settings, channel, event, &self.subject))
    }
}

/// Which recorded changes `judged` reads.
#[derive(Clone, Copy)]
pub enum Which {
    Unsettled,
    All,
    /// Recorded after this `issue_change` id.
    After(i64),
}

/// The recorded changes of cached issues, in the order they were recorded.
pub fn judged(conn: &Connection, which: Which, me: Option<&str>) -> AppResult<Vec<Judged>> {
    let (after, unsettled) = match which {
        Which::Unsettled => (0, "AND c.quiet IS NULL"),
        Which::All => (0, ""),
        Which::After(after) => (after, ""),
    };
    let people = people_field_ids(conn)?;
    let mut query = conn.prepare(&format!(
        "SELECT c.id, c.issue_id, c.quiet, c.field, c.old_value, c.new_value, c.changed_at, i.issue_type,
           COALESCE(i.assignee_id = ?1, 0), COALESCE(i.reporter_id = ?1, 0), i.custom,
           (SELECT category FROM cache.status WHERE id = c.old_value AND c.field = 'status'),
           (SELECT category FROM cache.status WHERE id = c.new_value AND c.field = 'status')
         FROM cache.issue_change c JOIN cache.issue i ON i.id = c.issue_id
         WHERE c.id > ?2 {unsettled}
         ORDER BY c.id"
    ))?;
    let rows = query.query_map(params![me, after], |r| {
        let custom: Option<String> = r.get(10)?;
        Ok(Judged {
            id: r.get(0)?,
            issue_id: r.get(1)?,
            quiet: r.get(2)?,
            change: Change {
                field: r.get(3)?,
                old_value: r.get(4)?,
                new_value: r.get(5)?,
                at: r.get(6)?,
            },
            subject: Subject {
                issue_type: r.get(7)?,
                assigned_to_me: r.get(8)?,
                reported_by_me: r.get(9)?,
                in_my_people_field: me
                    .is_some_and(|me| !named_in(custom.as_deref(), me, &people).is_empty()),
            },
            categories: Categories {
                from: r.get(11)?,
                to: r.get(12)?,
            },
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Decides `quiet` for the changes not decided yet, or for all of them when `again` (the settings or the
/// connected account changed). An issue whose unread state may have flipped counts as changed.
pub fn settle(
    conn: &Connection,
    changes: &mut Changes,
    settings: &Notifications,
    me: Option<&str>,
    again: bool,
) -> AppResult<()> {
    let which = if again { Which::All } else { Which::Unsettled };
    let mut update =
        conn.prepare_cached("UPDATE cache.issue_change SET quiet = ?2 WHERE id = ?1")?;
    for judged in judged(conn, which, me)? {
        let quiet = !judged.counts(settings, Channel::InApp, me);
        if judged.quiet == Some(quiet) {
            continue;
        }
        update.execute(params![judged.id, quiet])?;
        if judged.quiet.unwrap_or(false) != quiet {
            changes.issue_changed(judged.issue_id);
        }
    }
    Ok(())
}

/// `settle` everything again, for new settings or another connected account.
pub async fn resettle(db: &Db, settings: &Settings) -> AppResult<()> {
    let notifications = settings.notifications.clone();
    let me = settings.connection.account_id.clone();
    db.write(move |tx, changes| settle(tx, changes, &notifications, me.as_deref(), true))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::settings::Involvement;
    use crate::test_support::temp_config_dir;
    use std::collections::BTreeSet;
    use tempfile::TempDir;

    const ME: &str = "account-me";
    const MARTA: &str = "account-marta";

    fn change(field: &str, old: Option<&str>, new: Option<&str>) -> Change {
        Change {
            field: field.into(),
            old_value: old.map(String::from),
            new_value: new.map(String::from),
            at: "2026-10-28T09:00:00.000Z".into(),
        }
    }

    fn categories(from: Option<&str>, to: Option<&str>) -> Categories {
        Categories {
            from: from.map(String::from),
            to: to.map(String::from),
        }
    }

    fn event(field: &str, old: Option<&str>, new: Option<&str>) -> Option<NotifyEvent> {
        event_of(&change(field, old, new), Some(ME), &Categories::default())
    }

    #[test]
    fn a_mention_or_people_field_naming_me_is_about_me_and_one_naming_someone_else_never_counts() {
        assert_eq!(
            event("mention", Some("90001"), Some(ME)),
            Some(NotifyEvent::MentionsMe)
        );
        assert_eq!(event("mention", Some("90001"), Some(MARTA)), None);
        assert_eq!(
            event("people", Some("customfield_10042"), Some(ME)),
            Some(NotifyEvent::AddsMeToPeopleField)
        );
        assert_eq!(event("people", Some("customfield_10042"), Some(MARTA)), None);
    }

    #[test]
    fn without_a_connected_account_nothing_is_about_me() {
        let none = Categories::default();

        assert_eq!(
            event_of(&change("mention", Some("90001"), Some(ME)), None, &none),
            None
        );
        assert_eq!(
            event_of(&change("assignee", None, None), None, &none),
            Some(NotifyEvent::OtherChange)
        );
        assert_eq!(
            event_of(&change("assignee", None, Some(ME)), None, &none),
            Some(NotifyEvent::OtherChange)
        );
    }

    #[test]
    fn an_assignee_change_is_to_me_from_me_or_another_change() {
        let cases = [
            (None, Some(ME), NotifyEvent::AssignsMe),
            (Some(MARTA), Some(ME), NotifyEvent::AssignsMe),
            (Some(ME), Some(MARTA), NotifyEvent::UnassignsMe),
            (Some(ME), None, NotifyEvent::UnassignsMe),
            (None, Some(MARTA), NotifyEvent::OtherChange),
            (Some(MARTA), None, NotifyEvent::OtherChange),
        ];
        for (old, new, expected) in cases {
            assert_eq!(
                event("assignee", old, new),
                Some(expected),
                "{old:?} -> {new:?}"
            );
        }
    }

    #[test]
    fn other_fields_map_to_their_rows_and_unknown_ones_to_other_change() {
        let cases = [
            ("comment_added", Some(NotifyEvent::NewComment)),
            ("due_date", Some(NotifyEvent::DueOrPriority)),
            ("priority", Some(NotifyEvent::DueOrPriority)),
            ("created", Some(NotifyEvent::Created)),
            ("gone", None),
            ("updated", Some(NotifyEvent::OtherChange)),
            ("sprint", Some(NotifyEvent::OtherChange)),
        ];
        for (field, expected) in cases {
            assert_eq!(event(field, None, None), expected, "{field}");
        }
    }

    #[test]
    fn a_status_change_goes_by_the_category_it_moves_between() {
        let cases = [
            (Some("new"), Some("done"), NotifyEvent::MovedToDone),
            (
                Some("indeterminate"),
                Some("done"),
                NotifyEvent::MovedToDone,
            ),
            (None, Some("done"), NotifyEvent::MovedToDone),
            (Some("done"), Some("new"), NotifyEvent::Reopened),
            (
                Some("indeterminate"),
                Some("new"),
                NotifyEvent::Reopened,
            ),
            (None, Some("new"), NotifyEvent::Reopened),
            (Some("new"), Some("indeterminate"), NotifyEvent::OtherChange),
            (Some("done"), Some("indeterminate"), NotifyEvent::OtherChange),
            (Some("new"), Some("new"), NotifyEvent::OtherChange),
            (Some("done"), Some("done"), NotifyEvent::OtherChange),
            (Some("new"), None, NotifyEvent::OtherChange),
            (None, None, NotifyEvent::OtherChange),
        ];
        for (from, to, expected) in cases {
            assert_eq!(
                event_of(
                    &change("status", Some("1"), Some("2")),
                    Some(ME),
                    &categories(from, to)
                ),
                Some(expected),
                "{from:?} -> {to:?}"
            );
        }
    }

    fn all_events() -> Notifications {
        Notifications {
            mac: NotifyEvent::ALL.to_vec(),
            ..Notifications::default()
        }
    }

    fn story() -> Subject {
        Subject {
            issue_type: "Story".into(),
            ..Subject::default()
        }
    }

    #[test]
    fn an_event_not_ticked_in_the_channel_does_not_count() {
        let settings = Notifications {
            in_app: vec![NotifyEvent::NewComment],
            mac: vec![NotifyEvent::Created],
            ..Notifications::default()
        };

        for (channel, event, expected) in [
            (Channel::InApp, NotifyEvent::NewComment, true),
            (Channel::InApp, NotifyEvent::Created, false),
            (Channel::Mac, NotifyEvent::Created, true),
            (Channel::Mac, NotifyEvent::NewComment, false),
            (Channel::InApp, NotifyEvent::MentionsMe, false),
        ] {
            assert_eq!(
                counts(&settings, channel, event, &story()),
                expected,
                "{channel:?} {event:?}"
            );
        }
    }

    #[test]
    fn events_about_me_pass_the_excluded_types_and_the_involved_scope() {
        let settings = Notifications {
            scope: NotifyScope::Involved,
            excluded_types: vec!["Story".into()],
            ..all_events()
        };

        for event in NotifyEvent::ALL.into_iter().filter(|e| e.is_about_me()) {
            for channel in [Channel::InApp, Channel::Mac] {
                assert!(
                    counts(&settings, channel, event, &story()),
                    "{channel:?} {event:?}"
                );
            }
        }
    }

    #[test]
    fn an_excluded_type_blocks_the_other_events_only() {
        let settings = Notifications {
            excluded_types: vec!["Test Case".into()],
            ..all_events()
        };
        let test_case = Subject {
            issue_type: "Test Case".into(),
            ..Subject::default()
        };

        assert!(!counts(
            &settings,
            Channel::InApp,
            NotifyEvent::NewComment,
            &test_case
        ));
        assert!(!counts(
            &settings,
            Channel::Mac,
            NotifyEvent::Created,
            &test_case
        ));
        assert!(counts(
            &settings,
            Channel::InApp,
            NotifyEvent::NewComment,
            &story()
        ));
    }

    #[test]
    fn all_issues_counts_issues_the_owner_has_nothing_to_do_with() {
        let settings = Notifications {
            scope: NotifyScope::AllIssues,
            ..all_events()
        };

        assert!(counts(
            &settings,
            Channel::InApp,
            NotifyEvent::DueOrPriority,
            &story()
        ));
    }

    #[test]
    fn the_involved_scope_counts_only_through_a_ticked_involvement() {
        let subjects = [
            ("assigned", true, false, false),
            ("reported", false, true, false),
            ("people field", false, false, true),
        ];
        let ticks = |assigned, reported, people_fields| Involvement {
            assigned,
            reported,
            people_fields,
        };
        for (index, (label, assigned, reported, people)) in subjects.into_iter().enumerate() {
            let subject = Subject {
                assigned_to_me: assigned,
                reported_by_me: reported,
                in_my_people_field: people,
                ..story()
            };
            for (name, on, expected) in [
                ("ticked", [index == 0, index == 1, index == 2], true),
                ("others ticked", [index != 0, index != 1, index != 2], false),
                ("none ticked", [false; 3], false),
                ("all ticked", [true; 3], true),
            ] {
                let settings = Notifications {
                    scope: NotifyScope::Involved,
                    involvement: ticks(on[0], on[1], on[2]),
                    ..all_events()
                };

                assert_eq!(
                    counts(&settings, Channel::InApp, NotifyEvent::NewComment, &subject),
                    expected,
                    "{label}, {name}"
                );
            }
        }
    }

    #[test]
    fn the_involved_scope_does_not_count_an_issue_the_owner_is_not_involved_in() {
        let settings = Notifications {
            scope: NotifyScope::Involved,
            ..all_events()
        };

        assert!(!counts(
            &settings,
            Channel::Mac,
            NotifyEvent::NewComment,
            &story()
        ));
    }

    const SEED: &str = "
        INSERT INTO cache.space (id, key, name, raw) VALUES ('10001', 'NW', 'Northwind', '{}');
        INSERT INTO cache.status (id, name, category) VALUES
          ('1', 'To Do', 'new'), ('2', 'In Progress', 'indeterminate'), ('3', 'Done', 'done');
        INSERT INTO cache.field (id, name, type, items) VALUES
          ('customfield_10042', 'Developer', 'user', NULL);
        INSERT INTO field_choice (field_id, in_list, in_detail) VALUES ('customfield_10042', 1, 1);
        INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, status_category,
            assignee_id, reporter_id, custom, created_at, updated_at, synced_at) VALUES
          ('1', 'NW-1', '10001', 'First', 'Story', '1', 'new', 'account-me', NULL, NULL,
           '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z'),
          ('2', 'NW-2', '10001', 'Second', 'Task', '1', 'new', NULL, 'account-me', NULL,
           '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z'),
          ('3', 'NW-3', '10001', 'Third', 'Task', '1', 'new', 'account-marta', 'account-marta',
           '{\"customfield_10042\":{\"accountId\":\"account-me\"}}',
           '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z'),
          ('4', 'NW-4', '10001', 'Fourth', 'Bug', '1', 'new', 'account-marta', 'account-marta', NULL,
           '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z');
        INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at) VALUES
          ('1', 'status', '1', '3', '2026-10-28T09:00:00Z'),
          ('2', 'comment_added', '1', '2', '2026-10-28T09:01:00Z'),
          ('3', 'created', NULL, 'NW-3', '2026-10-28T09:02:00Z'),
          ('4', 'comment_added', '0', '1', '2026-10-28T09:03:00Z'),
          ('4', 'mention', '90001', 'account-me', '2026-10-28T09:03:00Z');";

    async fn seeded() -> (TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| {
            tx.execute_batch(SEED)?;
            Ok(())
        })
        .await
        .unwrap();
        (dir, db)
    }

    async fn judged_rows(db: &Db, which: Which) -> Vec<Judged> {
        db.read(move |conn| judged(conn, which, Some(ME)))
            .await
            .unwrap()
    }

    async fn quiet_marks(db: &Db) -> Vec<Option<bool>> {
        db.read(|conn| {
            let mut query = conn.prepare("SELECT quiet FROM cache.issue_change ORDER BY id")?;
            let marks: Vec<Option<bool>> = query
                .query_map([], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            Ok(marks)
        })
        .await
        .unwrap()
    }

    async fn settle_all(
        db: &Db,
        settings: Notifications,
        again: bool,
    ) -> BTreeSet<String> {
        db.write(move |tx, changes| {
            settle(tx, changes, &settings, Some(ME), again)?;
            Ok(changes.issues.clone())
        })
        .await
        .unwrap()
    }

    fn ids(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    #[tokio::test]
    async fn judged_reads_each_change_with_what_the_issue_is_to_me_and_its_status_categories() {
        let (_dir, db) = seeded().await;

        let rows = judged_rows(&db, Which::All).await;

        let seen: Vec<_> = rows
            .iter()
            .map(|j| {
                (
                    j.issue_id.as_str(),
                    j.change.field.as_str(),
                    j.quiet,
                    j.subject.clone(),
                    j.categories.clone(),
                )
            })
            .collect();
        let subject = |issue_type: &str, assigned, reported, people| Subject {
            issue_type: issue_type.into(),
            assigned_to_me: assigned,
            reported_by_me: reported,
            in_my_people_field: people,
        };
        assert_eq!(
            seen,
            [
                (
                    "1",
                    "status",
                    None,
                    subject("Story", true, false, false),
                    categories(Some("new"), Some("done"))
                ),
                (
                    "2",
                    "comment_added",
                    None,
                    subject("Task", false, true, false),
                    Categories::default()
                ),
                (
                    "3",
                    "created",
                    None,
                    subject("Task", false, false, true),
                    Categories::default()
                ),
                (
                    "4",
                    "comment_added",
                    None,
                    subject("Bug", false, false, false),
                    Categories::default()
                ),
                (
                    "4",
                    "mention",
                    None,
                    subject("Bug", false, false, false),
                    Categories::default()
                ),
            ]
        );
        let ids: Vec<i64> = rows.iter().map(|j| j.id).collect();
        assert_eq!(ids.len(), 5);
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[tokio::test]
    async fn judged_without_a_connected_account_finds_no_involvement() {
        let (_dir, db) = seeded().await;

        let rows = db
            .read(|conn| judged(conn, Which::All, None))
            .await
            .unwrap();

        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|j| !j.subject.assigned_to_me
            && !j.subject.reported_by_me
            && !j.subject.in_my_people_field));
    }

    #[tokio::test]
    async fn judged_can_start_after_a_change_id() {
        let (_dir, db) = seeded().await;
        let all = judged_rows(&db, Which::All).await;

        let later = judged_rows(&db, Which::After(all[2].id)).await;

        assert_eq!(
            later.iter().map(|j| j.id).collect::<Vec<_>>(),
            [all[3].id, all[4].id]
        );
    }

    fn mentions_and_comments_on_my_issues() -> Notifications {
        Notifications {
            scope: NotifyScope::Involved,
            in_app: vec![NotifyEvent::MentionsMe, NotifyEvent::NewComment],
            ..Notifications::default()
        }
    }

    #[tokio::test]
    async fn settling_marks_the_changes_that_do_not_count_in_app_as_quiet() {
        let (_dir, db) = seeded().await;

        let changed = settle_all(&db, mentions_and_comments_on_my_issues(), false).await;

        assert_eq!(
            quiet_marks(&db).await,
            [
                Some(true),
                Some(false),
                Some(true),
                Some(true),
                Some(false)
            ]
        );
        assert_eq!(changed, ids(&["1", "3", "4"]));
        assert!(
            judged_rows(&db, Which::Unsettled).await.is_empty(),
            "every change is decided"
        );
    }

    #[tokio::test]
    async fn settling_decides_only_new_changes_unless_asked_to_again() {
        let (_dir, db) = seeded().await;
        settle_all(&db, mentions_and_comments_on_my_issues(), false).await;
        db.write(|tx, _| {
            tx.execute_batch(
                "INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
                 VALUES ('4', 'priority', 'High', 'Low', '2026-10-28T10:00:00Z')",
            )?;
            Ok(())
        })
        .await
        .unwrap();

        let changed = settle_all(&db, Notifications::default(), false).await;

        assert_eq!(
            quiet_marks(&db).await,
            [
                Some(true),
                Some(false),
                Some(true),
                Some(true),
                Some(false),
                Some(false)
            ]
        );
        assert_eq!(changed, ids(&[]));

        let changed = settle_all(&db, Notifications::default(), true).await;

        assert_eq!(quiet_marks(&db).await, vec![Some(false); 6]);
        assert_eq!(changed, ids(&["1", "3", "4"]));
    }

    #[tokio::test]
    async fn settling_again_with_the_same_settings_changes_nothing() {
        let (_dir, db) = seeded().await;
        settle_all(&db, mentions_and_comments_on_my_issues(), false).await;
        let before = quiet_marks(&db).await;

        let changed = settle_all(&db, mentions_and_comments_on_my_issues(), true).await;

        assert_eq!(quiet_marks(&db).await, before);
        assert_eq!(changed, ids(&[]));
    }

    #[tokio::test]
    async fn settling_for_another_account_decides_the_changes_for_that_account() {
        let (_dir, db) = seeded().await;
        settle_all(&db, mentions_and_comments_on_my_issues(), false).await;

        let changed = db
            .write(|tx, changes| {
                settle(
                    tx,
                    changes,
                    &mentions_and_comments_on_my_issues(),
                    Some(MARTA),
                    true,
                )?;
                Ok(changes.issues.clone())
            })
            .await
            .unwrap();

        assert_eq!(
            quiet_marks(&db).await,
            [
                Some(true),
                Some(true),
                Some(true),
                Some(false),
                Some(true)
            ]
        );
        assert_eq!(changed, ids(&["2", "4"]));
    }
}
