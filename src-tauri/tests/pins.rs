//! Pins on a temp database with a small cached project: what pinning an issue, a comment and a filter
//! list stores and shows, how pins end (unpinned, run out, too old), and the Pinned counts and lists in
//! the sidebar and the Bookmarks view.

use chrono::{DateTime, Utc};
use tempfile::TempDir;

use taskboard_lib::connection::track_project;
use taskboard_lib::db::Db;
use taskboard_lib::error::ErrorKind;
use taskboard_lib::lists::{Kind, SidebarTree, Viewer, list_create, list_delete, sidebar_tree};
use taskboard_lib::pins::{
    PinDuration, PinItem, PinKind, Pinned, Pins, pin_create, pin_move, pin_remove, pin_rename,
    pin_toggle_issue, pins_list,
};
use taskboard_lib::space_config::{SpaceLayouts, space_set_layouts};
use taskboard_lib::status_style::Style;
use taskboard_lib::test_support::temp_config_dir;
use taskboard_lib::views::rows::person;
use taskboard_lib::views::{
    GroupBy, Layout, Narrowing, view_create, view_delete, view_rows, view_set_look, views_of,
};

const NOW: &str = "2026-10-28T10:00:00Z";
const ME: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const NW: &str = "10001";
const COMMENT_TEXT: &str = "Please check the staging deploy before Friday.";

fn viewer(now: &str) -> Viewer {
    Viewer {
        me: Some(ME.into()),
        time_zone: Some("Europe/Berlin".into()),
        now: now.parse::<DateTime<Utc>>().unwrap(),
    }
}

async fn execute(db: &Db, sql: impl Into<String>) {
    let sql = sql.into();
    db.write(move |tx, _| {
        tx.execute_batch(&sql)?;
        Ok(())
    })
    .await
    .unwrap();
}

