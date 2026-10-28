//! The sidebar tree and the view tabs on temp folders: the defaults a tracked project gets, the open-issue
//! counts on the demo site synced from the in-process fake Jira, ancestor filtering and the edits that keep
//! positions dense.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Row;
use serde_json::json;
use tempfile::TempDir;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, fetch_versions, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::error::{AppError, ErrorKind};
use taskboard_lib::filter::{Condition, Field, Filter};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::{
    Kind, LIST_VIEW, ListConditions, ListNode, LockedCondition, SidebarTree, SpaceNode,
    SpaceUpdate, ValueKind, Viewer, ensure_tracked_spaces, filter_options, list_conditions,
    list_create, list_delete, list_duplicate, list_match_count, list_move, list_rename,
    list_set_filter, sidebar_tree, space_move, space_update,
};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::{Emit, SyncContext, full_sync};
use taskboard_lib::test_support::demo::{DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::{
    DEFAULT_COLUMNS, GroupBy, Layout, Sort, SortBy, View, space_default_columns, view_create,
    view_delete, view_duplicate, view_move, view_rename, view_set_columns, view_set_group,
    view_set_layout, view_set_sort, view_set_widths, view_set_wrap, views_of,
};

const ME_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const REVIEWER: &str = "customfield_10071";
const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const NW: &str = "10001";
const NWS: &str = "10002";

fn viewer(me: Option<&str>) -> Viewer {
    Viewer {
        me: me.map(String::from),
        time_zone: Some("Europe/Vienna".into()),
        now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
    }
}

fn filter(conditions: Vec<Condition>) -> Filter {
    Filter { conditions }
}

fn bugs() -> Filter {
    filter(vec![Condition::is(Field::Type, "Bug")])
}

async fn rows<T: Send + 'static>(
    db: &Db,
    sql: impl Into<String>,
    map: fn(&Row<'_>) -> rusqlite::Result<T>,
) -> Vec<T> {
    let sql = sql.into();
    db.read(move |conn| {
        let mut query = conn.prepare(&sql)?;
        let found = query.query_map([], map)?.collect::<Result<Vec<_>, _>>()?;
        Ok(found)
    })
    .await
    .unwrap()
}

async fn number(db: &Db, sql: impl Into<String>) -> i64 {
    rows(db, sql, |row| row.get::<_, i64>(0)).await[0]
}

async fn text(db: &Db, sql: impl Into<String>) -> String {
    rows(db, sql, |row| row.get::<_, String>(0)).await.remove(0)
}

async fn execute(db: &Db, sql: &'static str) {
    db.write(move |tx, _| {
        tx.execute_batch(sql)?;
        Ok(())
    })
    .await
    .unwrap();
}

fn assert_dense(positions: &[i64]) {
    let expected: Vec<i64> = (0..positions.len() as i64).collect();
    assert_eq!(positions, expected);
}

/// `(name, position)` of the filter lists under a parent, in position order.
async fn children(db: &Db, parent: i64) -> Vec<(String, i64)> {
    rows(
        db,
        format!(
            "SELECT name, position FROM filter_list WHERE parent_id = {parent} ORDER BY position"
        ),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .await
}

fn names(pairs: &[(String, i64)]) -> Vec<&str> {
    pairs.iter().map(|(name, _)| name.as_str()).collect()
}

fn positions(pairs: &[(String, i64)]) -> Vec<i64> {
    pairs.iter().map(|(_, position)| *position).collect()
}

async fn space_node_id(db: &Db, project_id: &str) -> i64 {
    number(
        db,
        format!("SELECT id FROM filter_list WHERE kind = 'space' AND project_id = '{project_id}'"),
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

async fn list_id(db: &Db, name: &str) -> i64 {
    number(
        db,
        format!("SELECT id FROM filter_list WHERE name = '{name}'"),
    )
    .await
}

fn find_space<'a>(tree: &'a SidebarTree, key: &str) -> &'a SpaceNode {
    tree.spaces
        .iter()
        .find(|space| space.key == key)
        .unwrap_or_else(|| panic!("no Space {key}"))
}

fn find_list<'a>(space: &'a SpaceNode, name: &str) -> &'a ListNode {
    space
        .lists
        .iter()
        .find(|list| list.name == name)
        .unwrap_or_else(|| panic!("no list {name} in {}", space.key))
}

fn view_names(views: &[View]) -> Vec<&str> {
    views.iter().map(|view| view.name.as_str()).collect()
}

fn view_positions(views: &[View]) -> Vec<i64> {
    views.iter().map(|view| view.position).collect()
}

fn defaults_of(columns: &[&str]) -> Vec<String> {
    columns.iter().map(|c| c.to_string()).collect()
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()
}

/// A database with two cached projects and no tracked Space.
async fn lab() -> (TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    execute(
        &db,
        "INSERT INTO cache.project (id, key, name, raw) VALUES
           ('10001', 'NW', 'Website relaunch', '{}'),
           ('10002', 'NWS', 'Northwind support', '{}')",
    )
    .await;
    (dir, db)
}

/// A database whose Website relaunch project is tracked, without versions.
async fn tracked_lab() -> (TempDir, Db) {
    let (dir, db) = lab().await;
    track_project(&db, NW.into(), true).await.unwrap();
    (dir, db)
}

mod defaults_on_tracking {
    use super::*;

    #[tokio::test]
    async fn a_tracked_project_gets_a_space_with_three_views_grouped_by_status() {
        let (_dir, db) = tracked_lab().await;

        let space = space_node_id(&db, NW).await;

        let views = views_of(&db, space).await.unwrap();
        assert_eq!(view_names(&views), ["All issues", "Board", "Table"]);
        assert_eq!(
            views.iter().map(|v| v.layout).collect::<Vec<_>>(),
            [Layout::List, Layout::Board, Layout::Table]
        );
        assert!(views.iter().all(|v| v.group_by == GroupBy::Status));
        assert!(
            views
                .iter()
                .all(|v| v.columns == defaults_of(&DEFAULT_COLUMNS))
        );
        assert_eq!(
            views.iter().map(|v| v.is_default).collect::<Vec<_>>(),
            [true, false, false]
        );
        assert_dense(&view_positions(&views));
    }

    #[tokio::test]
    async fn without_cached_versions_there_are_three_lists_each_with_one_list_view() {
        let (_dir, db) = tracked_lab().await;
        let space = space_node_id(&db, NW).await;

        let lists = children(&db, space).await;

        assert_eq!(names(&lists), ["Current sprint", "Epics", "Open bugs"]);
        assert_dense(&positions(&lists));
        for (name, _) in &lists {
            let views = views_of(&db, list_id(&db, name).await).await.unwrap();
            assert_eq!(view_names(&views), [LIST_VIEW], "{name}");
            assert_eq!(views[0].layout, Layout::List, "{name}");
            assert_eq!(views[0].group_by, GroupBy::Status, "{name}");
        }
    }

    #[tokio::test]
    async fn the_release_list_names_the_unreleased_version_due_first() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        execute(
            &db,
            "INSERT INTO cache.project (id, key, name, raw) VALUES ('10001', 'NW', 'Website relaunch', '{}');
             INSERT INTO cache.version (id, project_id, name, release_date, released) VALUES
               ('20001', '10001', '2.3.1', '2026-09-01', 1),
               ('20002', '10001', '2.5.0', '2026-12-01', 0),
               ('20003', '10001', '2.4.0', '2026-11-15', 0),
               ('20004', '10001', '3.0.0', NULL, 0)",
        )
        .await;
        track_project(&db, NW.into(), true).await.unwrap();