async fn number(db: &Db, sql: impl Into<String>) -> i64 {
    let sql = sql.into();
    db.read(move |conn| Ok(conn.query_row(&sql, [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap()
}

/// A tracked Website relaunch project with three issues (to do bug, issue in review, done bug) and
/// three comments on the first two.
async fn lab() -> (TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    let issue = |id: &str, key: &str, summary: &str, kind: &str, status: &str, category: &str| {
        format!(
            "INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id,
               status_category, created_at, updated_at, synced_at)
             VALUES ('{id}', '{key}', '{NW}', '{summary}', '{kind}', '{status}', '{category}',
               '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z',
               '2026-10-20T09:00:00.000Z');"
        )
    };
    let comment = |id: &str, issue: &str, body: &str| {
        format!(
            "INSERT INTO cache.comment (id, issue_id, author_id, body_text, created_at, updated_at)
             VALUES ('{id}', '{issue}', '{ME}', '{body}', '2026-10-21T09:00:00.000Z',
               '2026-10-21T09:00:00.000Z');"
        )
    };
    execute(
        &db,
        format!(
            "INSERT INTO cache.space (id, key, name, raw)
               VALUES ('{NW}', 'NW', 'Website relaunch', '{{}}');
             INSERT INTO cache.status (id, name, category) VALUES
               ('s1', 'To Do', 'new'), ('s2', 'In Review', 'indeterminate'), ('s3', 'Done', 'done');
             INSERT INTO cache.user (account_id, name) VALUES ('{ME}', 'Alex Morgan');
             {}{}{}{}{}{}",
            issue("1001", "NW-1", "Fix the checkout total", "Bug", "s1", "new"),
            issue(
                "1002",
                "NW-2",
                "Review the shipping copy",
                "Task",
                "s2",
                "indeterminate"
            ),
            issue("1003", "NW-3", "Crash on empty basket", "Bug", "s3", "done"),
            comment("c1", "1001", COMMENT_TEXT),
            comment("c2", "1002", &"a".repeat(120)),
            comment("c3", "1002", "Looks good\n\nto   me"),
        ),
    )
    .await;
    track_project(&db, NW.into(), true).await.unwrap();
    (dir, db)
}

async fn list_id(db: &Db, name: &str) -> i64 {
    number(
        db,
        format!("SELECT id FROM filter_list WHERE name = '{name}'"),
    )
    .await
}

async fn node_id(db: &Db, kind: &str) -> i64 {
    number(
        db,
        format!("SELECT id FROM filter_list WHERE kind = '{kind}'"),
    )
    .await
}

async fn pin(
    db: &Db,
    now: &str,
    kind: PinKind,
    target: &str,
    duration: PinDuration,
    reason: Option<&str>,
) {
    pin_create(
        db,
        viewer(now),
        kind,
        target.into(),
        duration,
        reason.map(String::from),
    )
    .await
    .unwrap();
}

async fn pin_issue(db: &Db, now: &str, id: &str, duration: PinDuration) {
    pin(db, now, PinKind::Issue, id, duration, None).await;
}

async fn pin_list(db: &Db, now: &str, list: i64, duration: PinDuration) {
    pin(
        db,
        now,
        PinKind::FilterList,
        &list.to_string(),
        duration,
        None,
    )
    .await;
}

async fn pins_at(db: &Db, now: &str) -> Pins {
    pins_list(db, viewer(now)).await.unwrap()
}

async fn tree_at(db: &Db, now: &str) -> SidebarTree {
    sidebar_tree(db, viewer(now), false).await.unwrap()
}

fn targets(items: &[PinItem]) -> Vec<Pinned> {
    items.iter().map(|item| item.target.clone()).collect()
}

fn issue_target(id: &str, key: &str, summary: &str, style: Style, gone: bool) -> Pinned {
    Pinned::Issue {
        id: id.into(),
        key: key.into(),
        summary: summary.into(),
        style,
        gone,
    }
}

fn home_pinned_count(tree: &SidebarTree) -> Option<i64> {
    tree.home
        .iter()
        .find(|node| node.kind == Kind::Pinned)
        .unwrap()
        .count
}

fn pinned_list_names(tree: &SidebarTree) -> Vec<&str> {
    tree.pinned_lists
        .iter()
        .map(|node| node.name.as_str())
        .collect()
}

mod what_a_pin_shows {
    use super::*;

    #[tokio::test]
    async fn an_issue_a_comment_and_two_lists_are_listed_in_the_order_they_were_pinned() {
        let (_dir, db) = lab().await;
        let open_bugs = list_id(&db, "Open bugs").await;
        let everything = node_id(&db, "everything").await;
        let my_bugs = list_create(&db, viewer(NOW), everything, "My bugs".into(), None)
            .await
            .unwrap();

        pin_issue(
            &db,
            "2026-10-28T09:00:00Z",
            "1001",
            PinDuration::UntilUnpinned,
        )
        .await;
        pin(
            &db,
            "2026-10-28T09:30:00Z",
            PinKind::Comment,
            "c1",
            PinDuration::UntilUnpinned,
            None,
        )
        .await;
        pin_list(
            &db,
            "2026-10-28T09:45:00Z",
            open_bugs,
            PinDuration::UntilUnpinned,
        )
        .await;
        pin_list(
            &db,
            "2026-10-28T09:50:00Z",
            my_bugs,
            PinDuration::UntilUnpinned,
        )
        .await;

        let pins = pins_at(&db, NOW).await;

        assert_eq!(
            targets(&pins.active),
            [
                issue_target("1001", "NW-1", "Fix the checkout total", Style::Todo, false),
                Pinned::Comment {
                    id: "c1".into(),
                    issue_id: "1001".into(),
                    issue_key: "NW-1".into(),
                    author: Some(person(ME, "Alex Morgan", Some(ME))),
                    excerpt: COMMENT_TEXT.into(),
                },
                Pinned::List {
                    id: open_bugs,
                    name: "Open bugs".into(),
                    space: Some("Website relaunch".into()),
                },
                Pinned::List {
                    id: my_bugs,
                    name: "My bugs".into(),
                    space: None,
                },
            ]
        );
        assert!(pins.recently_unpinned.is_empty());
        for item in &pins.active {
            assert_eq!(item.time_left, "");
            assert_eq!(item.expires_at, None);
            assert_eq!(item.reason, None);
        }
    }

    #[tokio::test]
    async fn a_long_comment_is_cut_with_an_ellipsis_and_line_breaks_become_spaces() {
        let (_dir, db) = lab().await;
        for (comment, expected) in [
            ("c2", format!("{}…", "a".repeat(90))),
            ("c3", "Looks good to me".to_string()),
        ] {
            pin(&db, NOW, PinKind::Comment, comment, PinDuration::Week, None).await;

            let pins = pins_at(&db, NOW).await;

            let Pinned::Comment { excerpt, .. } = &pins.active.last().unwrap().target else {
                panic!("{comment} is not a comment pin");
            };
            assert_eq!(excerpt, &expected, "{comment}");
        }
    }

    #[tokio::test]
    async fn a_gone_issue_stays_pinned_and_is_marked_gone_with_its_status_style() {
        let (_dir, db) = lab().await;
        execute(&db, "UPDATE cache.issue SET gone = 1 WHERE id = '1002'").await;
        pin_issue(&db, NOW, "1002", PinDuration::UntilUnpinned).await;

        let pins = pins_at(&db, NOW).await;

        assert_eq!(
            targets(&pins.active),
            [issue_target(
                "1002",
                "NW-2",
                "Review the shipping copy",
                Style::Review,
                true
            )]
        );
    }

    #[tokio::test]
    async fn day_based_pins_show_how_long_is_left_and_when_they_end() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::Today).await;
        pin_issue(&db, NOW, "1002", PinDuration::TwoDays).await;
        pin_issue(&db, NOW, "1003", PinDuration::Week).await;

        let today = pins_at(&db, NOW).await;
        let tomorrow = pins_at(&db, "2026-10-29T10:00:00Z").await;

        let summary = |pins: &Pins| -> Vec<(String, Option<String>)> {
            pins.active
                .iter()
                .map(|item| (item.time_left.clone(), item.expires_at.clone()))
                .collect()
        };
        assert_eq!(
            summary(&today),
            [
                (
                    "today".to_string(),
                    Some("2026-10-28T23:00:00.000Z".to_string())
                ),
                (
                    "2d left".to_string(),
                    Some("2026-10-29T23:00:00.000Z".to_string())
                ),
                (
                    "7d left".to_string(),
                    Some("2026-11-03T23:00:00.000Z".to_string())
                ),
            ]
        );
        assert_eq!(
            tomorrow
                .active
                .iter()
                .map(|item| item.time_left.as_str())
                .collect::<Vec<_>>(),
            ["today", "6d left"]
        );
    }
}

mod creating_pins {
    use super::*;

    #[tokio::test]
    async fn pinning_the_same_target_again_replaces_its_reason_and_duration() {
        let (_dir, db) = lab().await;
        pin(
            &db,
            NOW,
            PinKind::Issue,
            "1001",
            PinDuration::Today,
            Some("first"),
        )
        .await;

        pin(
            &db,
            "2026-10-28T10:30:00Z",
            PinKind::Issue,
            "1001",
            PinDuration::Week,
            Some("second"),
        )
        .await;

        let pins = pins_at(&db, "2026-10-28T10:30:00Z").await;
        assert_eq!(pins.active.len(), 1);
        assert_eq!(pins.active[0].reason.as_deref(), Some("second"));
        assert_eq!(pins.active[0].time_left, "7d left");
        assert_eq!(
            pins.active[0].expires_at.as_deref(),
            Some("2026-11-03T23:00:00.000Z")
        );
        assert_eq!(number(&db, "SELECT COUNT(*) FROM pin").await, 1);
    }

    #[tokio::test]
    async fn a_reason_is_trimmed_flattened_capped_and_dropped_when_empty() {
        let (_dir, db) = lab().await;
        let cases = [
            (
                Some("  Fix before\nthe demo  ".to_string()),
                Some("Fix before the demo".to_string()),
            ),
            (Some("   ".to_string()), None),
            (None, None),
            (Some("x".repeat(300)), Some("x".repeat(200))),
        ];
        for (input, expected) in cases {
            pin_create(
                &db,
                viewer(NOW),
                PinKind::Issue,
                "1001".into(),
                PinDuration::UntilUnpinned,
                input.clone(),
            )
            .await
            .unwrap();

            let pins = pins_at(&db, NOW).await;

            assert_eq!(pins.active[0].reason, expected, "{input:?}");
        }
    }

    #[tokio::test]
    async fn pinning_a_missing_target_is_not_found_and_stores_nothing() {
        let (_dir, db) = lab().await;
        for (kind, target) in [
            (PinKind::Issue, "9999"),
            (PinKind::Comment, "c99"),
            (PinKind::FilterList, "9999"),
            (PinKind::View, "9999"),
            (PinKind::View, "not a view id"),
        ] {
            let err = pin_create(
                &db,
                viewer(NOW),
                kind,
                target.into(),
                PinDuration::Today,
                None,
            )
            .await
            .unwrap_err();

            assert_eq!(err.kind(), ErrorKind::NotFound, "{kind:?} {target}");
        }

        assert_eq!(number(&db, "SELECT COUNT(*) FROM pin").await, 0);
    }

    #[tokio::test]
    async fn toggling_an_issue_pins_it_until_unpinned_and_toggling_again_unpins_it() {
        let (_dir, db) = lab().await;

        pin_toggle_issue(&db, viewer(NOW), "1001".into(), PinDuration::UntilUnpinned)
            .await
            .unwrap();
        let pinned = pins_at(&db, NOW).await;
        pin_toggle_issue(
            &db,
            viewer("2026-10-28T10:05:00Z"),
            "1001".into(),
            PinDuration::UntilUnpinned,
        )
        .await
        .unwrap();
        let unpinned = pins_at(&db, "2026-10-28T10:10:00Z").await;

        assert_eq!(pinned.active.len(), 1);
        assert_eq!(pinned.active[0].time_left, "");
        assert_eq!(pinned.active[0].expires_at, None);
        assert!(unpinned.active.is_empty());
        assert_eq!(
            targets(&unpinned.recently_unpinned),
            [issue_target(
                "1001",
                "NW-1",
                "Fix the checkout total",
                Style::Todo,
                false
            )]
        );
    }
}

mod toggling_with_a_duration {
    use super::*;

    #[tokio::test]
    async fn toggling_an_issue_with_a_week_pins_it_for_seven_days() {
        let (_dir, db) = lab().await;

        pin_toggle_issue(&db, viewer(NOW), "1001".into(), PinDuration::Week)
            .await
            .unwrap();

        let pins = pins_at(&db, NOW).await;
        assert_eq!(pins.active.len(), 1);
        assert_eq!(pins.active[0].time_left, "7d left");
        assert_eq!(
            pins.active[0].expires_at.as_deref(),
            Some("2026-11-03T23:00:00.000Z")
        );
    }
}

mod ending_pins {
    use super::*;

    #[tokio::test]
    async fn an_unpinned_pin_moves_to_recently_unpinned_without_a_time_left() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::Week).await;
        let id = pins_at(&db, NOW).await.active[0].id;

        pin_remove(&db, viewer("2026-10-28T11:00:00Z"), id)
            .await
            .unwrap();

        let pins = pins_at(&db, "2026-10-28T11:30:00Z").await;
        assert!(pins.active.is_empty());
        assert_eq!(pins.recently_unpinned.len(), 1);
        assert_eq!(pins.recently_unpinned[0].id, id);
        assert_eq!(pins.recently_unpinned[0].time_left, "");
    }

    #[tokio::test]
    async fn a_pin_that_runs_out_leaves_active_at_its_end_and_moves_to_recently_unpinned() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::Today).await;

        let before = pins_at(&db, "2026-10-28T22:59:59Z").await;
        let after = pins_at(&db, "2026-10-29T00:00:00Z").await;

        assert_eq!(before.active.len(), 1);
        assert!(before.recently_unpinned.is_empty());
        assert!(after.active.is_empty());
        assert_eq!(after.recently_unpinned.len(), 1);
    }

    #[tokio::test]
    async fn ended_pins_stay_in_recently_unpinned_for_seven_days_only() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;
        pin_issue(&db, NOW, "1002", PinDuration::Today).await;
        let removed = pins_at(&db, NOW).await.active[0].id;
        pin_remove(&db, viewer(NOW), removed).await.unwrap();

        for (viewed, remembered) in [
            ("2026-11-03T10:00:00Z", 2),
            ("2026-11-04T11:00:00Z", 1),
            ("2026-11-05T10:00:00Z", 0),
        ] {
            let pins = pins_at(&db, viewed).await;

            assert_eq!(pins.recently_unpinned.len(), remembered, "{viewed}");
        }
    }

    #[tokio::test]
    async fn pinning_again_after_a_pin_ended_starts_a_new_active_pin() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::Today).await;

        pin_issue(&db, "2026-10-29T08:00:00Z", "1001", PinDuration::Week).await;

        let pins = pins_at(&db, "2026-10-29T08:00:00Z").await;
        assert_eq!(pins.active.len(), 1);
        assert_eq!(pins.recently_unpinned.len(), 1);
        assert_eq!(number(&db, "SELECT COUNT(*) FROM pin").await, 2);
    }

    #[tokio::test]
    async fn deleting_a_filter_list_removes_its_pins_everywhere() {
        let (_dir, db) = lab().await;
        let epics = list_id(&db, "Epics").await;
        let open_bugs = list_id(&db, "Open bugs").await;
        pin_list(
            &db,
            "2026-10-28T09:00:00Z",
            epics,
            PinDuration::UntilUnpinned,
        )
        .await;
        pin_list(
            &db,
            "2026-10-28T09:30:00Z",
            open_bugs,
            PinDuration::UntilUnpinned,
        )
        .await;

        list_delete(&db, epics).await.unwrap();

        let pins = pins_at(&db, NOW).await;
        assert_eq!(
            targets(&pins.active),
            [Pinned::List {
                id: open_bugs,
                name: "Open bugs".into(),
                space: Some("Website relaunch".into()),
            }]
        );
        assert!(pins.recently_unpinned.is_empty());
        let tree = tree_at(&db, NOW).await;
        assert_eq!(pinned_list_names(&tree), ["Open bugs"]);
        assert_eq!(home_pinned_count(&tree), Some(1));
    }
}