        let lists = children(&db, space_node_id(&db, NW).await).await;

        assert_eq!(
            names(&lists),
            ["Current sprint", "Epics", "Release 2.4.0", "Open bugs"]
        );
        let stored: serde_json::Value = serde_json::from_str(
            &text(
                &db,
                "SELECT filter FROM filter_list WHERE name = 'Release 2.4.0'",
            )
            .await,
        )
        .unwrap();
        assert_eq!(
            stored,
            json!({ "conditions": [{ "field": "release", "op": "is", "value": "20003" }] })
        );
    }

    #[tokio::test]
    async fn the_default_lists_store_their_conditions() {
        let (_dir, db) = tracked_lab().await;

        let stored = |name: &'static str| {
            let db = db.clone();
            async move {
                let json = text(
                    &db,
                    format!("SELECT filter FROM filter_list WHERE name = '{name}'"),
                )
                .await;
                serde_json::from_str::<serde_json::Value>(&json).unwrap()
            }
        };

        assert_eq!(
            stored("Current sprint").await,
            json!({ "conditions": [{ "field": "sprint", "op": "is", "value": "active" }] })
        );
        assert_eq!(
            stored("Epics").await,
            json!({ "conditions": [{ "field": "type", "op": "is", "value": "Epic" }] })
        );
        assert_eq!(
            stored("Open bugs").await,
            json!({ "conditions": [
                { "field": "type", "op": "is", "value": "Bug" },
                { "field": "statusCategory", "op": "isNot", "value": "done" },
            ] })
        );
    }

    #[tokio::test]
    async fn tracking_again_after_untracking_creates_nothing_twice() {
        let (_dir, db) = tracked_lab().await;
        let nodes = number(&db, "SELECT COUNT(*) FROM filter_list").await;
        let views = number(&db, "SELECT COUNT(*) FROM view").await;

        track_project(&db, NW.into(), false).await.unwrap();
        track_project(&db, NW.into(), true).await.unwrap();
        track_project(&db, NW.into(), true).await.unwrap();

        assert_eq!(number(&db, "SELECT COUNT(*) FROM filter_list").await, nodes);
        assert_eq!(number(&db, "SELECT COUNT(*) FROM view").await, views);
    }

    #[tokio::test]
    async fn an_untracked_space_leaves_the_tree_and_comes_back_with_its_lists() {
        let (_dir, db) = tracked_lab().await;
        track_project(&db, NWS.into(), true).await.unwrap();

        track_project(&db, NW.into(), false).await.unwrap();
        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        assert_eq!(
            tree.spaces
                .iter()
                .map(|s| s.key.as_str())
                .collect::<Vec<_>>(),
            ["NWS"]
        );

        track_project(&db, NW.into(), true).await.unwrap();
        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        let nw = find_space(&tree, "NW");
        assert_eq!(
            nw.lists.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
            ["Current sprint", "Epics", "Open bugs"]
        );
    }

    #[tokio::test]
    async fn tracking_a_project_never_marked_as_tracked_makes_no_space_node() {
        let (_dir, db) = lab().await;

        track_project(&db, NWS.into(), false).await.unwrap();

        assert_eq!(
            number(&db, "SELECT COUNT(*) FROM filter_list WHERE kind = 'space'").await,
            0
        );
    }

    #[tokio::test]
    async fn ensure_tracked_spaces_gives_earlier_tracked_spaces_their_defaults_once() {
        let (_dir, db) = lab().await;
        execute(
            &db,
            "INSERT INTO space (project_id, tracked, position) VALUES ('10001', 1, 0), ('10002', 0, 1)",
        )
        .await;

        ensure_tracked_spaces(&db).await.unwrap();
        let lists = children(&db, space_node_id(&db, NW).await).await;
        let nodes = number(&db, "SELECT COUNT(*) FROM filter_list").await;
        ensure_tracked_spaces(&db).await.unwrap();

        assert_eq!(names(&lists), ["Current sprint", "Epics", "Open bugs"]);
        assert_eq!(number(&db, "SELECT COUNT(*) FROM filter_list").await, nodes);
        assert_eq!(
            number(&db, "SELECT COUNT(*) FROM filter_list WHERE kind = 'space'").await,
            1
        );
    }
}

mod the_seeded_levels {
    use super::*;

    #[tokio::test]
    async fn everything_and_the_home_lists_start_with_one_all_issues_view() {
        let (_dir, db) = lab().await;

        for (kind, group) in [
            ("everything", GroupBy::Project),
            ("recent", GroupBy::Project),
            ("pinned", GroupBy::None),
        ] {
            let views = views_of(&db, node_id(&db, kind).await).await.unwrap();

            assert_eq!(view_names(&views), ["All issues"], "{kind}");
            assert_eq!(views[0].layout, Layout::List, "{kind}");
            assert_eq!(views[0].group_by, group, "{kind}");
            assert!(views[0].is_default, "{kind}");
        }
    }

    #[tokio::test]
    async fn home_holds_recently_changed_and_pinned_without_counts() {
        let (_dir, db) = lab().await;

        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();

        let home: Vec<(Kind, &str, Option<i64>)> = tree
            .home
            .iter()
            .map(|n| (n.kind, n.name.as_str(), n.count))
            .collect();
        assert_eq!(
            home,
            [
                (Kind::Recent, "Recently changed", None),
                (Kind::Pinned, "Pinned", None)
            ]
        );
        assert_eq!(tree.everything.kind, Kind::Everything);
        assert_eq!(tree.everything.count, Some(0));
        assert!(tree.spaces.is_empty());
    }

    #[tokio::test]
    async fn a_list_whose_stored_filter_is_broken_shows_an_error_and_no_count() {
        let (_dir, db) = tracked_lab().await;
        execute(
            &db,
            "UPDATE filter_list SET filter = 'not json' WHERE name = 'Epics'",
        )
        .await;

        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();

        let nw = find_space(&tree, "NW");
        let epics = find_list(nw, "Epics");
        assert_eq!(epics.count, None);
        assert!(epics.error.is_some());
        let sprint = find_list(nw, "Current sprint");
        assert_eq!((sprint.count, sprint.error.clone()), (Some(0), None));
        assert_eq!(nw.open_count, 0);
    }

    #[tokio::test]
    async fn every_list_node_carries_its_own_conditions_and_a_description() {
        let (_dir, db) = tracked_lab().await;

        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();

        let nw = find_space(&tree, "NW");
        let bugs = find_list(nw, "Open bugs");
        assert_eq!(bugs.own_conditions, 2);
        assert!(!bugs.description.is_empty());
        let epics = find_list(nw, "Epics");
        assert_eq!(epics.own_conditions, 1);
        assert!(!epics.description.is_empty());
        assert_eq!(tree.everything.own_conditions, 0);
    }
}

mod the_tree_on_the_demo_site {
    use super::*;

    struct Synced {
        _fake: FakeJira,
        _dir: TempDir,
        ctx: SyncContext,
    }