mod bookmarked_views {
    use super::*;

    async fn view_id(db: &Db, list: i64, name: &str) -> i64 {
        number(
            db,
            format!("SELECT id FROM view WHERE list_id = {list} AND name = '{name}'"),
        )
        .await
    }

    async fn pin_view(db: &Db, now: &str, view: i64) {
        pin(
            db,
            now,
            PinKind::View,
            &view.to_string(),
            PinDuration::UntilUnpinned,
            None,
        )
        .await;
    }

    fn view_target(
        id: i64,
        list_id: i64,
        name: &str,
        layout: Layout,
        list: &str,
        space: Option<&str>,
    ) -> Pinned {
        Pinned::View {
            id,
            list_id,
            name: name.into(),
            layout,
            list: list.into(),
            space: space.map(String::from),
            icon: None,
            color: None,
        }
    }

    fn look_of(pinned: &Pinned) -> (Option<&str>, Option<&str>) {
        match pinned {
            Pinned::View { icon, color, .. } => (icon.as_deref(), color.as_deref()),
            other => panic!("not a view: {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_bookmarked_view_carries_the_look_of_its_tab_and_follows_a_change() {
        let (_dir, db) = lab().await;
        let space = node_id(&db, "space").await;
        let board = view_id(&db, space, "Board").await;
        pin_view(&db, NOW, board).await;
        assert_eq!(
            look_of(&pins_at(&db, NOW).await.active[0].target),
            (None, None)
        );

        view_set_look(&db, board, Some("rocket".into()), Some("3".into()))
            .await
            .unwrap();
        let first = pins_at(&db, NOW).await;
        view_set_look(&db, board, Some("R".into()), Some("5".into()))
            .await
            .unwrap();
        let second = pins_at(&db, NOW).await;

        assert_eq!(
            look_of(&first.active[0].target),
            (Some("rocket"), Some("3"))
        );
        assert_eq!(look_of(&second.active[0].target), (Some("R"), Some("5")));
    }

    #[tokio::test]
    async fn views_of_a_space_a_filter_list_and_a_global_list_show_their_level_in_pin_order() {
        let (_dir, db) = lab().await;
        let space = node_id(&db, "space").await;
        let open_bugs = list_id(&db, "Open bugs").await;
        let everything = node_id(&db, "everything").await;
        let my_bugs = list_create(&db, viewer(NOW), everything, "My bugs".into(), None)
            .await
            .unwrap();
        let global_board = view_create(&db, my_bugs, "Board".into(), Layout::Board)
            .await
            .unwrap()
            .id;
        let space_board = view_id(&db, space, "Board").await;
        let bugs_list = view_id(&db, open_bugs, "List").await;
        pin_view(&db, "2026-10-28T09:00:00Z", space_board).await;
        pin_view(&db, "2026-10-28T09:30:00Z", bugs_list).await;
        pin_view(&db, "2026-10-28T09:45:00Z", global_board).await;

        let pins = pins_at(&db, NOW).await;

        assert_eq!(
            targets(&pins.active),
            [
                view_target(
                    space_board,
                    space,
                    "Board",
                    Layout::Board,
                    "Website relaunch",
                    None
                ),
                view_target(
                    bugs_list,
                    open_bugs,
                    "List",
                    Layout::List,
                    "Open bugs",
                    Some("Website relaunch")
                ),
                view_target(
                    global_board,
                    my_bugs,
                    "Board",
                    Layout::Board,
                    "My bugs",
                    None
                ),
            ]
        );
        assert!(pins.recently_unpinned.is_empty());
        assert_eq!(
            number(&db, "SELECT COUNT(*) FROM pin WHERE target_kind = 'view'").await,
            3
        );
    }

    #[tokio::test]
    async fn a_layout_the_space_switched_off_shows_as_the_spaces_default() {
        let (_dir, db) = lab().await;
        let space = node_id(&db, "space").await;
        let space_board = view_id(&db, space, "Board").await;
        let space_table = view_id(&db, space, "Table").await;
        pin_view(&db, "2026-10-28T09:00:00Z", space_board).await;
        pin_view(&db, "2026-10-28T09:30:00Z", space_table).await;
        space_set_layouts(
            &db,
            NW.into(),
            SpaceLayouts {
                layouts: vec![Layout::List, Layout::Table],
                default_layout: Layout::List,
                default_group: GroupBy::Status,
            },
        )
        .await
        .unwrap();

        let pins = pins_at(&db, NOW).await;

        assert_eq!(
            targets(&pins.active),
            [
                view_target(
                    space_board,
                    space,
                    "Board",
                    Layout::List,
                    "Website relaunch",
                    None
                ),
                view_target(
                    space_table,
                    space,
                    "Table",
                    Layout::Table,
                    "Website relaunch",
                    None
                ),
            ]
        );
    }

    #[tokio::test]
    async fn removing_a_view_pin_moves_it_to_recently_unpinned() {
        let (_dir, db) = lab().await;
        let space = node_id(&db, "space").await;
        let space_board = view_id(&db, space, "Board").await;
        pin_view(&db, NOW, space_board).await;
        let id = pins_at(&db, NOW).await.active[0].id;

        pin_remove(&db, viewer("2026-10-28T11:00:00Z"), id)
            .await
            .unwrap();

        let pins = pins_at(&db, "2026-10-28T11:30:00Z").await;
        assert!(pins.active.is_empty());
        assert_eq!(
            targets(&pins.recently_unpinned),
            [view_target(
                space_board,
                space,
                "Board",
                Layout::Board,
                "Website relaunch",
                None
            )]
        );
        assert_eq!(pins.recently_unpinned[0].id, id);
    }

    #[tokio::test]
    async fn a_deleted_view_is_left_out_of_active_and_recently_unpinned() {
        let (_dir, db) = lab().await;
        let space = node_id(&db, "space").await;
        let board = view_id(&db, space, "Board").await;
        let table = view_id(&db, space, "Table").await;
        let all_issues = view_id(&db, space, "All issues").await;
        pin_view(&db, "2026-10-28T09:00:00Z", board).await;
        pin_view(&db, "2026-10-28T09:10:00Z", table).await;
        pin_view(&db, "2026-10-28T09:20:00Z", all_issues).await;
        let table_pin = pins_at(&db, NOW).await.active[1].id;
        pin_remove(&db, viewer(NOW), table_pin).await.unwrap();

        view_delete(&db, board).await.unwrap();
        view_delete(&db, table).await.unwrap();

        let pins = pins_at(&db, NOW).await;
        assert_eq!(
            targets(&pins.active),
            [view_target(
                all_issues,
                space,
                "All issues",
                Layout::List,
                "Website relaunch",
                None
            )]
        );
        assert!(pins.recently_unpinned.is_empty());
    }

    #[tokio::test]
    async fn the_view_pin_kind_and_target_keep_their_names_towards_the_page() {
        let (_dir, db) = lab().await;
        let open_bugs = list_id(&db, "Open bugs").await;
        let bugs_list = view_id(&db, open_bugs, "List").await;
        pin_view(&db, NOW, bugs_list).await;

        let pins = pins_at(&db, NOW).await;

        assert_eq!(serde_json::to_value(PinKind::View).unwrap(), "view");
        assert_eq!(
            serde_json::to_value(&pins.active[0].target).unwrap(),
            serde_json::json!({
                "kind": "view",
                "id": bugs_list,
                "listId": open_bugs,
                "name": "List",
                "layout": "list",
                "list": "Open bugs",
                "space": "Website relaunch",
                "icon": null,
                "color": null,
            })
        );
    }
}

mod in_the_sidebar {
    use super::*;

    #[tokio::test]
    async fn pinned_filter_lists_appear_in_pin_order_with_their_counts() {
        let (_dir, db) = lab().await;
        let sprint = list_id(&db, "Current sprint").await;
        let open_bugs = list_id(&db, "Open bugs").await;
        pin_list(
            &db,
            "2026-10-28T09:00:00Z",
            sprint,
            PinDuration::UntilUnpinned,
        )
        .await;
        pin_list(
            &db,
            "2026-10-28T09:30:00Z",
            open_bugs,
            PinDuration::UntilUnpinned,
        )
        .await;

        let tree = tree_at(&db, NOW).await;

        let pinned: Vec<(i64, Kind, &str, Option<i64>)> = tree
            .pinned_lists
            .iter()
            .map(|node| (node.id, node.kind, node.name.as_str(), node.count))
            .collect();
        assert_eq!(
            pinned,
            [
                (sprint, Kind::List, "Current sprint", Some(0)),
                (open_bugs, Kind::List, "Open bugs", Some(1)),
            ]
        );
        let space = tree.spaces.iter().find(|s| s.key == "NW").unwrap();
        assert!(space.lists.iter().any(|l| l.id == open_bugs));
    }

    #[tokio::test]
    async fn the_pinned_home_node_counts_the_active_pins_of_every_kind() {
        let (_dir, db) = lab().await;
        let open_bugs = list_id(&db, "Open bugs").await;
        assert_eq!(home_pinned_count(&tree_at(&db, NOW).await), Some(0));

        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;
        pin(
            &db,
            NOW,
            PinKind::Comment,
            "c1",
            PinDuration::UntilUnpinned,
            None,
        )
        .await;
        pin_list(&db, NOW, open_bugs, PinDuration::UntilUnpinned).await;

        assert_eq!(home_pinned_count(&tree_at(&db, NOW).await), Some(3));
    }

    #[tokio::test]
    async fn unpinned_and_expired_pins_leave_the_count_and_the_pinned_lists() {
        let (_dir, db) = lab().await;
        let open_bugs = list_id(&db, "Open bugs").await;
        let sprint = list_id(&db, "Current sprint").await;
        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;
        pin_issue(&db, NOW, "1002", PinDuration::Today).await;
        pin_list(&db, NOW, open_bugs, PinDuration::Today).await;
        pin_list(&db, NOW, sprint, PinDuration::UntilUnpinned).await;
        let tomorrow = "2026-10-29T08:00:00Z";

        let expired = tree_at(&db, tomorrow).await;
        let issue_pin = pins_at(&db, tomorrow).await.active[0].id;
        pin_remove(&db, viewer(tomorrow), issue_pin).await.unwrap();
        let removed = tree_at(&db, tomorrow).await;

        assert_eq!(home_pinned_count(&expired), Some(2));
        assert_eq!(pinned_list_names(&expired), ["Current sprint"]);
        assert_eq!(home_pinned_count(&removed), Some(1));
        assert_eq!(pinned_list_names(&removed), ["Current sprint"]);
    }
}

mod in_the_pinned_view {
    use super::*;

    async fn shown_ids(db: &Db, now: &str) -> Vec<String> {
        let list = node_id(db, "pinned").await;
        let view = views_of(db, list).await.unwrap()[0].id;
        let data = view_rows(db, viewer(now), list, view, Narrowing::default())
            .await
            .unwrap();
        let mut ids: Vec<String> = data.rows.keys().cloned().collect();
        ids.sort();
        ids
    }

    #[tokio::test]
    async fn only_actively_pinned_issues_are_shown() {
        let (_dir, db) = lab().await;
        let open_bugs = list_id(&db, "Open bugs").await;
        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;
        pin_issue(&db, NOW, "1003", PinDuration::Week).await;
        pin(&db, NOW, PinKind::Comment, "c2", PinDuration::Week, None).await;
        pin_list(&db, NOW, open_bugs, PinDuration::Week).await;

        assert_eq!(shown_ids(&db, NOW).await, ["1001", "1003"]);
    }

    #[tokio::test]
    async fn a_bookmarked_subtask_is_shown_and_counted_like_any_other_issue() {
        let (_dir, db) = lab().await;
        execute(
            &db,
            format!(
                "INSERT INTO cache.issue (id, key, project_id, summary, issue_type, type_is_subtask,
                   parent_id, status_id, status_category, created_at, updated_at, synced_at)
                 VALUES ('1004', 'NW-4', '{NW}', 'Add a regression test', 'Subtask', 1, '1001',
                   's1', 'new', '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z',
                   '2026-10-20T09:00:00.000Z')"
            ),
        )
        .await;

        pin_issue(&db, NOW, "1004", PinDuration::UntilUnpinned).await;

        assert_eq!(shown_ids(&db, NOW).await, ["1004"]);
        assert_eq!(home_pinned_count(&tree_at(&db, NOW).await), Some(1));
    }

    #[tokio::test]
    async fn an_expired_or_unpinned_issue_is_gone_from_the_view() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;
        pin_issue(&db, NOW, "1002", PinDuration::Today).await;
        pin_issue(&db, NOW, "1003", PinDuration::UntilUnpinned).await;
        let next_day = "2026-10-29T08:00:00Z";

        assert_eq!(
            shown_ids(&db, "2026-10-28T22:00:00Z").await,
            ["1001", "1002", "1003"]
        );
        assert_eq!(shown_ids(&db, next_day).await, ["1001", "1003"]);

        let pins = pins_at(&db, next_day).await;
        pin_remove(&db, viewer(next_day), pins.active[1].id)
            .await
            .unwrap();
        assert_eq!(shown_ids(&db, next_day).await, ["1001"]);
    }
}

mod naming_and_ordering {
    use super::*;

    /// Pins the three issues in order and returns their pin ids.
    async fn three_pins(db: &Db) -> Vec<i64> {
        for id in ["1001", "1002", "1003"] {
            pin_issue(db, NOW, id, PinDuration::UntilUnpinned).await;
        }
        active_ids(db).await
    }

    async fn active_ids(db: &Db) -> Vec<i64> {
        pins_at(db, NOW).await.active.iter().map(|p| p.id).collect()
    }

    async fn position_of(db: &Db, pin: i64) -> i64 {
        number(db, format!("SELECT position FROM pin WHERE id = {pin}")).await
    }

    async fn titles(db: &Db) -> Vec<Option<String>> {
        pins_at(db, NOW)
            .await
            .active
            .into_iter()
            .map(|p| p.title)
            .collect()
    }

    #[tokio::test]
    async fn a_new_pin_goes_last_even_after_an_earlier_one_ended() {
        let (_dir, db) = lab().await;
        let ids = three_pins(&db).await;
        pin_remove(&db, viewer(NOW), ids[0]).await.unwrap();
        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;

        let order = active_ids(&db).await;

        assert_eq!(order[..2], ids[1..]);
        assert_eq!(order.len(), 3);
        assert_ne!(order[2], ids[0]);
    }