    impl Synced {
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
                today(),
            )
            .await
            .unwrap();
            let client = fake.client(TokenKind::Classic);
            refresh_projects(&ctx.db, &client).await.unwrap();
            for id in PROJECT_IDS {
                fetch_versions(&ctx.db, &client, id.into()).await.unwrap();
                track_project(&ctx.db, id.into(), true).await.unwrap();
            }
            let synced = Self {
                _fake: fake,
                _dir: dir,
                ctx,
            };
            synced.sync().await;
            synced
        }

        async fn sync(&self) {
            let emit: Emit = Arc::new(|_| {});
            full_sync(&self.ctx, &*emit).await.unwrap();
        }

        fn db(&self) -> &Db {
            &self.ctx.db
        }

        async fn tree(&self) -> SidebarTree {
            sidebar_tree(self.db(), viewer(Some(ME_ACCOUNT_ID)))
                .await
                .unwrap()
        }
    }

    #[tokio::test]
    async fn spaces_count_their_open_issues_and_everything_sums_them() {
        let site = Synced::new().await;

        let tree = site.tree().await;

        let counts: Vec<(&str, &str, i64)> = tree
            .spaces
            .iter()
            .map(|s| (s.key.as_str(), s.name.as_str(), s.open_count))
            .collect();
        assert_eq!(
            counts,
            [
                ("NW", "Website relaunch", 21),
                ("NWS", "Northwind support", 6),
                ("LUM", "Lumen iOS app", 9),
                ("FBA", "Fernweh booking API", 11),
                ("INT", "Internal tools", 8),
            ]
        );
        assert_eq!(tree.everything.count, Some(55));
        assert_eq!(
            tree.spaces
                .iter()
                .map(|s| s.project_id.as_str())
                .collect::<Vec<_>>(),
            PROJECT_IDS
        );
    }

    #[tokio::test]
    async fn the_website_relaunch_default_lists_count_their_open_issues() {
        let site = Synced::new().await;

        let tree = site.tree().await;

        let nw = find_space(&tree, "NW");
        let lists: Vec<(&str, Option<i64>)> = nw
            .lists
            .iter()
            .map(|l| (l.name.as_str(), l.count))
            .collect();
        assert_eq!(
            lists,
            [
                ("Current sprint", Some(8)),
                ("Epics", Some(5)),
                ("Release 2.4.0", Some(9)),
                ("Open bugs", Some(3)),
            ]
        );
        assert!(nw.lists.iter().all(|l| l.error.is_none()));
    }

    #[tokio::test]
    async fn the_archive_project_stays_out_of_the_tree() {
        let site = Synced::new().await;

        let tree = site.tree().await;

        assert!(tree.spaces.iter().all(|s| s.key != "ARC"));
        assert_eq!(tree.spaces.len(), 5);
    }

    #[tokio::test]
    async fn a_reviewer_list_counts_the_issues_in_review_waiting_for_me_after_the_next_sync() {
        let site = Synced::new().await;
        let in_review = text(
            site.db(),
            "SELECT id FROM cache.status WHERE name = 'In Review'",
        )
        .await;
        let nw = space_node_id(site.db(), NW).await;

        let created = list_create(
            site.db(),
            viewer(Some(ME_ACCOUNT_ID)),
            nw,
            "Code reviews".into(),
            Some(filter(vec![
                Condition::is(Field::Status, in_review),
                Condition::is(Field::Custom(REVIEWER.into()), "me"),
            ])),
        )
        .await
        .unwrap();
        site.sync().await;

        let tree = site.tree().await;
        let reviews = find_list(find_space(&tree, "NW"), "Code reviews");
        assert_eq!(reviews.id, created);
        assert_eq!(reviews.count, Some(2));
        assert_eq!(reviews.own_conditions, 2);
        assert_eq!(
            number(
                site.db(),
                format!("SELECT shown FROM cache.field WHERE id = '{REVIEWER}'")
            )
            .await,
            1
        );
    }

    #[tokio::test]
    async fn a_list_never_counts_issues_of_another_project() {
        let site = Synced::new().await;
        let viewer = viewer(Some(ME_ACCOUNT_ID));
        let lum = space_node_id(site.db(), "10003").await;
        let everything = node_id(site.db(), "everything").await;

        list_create(site.db(), viewer.clone(), lum, "Bugs".into(), Some(bugs()))
            .await
            .unwrap();
        list_create(
            site.db(),
            viewer,
            everything,
            "All bugs".into(),
            Some(bugs()),
        )
        .await
        .unwrap();

        let tree = site.tree().await;
        let open_bugs_per_space: Vec<i64> = tree
            .spaces
            .iter()
            .map(|s| find_list(s, "Open bugs").count.unwrap())
            .collect();
        assert_eq!(
            find_list(find_space(&tree, "LUM"), "Bugs").count,
            find_list(find_space(&tree, "LUM"), "Open bugs").count
        );
        let global = tree.home.iter().find(|n| n.name == "All bugs").unwrap();
        assert_eq!(global.count, Some(open_bugs_per_space.iter().sum::<i64>()));
        assert!(global.count > find_list(find_space(&tree, "LUM"), "Bugs").count);
    }

    #[tokio::test]
    async fn the_match_count_of_a_space_is_the_space_and_the_draft() {
        let site = Synced::new().await;
        let nw = space_node_id(site.db(), NW).await;
        let tree = site.tree().await;
        let open_bugs = find_list(find_space(&tree, "NW"), "Open bugs")
            .count
            .unwrap();
        let all_bugs = number(
            site.db(),
            "SELECT COUNT(*) FROM cache.issue WHERE project_id = '10001' AND issue_type = 'Bug'",
        )
        .await;

        let everything_in_the_space = list_match_count(
            site.db(),
            viewer(Some(ME_ACCOUNT_ID)),
            nw,
            Filter::default(),
        )
        .await
        .unwrap();
        let bugs_in_the_space =
            list_match_count(site.db(), viewer(Some(ME_ACCOUNT_ID)), nw, bugs())
                .await
                .unwrap();

        assert_eq!(
            (everything_in_the_space.open, everything_in_the_space.total),
            (21, 33)
        );
        assert_eq!(bugs_in_the_space.open, open_bugs);
        assert_eq!(bugs_in_the_space.total, all_bugs);
    }

    #[tokio::test]
    async fn the_match_count_of_a_list_replaces_its_own_filter_but_keeps_its_ancestors() {
        let site = Synced::new().await;
        let nw = space_node_id(site.db(), NW).await;
        let epics = number(
            site.db(),
            format!("SELECT id FROM filter_list WHERE name = 'Epics' AND parent_id = {nw}"),
        )
        .await;
        let me = viewer(Some(ME_ACCOUNT_ID));

        let from_the_list = list_match_count(site.db(), me.clone(), epics, bugs())
            .await
            .unwrap();
        let from_the_space = list_match_count(site.db(), me, nw, bugs()).await.unwrap();

        assert_eq!(from_the_list, from_the_space);
    }

    #[tokio::test]
    async fn the_match_count_on_everything_covers_all_tracked_projects() {
        let site = Synced::new().await;
        let everything = node_id(site.db(), "everything").await;

        let count = list_match_count(
            site.db(),
            viewer(Some(ME_ACCOUNT_ID)),
            everything,
            Filter::default(),
        )
        .await
        .unwrap();

        assert_eq!(count.open, 55);
        assert_eq!(count.total, 74);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_filter_fields_are_the_standard_ones_then_the_custom_fields_worth_filtering() {
        let site = Synced::new().await;

        let options = filter_options(site.db()).await.unwrap();

        let ids: Vec<&str> = options.fields.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(
            ids[..14],
            [
                "status",
                "statusCategory",
                "type",
                "assignee",
                "reporter",
                "priority",
                "epic",
                "release",
                "sprint",
                "labels",
                "due",
                "updated",
                "created",
                "project",
            ]
        );
        let kind_of = |id: &str| options.fields.iter().find(|f| f.id == id).map(|f| f.kind);
        assert_eq!(kind_of(REVIEWER), Some(ValueKind::User));
        assert_eq!(kind_of("customfield_10016"), Some(ValueKind::Number));
        for left_out in [
            "customfield_10020",
            "customfield_10017",
            "customfield_10019",
        ] {
            assert_eq!(kind_of(left_out), None, "{left_out}");
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_filter_choices_come_from_the_cache_in_a_useful_order() {
        let site = Synced::new().await;

        let options = filter_options(site.db()).await.unwrap();

        assert!(!options.statuses.is_empty());
        let priorities: Vec<&str> = options.priorities.iter().map(|c| c.name.as_str()).collect();
        let rank = |name: &str| priorities.iter().position(|p| *p == name);
        for (higher, lower) in [("Highest", "High"), ("High", "Medium"), ("Medium", "Low")] {
            if let (Some(a), Some(b)) = (rank(higher), rank(lower)) {
                assert!(a < b, "{higher} before {lower}: {priorities:?}");
            }
        }
        let sprint_ids: Vec<&str> = options.sprints.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(sprint_ids[..3], ["active", "future", "closed"]);
        let tracked_epics = number(
            site.db(),
            "SELECT COUNT(*) FROM cache.issue
             WHERE is_epic = 1 AND gone = 0 AND project_id IN (SELECT project_id FROM space WHERE tracked = 1)",
        )
        .await;
        assert_eq!(options.epics.len() as i64, tracked_epics);
        assert!(options.epics.iter().any(|c| c.name == "NW · Checkout"));
        assert!(options.releases.iter().any(|c| c.name == "NW · 2.4.0"));
        let mut labels: Vec<&str> = options.labels.iter().map(|c| c.id.as_str()).collect();
        let total = labels.len();
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), total);
    }
}

mod the_conditions_a_level_shows {
    use super::*;

    fn locked(source: &str) -> LockedCondition {
        LockedCondition {
            field: "Project".into(),
            op: "=".into(),
            value: "NW".into(),
            source: source.into(),
        }
    }

    #[tokio::test]
    async fn everything_has_no_locked_and_no_own_conditions() {
        let (_dir, db) = tracked_lab().await;

        let shown = list_conditions(&db, node_id(&db, "everything").await)
            .await
            .unwrap();

        assert_eq!(
            shown,
            ListConditions {
                kind: Kind::Everything,
                locked: vec![],
                own: vec![],
                error: None,
            }
        );
    }

    #[tokio::test]
    async fn a_space_locks_its_project_and_has_no_own_conditions() {
        let (_dir, db) = tracked_lab().await;

        let shown = list_conditions(&db, space_node_id(&db, NW).await)
            .await
            .unwrap();

        assert_eq!(
            shown,
            ListConditions {
                kind: Kind::Space,
                locked: vec![locked("this Space")],
                own: vec![],
                error: None,
            }
        );
    }

    #[tokio::test]
    async fn a_list_in_a_space_locks_the_spaces_project_and_keeps_its_stored_conditions_as_own() {
        let (_dir, db) = tracked_lab().await;
        let open_bugs = list_id(&db, "Open bugs").await;
        let stored = text(
            &db,
            format!("SELECT filter FROM filter_list WHERE id = {open_bugs}"),
        )
        .await;

        let shown = list_conditions(&db, open_bugs).await.unwrap();

        assert_eq!(shown.kind, Kind::List);
        assert_eq!(shown.locked, [locked("from Space")]);
        assert_eq!(shown.own, Filter::parse(Some(&stored)).unwrap().conditions);
        assert_eq!(shown.own.len(), 2);
        assert_eq!(shown.error, None);
    }

    #[tokio::test]
    async fn a_list_whose_stored_filter_is_broken_reports_an_error() {
        let (_dir, db) = tracked_lab().await;
        execute(
            &db,
            "UPDATE filter_list SET filter = 'not json' WHERE name = 'Epics'",
        )
        .await;

        let shown = list_conditions(&db, list_id(&db, "Epics").await)
            .await
            .unwrap();

        assert!(shown.error.is_some());
        assert!(shown.own.is_empty());
    }

    #[tokio::test]
    async fn an_unknown_level_is_not_found() {
        let (_dir, db) = tracked_lab().await;

        let err = list_conditions(&db, 9999).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }
}

mod editing_filter_lists {
    use super::*;

    async fn nw_lists(db: &Db) -> Vec<(String, i64)> {
        children(db, space_node_id(db, NW).await).await
    }

    #[tokio::test]
    async fn a_new_list_goes_last_in_its_space_with_one_list_view() {
        let (_dir, db) = tracked_lab().await;
        let space = space_node_id(&db, NW).await;

        let id = list_create(
            &db,
            viewer(None),
            space,
            "  Code reviews ".into(),
            Some(bugs()),
        )
        .await
        .unwrap();

        let lists = nw_lists(&db).await;
        assert_eq!(
            names(&lists),
            ["Current sprint", "Epics", "Open bugs", "Code reviews"]
        );
        assert_dense(&positions(&lists));
        let views = views_of(&db, id).await.unwrap();
        assert_eq!(view_names(&views), [LIST_VIEW]);
    }

    #[tokio::test]
    async fn a_new_list_under_everything_goes_to_the_end_of_home() {
        let (_dir, db) = tracked_lab().await;
        let everything = node_id(&db, "everything").await;

        list_create(&db, viewer(None), everything, "My bugs".into(), None)
            .await
            .unwrap();

        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        assert_eq!(
            tree.home
                .iter()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>(),
            ["Recently changed", "Pinned", "My bugs"]
        );
        assert_eq!(tree.home[2].kind, Kind::List);
        assert_eq!(
            number(
                &db,
                "SELECT position FROM filter_list WHERE name = 'My bugs'"
            )
            .await,
            2
        );
    }

    #[tokio::test]
    async fn a_list_needs_a_name_and_a_space_or_everything_as_parent() {
        let (_dir, db) = tracked_lab().await;
        let space = space_node_id(&db, NW).await;
        let epics = list_id(&db, "Epics").await;

        let empty = list_create(&db, viewer(None), space, "   ".into(), None)
            .await
            .unwrap_err();
        let under_a_list = list_create(&db, viewer(None), epics, "Nested".into(), None)
            .await
            .unwrap_err();
        let under_nothing = list_create(&db, viewer(None), 9999, "Lost".into(), None)
            .await
            .unwrap_err();

        assert_eq!(empty.kind(), ErrorKind::InvalidInput);
        assert_eq!(under_a_list.kind(), ErrorKind::InvalidInput);
        assert_eq!(under_nothing.kind(), ErrorKind::NotFound);
        assert_eq!(nw_lists(&db).await.len(), 3);
    }