    #[tokio::test]
    async fn pinning_again_keeps_the_title_and_the_place() {
        let (_dir, db) = lab().await;
        let ids = three_pins(&db).await;
        pin_rename(&db, ids[0], Some("Checkout".into()))
            .await
            .unwrap();

        pin_issue(&db, NOW, "1001", PinDuration::Week).await;

        assert_eq!(active_ids(&db).await, ids);
        assert_eq!(
            titles(&db).await,
            [Some("Checkout".to_string()), None, None]
        );
        assert_eq!(number(&db, "SELECT COUNT(*) FROM pin").await, 3);
    }

    #[tokio::test]
    async fn a_new_title_is_flattened_capped_and_cleared_when_blank() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;
        let id = active_ids(&db).await[0];
        let cases = [
            (
                Some("  Release \n  checklist ".to_string()),
                Some("Release checklist".to_string()),
            ),
            (Some("   ".to_string()), None),
            (None, None),
            (Some("x".repeat(200)), Some("x".repeat(120))),
        ];
        for (input, expected) in cases {
            pin_rename(&db, id, Some("Old".into())).await.unwrap();

            pin_rename(&db, id, input.clone()).await.unwrap();

            assert_eq!(titles(&db).await, [expected], "{input:?}");
        }
    }

    #[tokio::test]
    async fn renaming_an_unknown_bookmark_is_not_found() {
        let (_dir, db) = lab().await;

        let err = pin_rename(&db, 999, Some("Checkout".into()))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn the_title_reaches_the_page_as_title() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;
        let id = active_ids(&db).await[0];
        pin_rename(&db, id, Some("Checkout".into())).await.unwrap();

        let item = pins_at(&db, NOW).await.active.remove(0);

        assert_eq!(
            serde_json::to_value(&item).unwrap(),
            serde_json::json!({
                "id": id,
                "target": {
                    "kind": "issue",
                    "id": "1001",
                    "key": "NW-1",
                    "summary": "Fix the checkout total",
                    "style": serde_json::to_value(Style::Todo).unwrap(),
                    "gone": false,
                },
                "title": "Checkout",
                "reason": null,
                "timeLeft": "",
                "expiresAt": null,
            })
        );
    }

    #[tokio::test]
    async fn moving_a_bookmark_puts_it_before_another_or_last() {
        // (moved, before, expected order as indices of the three pins)
        let cases: [(usize, Option<usize>, [usize; 3]); 4] = [
            (2, Some(0), [2, 0, 1]),
            (0, None, [1, 2, 0]),
            (0, Some(2), [1, 0, 2]),
            (0, Some(1), [0, 1, 2]),
        ];
        for (moved, before, expected) in cases {
            let (_dir, db) = lab().await;
            let ids = three_pins(&db).await;

            pin_move(&db, viewer(NOW), ids[moved], before.map(|b| ids[b]))
                .await
                .unwrap();

            let want: Vec<i64> = expected.iter().map(|&i| ids[i]).collect();
            assert_eq!(active_ids(&db).await, want, "{moved} before {before:?}");
        }
    }

    #[tokio::test]
    async fn moving_numbers_the_active_bookmarks_from_zero_and_leaves_ended_ones_out() {
        let (_dir, db) = lab().await;
        let ids = three_pins(&db).await;
        pin_remove(&db, viewer(NOW), ids[1]).await.unwrap();

        pin_move(&db, viewer(NOW), ids[2], Some(ids[0]))
            .await
            .unwrap();

        assert_eq!(position_of(&db, ids[2]).await, 0);
        assert_eq!(position_of(&db, ids[0]).await, 1);
        assert_eq!(active_ids(&db).await, [ids[2], ids[0]]);
    }

    #[tokio::test]
    async fn moving_to_or_from_a_bookmark_that_isnt_active_is_not_found() {
        let (_dir, db) = lab().await;
        let ids = three_pins(&db).await;
        pin_remove(&db, viewer(NOW), ids[1]).await.unwrap();
        let cases = [
            (999, Some(ids[0])),
            (ids[1], Some(ids[0])),
            (ids[0], Some(ids[1])),
            (ids[0], Some(999)),
            (ids[0], Some(ids[0])),
        ];
        for (moved, before) in cases {
            let err = pin_move(&db, viewer(NOW), moved, before).await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::NotFound, "{moved} before {before:?}");
        }

        assert_eq!(active_ids(&db).await, [ids[0], ids[2]]);
    }

    #[tokio::test]
    async fn the_sidebar_lists_bookmarked_lists_in_the_bookmarks_order_under_their_titles() {
        let (_dir, db) = lab().await;
        let sprint = list_id(&db, "Current sprint").await;
        let open_bugs = list_id(&db, "Open bugs").await;
        pin_list(&db, NOW, sprint, PinDuration::UntilUnpinned).await;
        pin_list(&db, NOW, open_bugs, PinDuration::UntilUnpinned).await;
        let ids = active_ids(&db).await;
        pin_rename(&db, ids[1], Some("Triage".into()))
            .await
            .unwrap();
        assert_eq!(
            pinned_list_names(&tree_at(&db, NOW).await),
            ["Current sprint", "Triage"]
        );

        pin_move(&db, viewer(NOW), ids[1], Some(ids[0]))
            .await
            .unwrap();

        let tree = tree_at(&db, NOW).await;
        assert_eq!(pinned_list_names(&tree), ["Triage", "Current sprint"]);
        assert_eq!(tree.pinned_lists[0].id, open_bugs);
        assert_eq!(tree.pinned_lists[1].id, sprint);
    }
}