    #[tokio::test]
    async fn a_list_with_an_unknown_custom_field_is_rejected_and_not_stored() {
        let (_dir, db) = tracked_lab().await;
        let space = space_node_id(&db, NW).await;

        let err = list_create(
            &db,
            viewer(None),
            space,
            "Ghost".into(),
            Some(filter(vec![Condition::is(
                Field::Custom("customfield_99999".into()),
                "x",
            )])),
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(nw_lists(&db).await.len(), 3);
    }

    #[tokio::test]
    async fn renaming_trims_the_name_and_refuses_an_empty_one() {
        let (_dir, db) = tracked_lab().await;
        let epics = list_id(&db, "Epics").await;

        list_rename(&db, epics, "  Big rocks ".into())
            .await
            .unwrap();
        let err = list_rename(&db, epics, "  ".into()).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(names(&nw_lists(&db).await)[1], "Big rocks");
    }

    #[tokio::test]
    async fn a_duplicate_sits_right_after_the_original_with_copies_of_its_views() {
        let (_dir, db) = tracked_lab().await;
        let epics = list_id(&db, "Epics").await;
        let extra = view_create(&db, epics, "Board".into(), Layout::Board)
            .await
            .unwrap();
        view_set_sort(
            &db,
            extra.id,
            Sort {
                by: SortBy::Key,
                descending: false,
            },
        )
        .await
        .unwrap();

        let copy = list_duplicate(&db, epics).await.unwrap();

        let lists = nw_lists(&db).await;
        assert_eq!(
            names(&lists),
            ["Current sprint", "Epics", "Epics copy", "Open bugs"]
        );
        assert_dense(&positions(&lists));
        assert_eq!(list_id(&db, "Epics copy").await, copy);
        let original = views_of(&db, epics).await.unwrap();
        let copied = views_of(&db, copy).await.unwrap();
        let shape = |views: &[View]| -> Vec<_> {
            views
                .iter()
                .map(|v| {
                    (
                        v.name.clone(),
                        v.layout,
                        v.columns.clone(),
                        v.group_by,
                        v.sort,
                        v.position,
                        v.is_default,
                    )
                })
                .collect()
        };
        assert_eq!(shape(&copied), shape(&original));
        assert_eq!(
            text(
                &db,
                format!("SELECT filter FROM filter_list WHERE id = {copy}")
            )
            .await,
            text(
                &db,
                format!("SELECT filter FROM filter_list WHERE id = {epics}")
            )
            .await
        );
    }

    #[tokio::test]
    async fn moving_reorders_among_siblings_and_clamps_the_position() {
        let (_dir, db) = tracked_lab().await;
        let bugs_id = list_id(&db, "Open bugs").await;

        list_move(&db, bugs_id, 0).await.unwrap();
        let first = nw_lists(&db).await;
        list_move(&db, bugs_id, 99).await.unwrap();
        let last = nw_lists(&db).await;
        list_move(&db, bugs_id, -4).await.unwrap();
        let negative = nw_lists(&db).await;

        assert_eq!(names(&first), ["Open bugs", "Current sprint", "Epics"]);
        assert_eq!(names(&last), ["Current sprint", "Epics", "Open bugs"]);
        assert_eq!(names(&negative), ["Open bugs", "Current sprint", "Epics"]);
        for lists in [first, last, negative] {
            assert_dense(&positions(&lists));
        }
    }

    #[tokio::test]
    async fn deleting_removes_the_views_and_the_pins_and_closes_the_gap() {
        let (_dir, db) = tracked_lab().await;
        let epics = list_id(&db, "Epics").await;
        let other = list_id(&db, "Open bugs").await;
        execute(
            &db,
            "INSERT INTO pin (target_kind, target_id, created_at) VALUES ('issue', 'x', '2026-10-28T11:00:00Z')",
        )
        .await;
        db.write(move |tx, _| {
            for target in [epics, other] {
                tx.execute(
                    "INSERT INTO pin (target_kind, target_id, created_at)
                     VALUES ('filter_list', ?1, '2026-10-28T11:00:00Z')",
                    [target.to_string()],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();

        list_delete(&db, epics).await.unwrap();

        let lists = nw_lists(&db).await;
        assert_eq!(names(&lists), ["Current sprint", "Open bugs"]);
        assert_dense(&positions(&lists));
        assert_eq!(
            number(
                &db,
                format!("SELECT COUNT(*) FROM view WHERE list_id = {epics}")
            )
            .await,
            0
        );
        let pins = rows(
            &db,
            "SELECT target_kind, target_id FROM pin ORDER BY id",
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .await;
        assert_eq!(
            pins,
            [
                ("issue".to_string(), "x".to_string()),
                ("filter_list".to_string(), other.to_string())
            ]
        );
    }

    #[tokio::test]
    async fn deleting_a_home_list_closes_the_gap_in_home() {
        let (_dir, db) = tracked_lab().await;
        let everything = node_id(&db, "everything").await;
        let first = list_create(&db, viewer(None), everything, "First".into(), None)
            .await
            .unwrap();
        list_create(&db, viewer(None), everything, "Second".into(), None)
            .await
            .unwrap();

        list_delete(&db, first).await.unwrap();

        let home = rows(
            &db,
            "SELECT name, position FROM filter_list
             WHERE parent_id IS NULL AND kind IN ('recent', 'pinned', 'list') ORDER BY position",
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .await;
        assert_eq!(names(&home), ["Recently changed", "Pinned", "Second"]);
        assert_dense(&positions(&home));
    }

    #[tokio::test]
    async fn spaces_everything_and_the_built_in_lists_cannot_be_edited_as_lists() {
        let (_dir, db) = tracked_lab().await;
        let nodes_before = number(&db, "SELECT COUNT(*) FROM filter_list").await;

        for kind in ["space", "everything", "recent", "pinned"] {
            let id = node_id(&db, kind).await;
            let errors = [
                list_rename(&db, id, "Renamed".into()).await.unwrap_err(),
                list_delete(&db, id).await.unwrap_err(),
                list_duplicate(&db, id).await.unwrap_err(),
                list_move(&db, id, 0).await.unwrap_err(),
                list_set_filter(&db, viewer(None), id, bugs())
                    .await
                    .unwrap_err(),
            ];

            for err in errors {
                assert_eq!(err.kind(), ErrorKind::InvalidInput, "{kind}");
            }
        }

        assert_eq!(
            number(&db, "SELECT COUNT(*) FROM filter_list").await,
            nodes_before
        );
        assert_eq!(
            number(
                &db,
                "SELECT COUNT(*) FROM filter_list WHERE name = 'Renamed'"
            )
            .await,
            0
        );
    }

    #[tokio::test]
    async fn setting_a_filter_stores_it_and_an_empty_one_clears_it() {
        let (_dir, db) = tracked_lab().await;
        let epics = list_id(&db, "Epics").await;

        list_set_filter(&db, viewer(None), epics, bugs())
            .await
            .unwrap();
        let stored: serde_json::Value = serde_json::from_str(
            &text(
                &db,
                format!("SELECT filter FROM filter_list WHERE id = {epics}"),
            )
            .await,
        )
        .unwrap();
        list_set_filter(&db, viewer(None), epics, Filter::default())
            .await
            .unwrap();

        assert_eq!(
            stored,
            json!({ "conditions": [{ "field": "type", "op": "is", "value": "Bug" }] })
        );
        assert_eq!(
            number(
                &db,
                format!("SELECT filter IS NULL FROM filter_list WHERE id = {epics}")
            )
            .await,
            1
        );
    }

    #[tokio::test]
    async fn a_filter_with_an_unknown_custom_field_is_rejected_and_the_old_one_stays() {
        let (_dir, db) = tracked_lab().await;
        let epics = list_id(&db, "Epics").await;
        let before = text(
            &db,
            format!("SELECT filter FROM filter_list WHERE id = {epics}"),
        )
        .await;

        let err = list_set_filter(
            &db,
            viewer(None),
            epics,
            filter(vec![Condition::is(
                Field::Custom("customfield_99999".into()),
                "x",
            )]),
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(
            text(
                &db,
                format!("SELECT filter FROM filter_list WHERE id = {epics}")
            )
            .await,
            before
        );
    }
}

mod editing_views {
    use super::*;

    async fn space_views(db: &Db) -> Vec<View> {
        views_of(db, space_node_id(db, NW).await).await.unwrap()
    }

    #[tokio::test]
    async fn a_new_view_goes_last_with_the_apps_default_columns() {
        let (_dir, db) = tracked_lab().await;
        let space = space_node_id(&db, NW).await;

        let created = view_create(&db, space, "  Roadmap ".into(), Layout::Table)
            .await
            .unwrap();

        assert_eq!(created.name, "Roadmap");
        assert_eq!(created.layout, Layout::Table);
        assert_eq!(created.columns, defaults_of(&DEFAULT_COLUMNS));
        assert_eq!(created.group_by, GroupBy::Status);
        assert_eq!(created.position, 3);
        assert!(!created.is_default);
        assert_eq!(created.sort, Sort::default());
        assert_dense(&view_positions(&space_views(&db).await));
    }

    #[tokio::test]
    async fn new_views_start_with_the_spaces_default_columns_once_it_has_some() {
        let (_dir, db) = tracked_lab().await;
        let space = space_node_id(&db, NW).await;
        let epics = list_id(&db, "Epics").await;
        space_default_columns(
            &db,
            NW.into(),
            vec!["assignee".into(), "due".into(), "assignee".into()],
        )
        .await
        .unwrap();

        let in_space = view_create(&db, space, "Plan".into(), Layout::List)
            .await
            .unwrap();
        let in_list = view_create(&db, epics, "Plan".into(), Layout::List)
            .await
            .unwrap();

        assert_eq!(in_space.columns, defaults_of(&["assignee", "due"]));
        assert_eq!(in_list.columns, defaults_of(&["assignee", "due"]));
    }

    #[tokio::test]
    async fn a_space_default_needs_a_space_and_clean_column_ids() {
        let (_dir, db) = tracked_lab().await;

        let unknown = space_default_columns(&db, "99999".into(), vec!["assignee".into()])
            .await
            .unwrap_err();
        let empty_id = space_default_columns(&db, NW.into(), vec![" ".into()])
            .await
            .unwrap_err();

        assert_eq!(unknown.kind(), ErrorKind::NotFound);
        assert_eq!(empty_id.kind(), ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn new_views_of_everything_group_by_project() {
        let (_dir, db) = tracked_lab().await;
        let everything = node_id(&db, "everything").await;

        let created = view_create(&db, everything, "By client".into(), Layout::Table)
            .await
            .unwrap();

        assert_eq!(created.group_by, GroupBy::Project);
        assert_eq!(created.position, 1);
    }

    #[tokio::test]
    async fn creating_a_view_needs_a_name_and_an_existing_level() {
        let (_dir, db) = tracked_lab().await;
        let space = space_node_id(&db, NW).await;

        let blank = view_create(&db, space, " ".into(), Layout::List)
            .await
            .unwrap_err();
        let nowhere = view_create(&db, 9999, "Lost".into(), Layout::List)
            .await
            .unwrap_err();

        assert_eq!(blank.kind(), ErrorKind::InvalidInput);
        assert_eq!(nowhere.kind(), ErrorKind::NotFound);
        assert_eq!(space_views(&db).await.len(), 3);
    }

    #[tokio::test]
    async fn renaming_trims_and_refuses_an_empty_name() {
        let (_dir, db) = tracked_lab().await;
        let board = space_views(&db).await[1].id;

        let renamed = view_rename(&db, board, " Kanban ".into()).await.unwrap();
        let err = view_rename(&db, board, "".into()).await.unwrap_err();

        assert_eq!(renamed.name, "Kanban");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn a_duplicate_sits_right_after_the_original_and_is_not_the_default() {
        let (_dir, db) = tracked_lab().await;
        let views = space_views(&db).await;
        view_set_sort(
            &db,
            views[0].id,
            Sort {
                by: SortBy::Priority,
                descending: false,
            },
        )
        .await
        .unwrap();

        let copy = view_duplicate(&db, views[0].id).await.unwrap();

        let after = space_views(&db).await;
        assert_eq!(
            view_names(&after),
            ["All issues", "All issues copy", "Board", "Table"]
        );
        assert_dense(&view_positions(&after));
        assert_eq!(copy.position, 1);
        assert!(!copy.is_default);
        assert_eq!(copy.layout, Layout::List);
        assert_eq!(copy.columns, views[0].columns);
        assert_eq!(
            copy.sort,
            Sort {
                by: SortBy::Priority,
                descending: false
            }
        );
        assert!(after[0].is_default);
    }

    #[tokio::test]
    async fn layout_group_and_sort_are_stored_per_view() {
        let (_dir, db) = tracked_lab().await;
        let id = space_views(&db).await[0].id;

        view_set_layout(&db, id, Layout::Table).await.unwrap();
        view_set_group(&db, id, GroupBy::Assignee).await.unwrap();
        let view = view_set_sort(
            &db,
            id,
            Sort {
                by: SortBy::Due,
                descending: false,
            },
        )
        .await
        .unwrap();

        assert_eq!(view.layout, Layout::Table);
        assert_eq!(view.group_by, GroupBy::Assignee);
        assert_eq!(
            view.sort,
            Sort {
                by: SortBy::Due,
                descending: false
            }
        );
        let others = space_views(&db).await;
        assert_eq!(others[1].layout, Layout::Board);
        assert_eq!(others[1].group_by, GroupBy::Status);
    }

    #[tokio::test]
    async fn columns_are_deduplicated_in_order_and_an_empty_id_is_refused() {
        let (_dir, db) = tracked_lab().await;
        let id = space_views(&db).await[0].id;

        let view = view_set_columns(
            &db,
            id,
            vec![
                " due ".into(),
                "assignee".into(),
                "due".into(),
                "customfield_10071".into(),
            ],
        )
        .await
        .unwrap();
        let err = view_set_columns(&db, id, vec!["assignee".into(), "  ".into()])
            .await
            .unwrap_err();

        assert_eq!(
            view.columns,
            defaults_of(&["due", "assignee", "customfield_10071"])
        );
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(
            space_views(&db).await[0].columns,
            defaults_of(&["due", "assignee", "customfield_10071"])
        );
    }

    #[tokio::test]
    async fn moving_reorders_the_tabs_and_clamps_the_position() {
        let (_dir, db) = tracked_lab().await;
        let table = space_views(&db).await[2].id;

        let first = view_move(&db, table, 0).await.unwrap();
        let last = view_move(&db, table, 99).await.unwrap();

        assert_eq!(view_names(&first), ["Table", "All issues", "Board"]);
        assert_eq!(view_names(&last), ["All issues", "Board", "Table"]);
        assert_dense(&view_positions(&first));
        assert_dense(&view_positions(&last));
        assert_eq!(
            last.iter().map(|v| v.is_default).collect::<Vec<_>>(),
            [true, false, false]
        );
    }

    #[tokio::test]
    async fn deleting_the_default_view_makes_the_first_remaining_one_the_default() {
        let (_dir, db) = tracked_lab().await;
        let views = space_views(&db).await;

        let left = view_delete(&db, views[0].id).await.unwrap();

        assert_eq!(view_names(&left), ["Board", "Table"]);
        assert_dense(&view_positions(&left));
        assert_eq!(
            left.iter().map(|v| v.is_default).collect::<Vec<_>>(),
            [true, false]
        );
    }

    #[tokio::test]
    async fn deleting_another_view_keeps_the_default_and_closes_the_gap() {
        let (_dir, db) = tracked_lab().await;
        let views = space_views(&db).await;

        let left = view_delete(&db, views[1].id).await.unwrap();

        assert_eq!(view_names(&left), ["All issues", "Table"]);
        assert_dense(&view_positions(&left));
        assert_eq!(
            left.iter().map(|v| v.is_default).collect::<Vec<_>>(),
            [true, false]
        );
    }

    #[tokio::test]
    async fn the_last_view_of_a_level_cannot_be_deleted() {
        let (_dir, db) = tracked_lab().await;
        let epics = list_id(&db, "Epics").await;
        let only = views_of(&db, epics).await.unwrap()[0].id;

        let err = view_delete(&db, only).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(views_of(&db, epics).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn an_unknown_view_is_not_found() {
        let (_dir, db) = tracked_lab().await;

        let err = view_rename(&db, 9999, "Ghost".into()).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    fn widths(pairs: &[(&str, i64)]) -> BTreeMap<String, i64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[tokio::test]
    async fn a_new_view_has_no_widths() {
        let (_dir, db) = tracked_lab().await;

        assert!(
            space_views(&db)
                .await
                .iter()
                .all(|view| view.widths.is_empty())
        );
    }

    #[tokio::test]
    async fn widths_are_stored_per_view_and_read_back_with_the_tabs() {
        let (_dir, db) = tracked_lab().await;
        let views = space_views(&db).await;

        let table = view_set_widths(&db, views[2].id, widths(&[("assignee", 120), ("due", 90)]))
            .await
            .unwrap();

        assert_eq!(table.widths, widths(&[("assignee", 120), ("due", 90)]));
        let after = space_views(&db).await;
        assert_eq!(after[2].widths, table.widths);
        assert!(after[0].widths.is_empty());
    }

    #[tokio::test]
    async fn widths_outside_the_allowed_range_are_clamped() {
        let (_dir, db) = tracked_lab().await;
        let id = space_views(&db).await[2].id;

        let view = view_set_widths(
            &db,
            id,
            widths(&[
                ("a", 10),
                ("b", 48),
                ("c", 300),
                ("d", 800),
                ("e", 5000),
                ("f", -3),
            ]),
        )
        .await
        .unwrap();

        assert_eq!(
            view.widths,
            widths(&[
                ("a", 48),
                ("b", 48),
                ("c", 300),
                ("d", 800),
                ("e", 800),
                ("f", 48)
            ])
        );
    }

    #[tokio::test]
    async fn setting_widths_replaces_the_earlier_ones() {
        let (_dir, db) = tracked_lab().await;
        let id = space_views(&db).await[2].id;
        view_set_widths(&db, id, widths(&[("assignee", 120)]))
            .await
            .unwrap();

        let view = view_set_widths(&db, id, widths(&[("due", 90)]))
            .await
            .unwrap();

        assert_eq!(view.widths, widths(&[("due", 90)]));
    }

    #[tokio::test]
    async fn widths_of_an_unknown_view_are_not_found() {
        let (_dir, db) = tracked_lab().await;

        let err = view_set_widths(&db, 9999, widths(&[("due", 90)]))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn a_duplicate_view_and_a_duplicate_list_keep_the_widths() {
        let (_dir, db) = tracked_lab().await;
        let table = space_views(&db).await[2].id;
        view_set_widths(&db, table, widths(&[("assignee", 120)]))
            .await
            .unwrap();
        let epics = list_id(&db, "Epics").await;
        let epics_view = views_of(&db, epics).await.unwrap()[0].id;
        view_set_widths(&db, epics_view, widths(&[("due", 77)]))
            .await
            .unwrap();

        let copy = view_duplicate(&db, table).await.unwrap();
        let copied_list = list_duplicate(&db, epics).await.unwrap();

        assert_eq!(copy.widths, widths(&[("assignee", 120)]));
        assert_eq!(
            views_of(&db, copied_list).await.unwrap()[0].widths,
            widths(&[("due", 77)])
        );
    }

    #[tokio::test]
    async fn a_view_serializes_for_the_page_in_camel_case() {
        let (_dir, db) = tracked_lab().await;
        let space = space_node_id(&db, NW).await;
        let first = views_of(&db, space).await.unwrap().remove(0);

        assert_eq!(
            serde_json::to_value(&first).unwrap(),
            json!({
                "id": first.id,
                "listId": space,
                "name": "All issues",
                "layout": "list",
                "columns": ["assignee", "epic", "release", "priority"],
                "groupBy": "status",
                "sort": { "by": "updated", "descending": true },
                "widths": {},
                "wrapTitles": true,
                "position": 0,
                "isDefault": true,
            })
        );
    }

    #[tokio::test]
    async fn a_new_view_wraps_titles_by_default() {
        let (_dir, db) = tracked_lab().await;

        assert!(space_views(&db).await.iter().all(|view| view.wrap_titles));
    }

    #[tokio::test]
    async fn the_wrap_setting_is_stored_per_view_and_read_back_with_the_tabs() {
        let (_dir, db) = tracked_lab().await;
        let views = space_views(&db).await;

        let table = view_set_wrap(&db, views[2].id, false).await.unwrap();

        assert!(!table.wrap_titles);
        let after = space_views(&db).await;
        assert!(!after[2].wrap_titles);
        assert!(after[0].wrap_titles);

        let back = view_set_wrap(&db, views[2].id, true).await.unwrap();
        assert!(back.wrap_titles);
    }

    #[tokio::test]
    async fn wrap_of_an_unknown_view_is_not_found() {
        let (_dir, db) = tracked_lab().await;

        let err = view_set_wrap(&db, 9999, false).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn a_duplicate_view_and_a_duplicate_list_keep_the_wrap_setting() {
        let (_dir, db) = tracked_lab().await;
        let table = space_views(&db).await[2].id;
        view_set_wrap(&db, table, false).await.unwrap();
        let epics = list_id(&db, "Epics").await;
        let epics_view = views_of(&db, epics).await.unwrap()[0].id;
        view_set_wrap(&db, epics_view, false).await.unwrap();

        let copy = view_duplicate(&db, table).await.unwrap();
        let copied_list = list_duplicate(&db, epics).await.unwrap();

        assert!(!copy.wrap_titles);
        assert!(!views_of(&db, copied_list).await.unwrap()[0].wrap_titles);
    }
}

mod editing_spaces {
    use super::*;

    async fn two_spaces() -> (TempDir, Db) {
        let (dir, db) = tracked_lab().await;
        track_project(&db, NWS.into(), true).await.unwrap();
        (dir, db)
    }

    #[tokio::test]
    async fn name_icon_color_and_client_are_stored_and_shown_in_the_tree() {
        let (_dir, db) = tracked_lab().await;

        space_update(
            &db,
            NW.into(),
            SpaceUpdate {
                name: Some("  Storefront ".into()),
                icon: Some(" S ".into()),
                color: Some("3".into()),
                client: Some(" Northwind ".into()),
            },
        )
        .await
        .unwrap();

        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        let space = find_space(&tree, "NW");
        assert_eq!(space.name, "Storefront");
        assert_eq!(space.icon.as_deref(), Some("S"));
        assert_eq!(space.color.as_deref(), Some("3"));
        assert_eq!(space.client.as_deref(), Some("Northwind"));
    }

    #[tokio::test]
    async fn blank_values_go_back_to_null_and_the_tree_shows_the_project_name() {
        let (_dir, db) = tracked_lab().await;
        space_update(
            &db,
            NW.into(),
            SpaceUpdate {
                name: Some("Storefront".into()),
                icon: Some("S".into()),
                color: Some("3".into()),
                client: Some("Northwind".into()),
            },
        )
        .await
        .unwrap();

        space_update(
            &db,
            NW.into(),
            SpaceUpdate {
                name: Some("  ".into()),
                icon: Some("".into()),
                color: Some(" ".into()),
                client: None,
            },
        )
        .await
        .unwrap();

        let stored = rows(
            &db,
            "SELECT name IS NULL, icon IS NULL, color IS NULL, client IS NULL FROM space WHERE project_id = '10001'",
            |row| {
                Ok([row.get::<_, i64>(0)?, row.get(1)?, row.get(2)?, row.get(3)?])
            },
        )
        .await;
        assert_eq!(stored, [[1, 1, 1, 1]]);
        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        let space = find_space(&tree, "NW");
        assert_eq!(space.name, "Website relaunch");
        assert_eq!(space.icon, None);
        assert_eq!(space.color, None);
        assert_eq!(space.client, None);
    }

    #[tokio::test]
    async fn a_name_equal_to_the_project_name_is_stored_as_null() {
        let (_dir, db) = tracked_lab().await;
        let name_is_null = || {
            rows(
                &db,
                "SELECT name IS NULL FROM space WHERE project_id = '10001'",
                |row| row.get::<_, i64>(0),
            )
        };

        space_update(
            &db,
            NW.into(),
            SpaceUpdate {
                name: Some("Website relaunch".into()),
                ..SpaceUpdate::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(name_is_null().await, [1]);
        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        let space = find_space(&tree, "NW");
        assert_eq!(space.name, "Website relaunch");
        assert_eq!(space.project_name, "Website relaunch");

        space_update(
            &db,
            NW.into(),
            SpaceUpdate {
                name: Some("Storefront".into()),
                ..SpaceUpdate::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(name_is_null().await, [0]);
        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        let space = find_space(&tree, "NW");
        assert_eq!(space.name, "Storefront");
        assert_eq!(space.project_name, "Website relaunch");
    }

    #[tokio::test]
    async fn an_icon_must_be_one_letter_or_digit() {
        let (_dir, db) = tracked_lab().await;

        for icon in ["AB", "!", "ÄÖ", "1 2"] {
            let err = space_update(
                &db,
                NW.into(),
                SpaceUpdate {
                    icon: Some(icon.into()),
                    ..SpaceUpdate::default()
                },
            )
            .await
            .unwrap_err();

            assert!(
                matches!(&err, AppError::InvalidField { field, .. } if field == "icon"),
                "{icon}: {err:?}"
            );
        }
        for icon in ["7", "ä"] {
            space_update(
                &db,
                NW.into(),
                SpaceUpdate {
                    icon: Some(icon.into()),
                    ..SpaceUpdate::default()
                },
            )
            .await
            .unwrap();
        }
    }

    #[tokio::test]
    async fn a_color_must_be_a_tile_number_from_one_to_five() {
        let (_dir, db) = tracked_lab().await;

        for color in ["7", "0", "6", "red", "-1"] {
            let err = space_update(
                &db,
                NW.into(),
                SpaceUpdate {
                    color: Some(color.into()),
                    ..SpaceUpdate::default()
                },
            )
            .await
            .unwrap_err();

            assert!(
                matches!(&err, AppError::InvalidField { field, .. } if field == "color"),
                "{color}: {err:?}"
            );
        }
        for color in ["1", "5"] {
            space_update(
                &db,
                NW.into(),
                SpaceUpdate {
                    color: Some(color.into()),
                    ..SpaceUpdate::default()
                },
            )
            .await
            .unwrap();
        }
    }

    #[tokio::test]
    async fn a_rejected_update_changes_nothing() {
        let (_dir, db) = tracked_lab().await;
        space_update(
            &db,
            NW.into(),
            SpaceUpdate {
                name: Some("Storefront".into()),
                ..SpaceUpdate::default()
            },
        )
        .await
        .unwrap();

        let err = space_update(
            &db,
            NW.into(),
            SpaceUpdate {
                name: Some("Renamed".into()),
                icon: Some("AB".into()),
                ..SpaceUpdate::default()
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(
            text(&db, "SELECT name FROM space WHERE project_id = '10001'").await,
            "Storefront"
        );
    }

    #[tokio::test]
    async fn updating_or_moving_a_project_without_a_space_is_not_found() {
        let (_dir, db) = tracked_lab().await;

        let update = space_update(&db, "99999".into(), SpaceUpdate::default())
            .await
            .unwrap_err();
        let moved = space_move(&db, "99999".into(), 0).await.unwrap_err();

        assert_eq!(update.kind(), ErrorKind::NotFound);
        assert_eq!(moved.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn moving_a_space_reorders_the_tree_and_keeps_positions_dense() {
        let (_dir, db) = two_spaces().await;

        space_move(&db, NWS.into(), 0).await.unwrap();

        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        assert_eq!(
            tree.spaces
                .iter()
                .map(|s| s.key.as_str())
                .collect::<Vec<_>>(),
            ["NWS", "NW"]
        );
        let stored = rows(&db, "SELECT position FROM space ORDER BY position", |row| {
            row.get::<_, i64>(0)
        })
        .await;
        assert_dense(&stored);

        space_move(&db, NWS.into(), 99).await.unwrap();
        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        assert_eq!(
            tree.spaces
                .iter()
                .map(|s| s.key.as_str())
                .collect::<Vec<_>>(),
            ["NW", "NWS"]
        );
    }

    #[tokio::test]
    async fn moving_a_space_orders_only_tracked_spaces() {
        const LUM: &str = "10003";
        let spaces = |tree: &SidebarTree| {
            tree.spaces
                .iter()
                .map(|s| s.key.clone())
                .collect::<Vec<_>>()
        };
        let (_dir, db) = two_spaces().await;
        execute(
            &db,
            "INSERT INTO cache.project (id, key, name, raw) VALUES ('10003', 'LUM', 'Lumen iOS app', '{}')",
        )
        .await;
        track_project(&db, LUM.into(), true).await.unwrap();
        track_project(&db, NWS.into(), false).await.unwrap();

        space_move(&db, LUM.into(), 0).await.unwrap();

        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        assert_eq!(spaces(&tree), ["LUM", "NW"]);

        space_move(&db, NW.into(), 0).await.unwrap();
        space_move(&db, NW.into(), 1).await.unwrap();

        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        assert_eq!(spaces(&tree), ["LUM", "NW"]);

        space_move(&db, LUM.into(), 0).await.unwrap();
        track_project(&db, NWS.into(), true).await.unwrap();

        let tree = sidebar_tree(&db, viewer(None)).await.unwrap();
        let keys = spaces(&tree);
        let position = |key: &str| keys.iter().position(|k| k == key).unwrap();
        assert_eq!(keys.len(), 3);
        assert!(position("LUM") < position("NW"), "{keys:?}");
    }
}