mod issue_activity {
    use rusqlite::OptionalExtension;

    use super::*;

    const LATER: &str = "2026-10-28T11:00:00Z";

    /// `(viewed_at, interacted_at)` of an issue; none when it has no activity row.
    async fn activity(db: &Db, issue_id: &str) -> Option<(Option<String>, Option<String>)> {
        let issue_id = issue_id.to_string();
        db.read(move |conn| {
            Ok(conn
                .query_row(
                    "SELECT viewed_at, interacted_at FROM issue_activity WHERE issue_id = ?1",
                    [issue_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?)
        })
        .await
        .unwrap()
    }

    fn interacted_at(text: &str) -> Option<(Option<String>, Option<String>)> {
        Some((None, Some(text.to_string())))
    }

    #[tokio::test]
    async fn pinning_an_issue_counts_as_an_interaction_but_not_a_view() {
        let (_dir, db) = lab().await;

        pin_issue(&db, NOW, "1001", PinDuration::Today).await;

        assert_eq!(
            activity(&db, "1001").await,
            interacted_at("2026-10-28T10:00:00.000Z")
        );
        assert_eq!(activity(&db, "1002").await, None);
    }

    #[tokio::test]
    async fn pinning_an_issue_again_moves_its_interaction() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::Today).await;

        pin_issue(&db, LATER, "1001", PinDuration::Week).await;

        assert_eq!(
            activity(&db, "1001").await,
            interacted_at("2026-10-28T11:00:00.000Z")
        );
    }

    #[tokio::test]
    async fn removing_the_pin_of_an_issue_counts_as_an_interaction() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;
        let id = pins_at(&db, NOW).await.active[0].id;

        pin_remove(&db, viewer(LATER), id).await.unwrap();

        assert_eq!(
            activity(&db, "1001").await,
            interacted_at("2026-10-28T11:00:00.000Z")
        );
    }

    #[tokio::test]
    async fn ending_a_pin_that_already_ended_is_not_an_interaction() {
        let (_dir, db) = lab().await;
        pin_issue(&db, NOW, "1001", PinDuration::UntilUnpinned).await;
        let id = pins_at(&db, NOW).await.active[0].id;
        pin_remove(&db, viewer(LATER), id).await.unwrap();

        pin_remove(&db, viewer("2026-10-28T12:00:00Z"), id)
            .await
            .unwrap();

        assert_eq!(
            activity(&db, "1001").await,
            interacted_at("2026-10-28T11:00:00.000Z")
        );
    }

    #[tokio::test]
    async fn toggling_an_issue_on_and_off_stamps_the_interaction_each_time() {
        let (_dir, db) = lab().await;

        pin_toggle_issue(&db, viewer(NOW), "1001".into(), PinDuration::UntilUnpinned)
            .await
            .unwrap();
        assert_eq!(
            activity(&db, "1001").await,
            interacted_at("2026-10-28T10:00:00.000Z")
        );

        pin_toggle_issue(
            &db,
            viewer(LATER),
            "1001".into(),
            PinDuration::UntilUnpinned,
        )
        .await
        .unwrap();
        assert_eq!(
            activity(&db, "1001").await,
            interacted_at("2026-10-28T11:00:00.000Z")
        );
    }

    #[tokio::test]
    async fn pinning_a_comment_or_a_list_leaves_the_issues_untouched() {
        let (_dir, db) = lab().await;
        let everything = node_id(&db, "everything").await;

        pin(&db, NOW, PinKind::Comment, "c1", PinDuration::Today, None).await;
        pin_list(&db, NOW, everything, PinDuration::Today).await;
        let id = pins_at(&db, NOW).await.active[0].id;
        pin_remove(&db, viewer(LATER), id).await.unwrap();

        assert_eq!(number(&db, "SELECT COUNT(*) FROM issue_activity").await, 0);
    }

    #[tokio::test]
    async fn pinning_a_missing_issue_stamps_nothing() {
        let (_dir, db) = lab().await;

        pin_create(
            &db,
            viewer(NOW),
            PinKind::Issue,
            "9999".into(),
            PinDuration::Today,
            None,
        )
        .await
        .unwrap_err();

        assert_eq!(number(&db, "SELECT COUNT(*) FROM issue_activity").await, 0);
    }
}
