//! The main window's place on temp folders: where a fresh store starts, how a level opens and remembers its
//! view, how a vanished level or view falls back, how the Space shortcuts step and what survives a restart.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::Utc;
use tempfile::TempDir;

use taskboard_lib::connection::track_project;
use taskboard_lib::db::Db;
use taskboard_lib::error::ErrorKind;
use taskboard_lib::lists::{Viewer, list_create, list_delete, space_move};
use taskboard_lib::nav::{Location, Nav, NavStore};
use taskboard_lib::test_support::temp_config_dir;
use taskboard_lib::views::{View, view_delete, views_of};

const NW: &str = "10001";
const NWS: &str = "10002";
const LUM: &str = "10003";

async fn id_where(db: &Db, condition: impl Into<String>) -> i64 {
    let condition = condition.into();
    db.read(move |conn| {
        let sql = format!("SELECT id FROM filter_list WHERE {condition} ORDER BY id LIMIT 1");
        Ok(conn.query_row(&sql, [], |row| row.get::<_, i64>(0))?)
    })
    .await
    .unwrap()
}

async fn everything(db: &Db) -> i64 {
    id_where(db, "kind = 'everything'").await
}

async fn recent(db: &Db) -> i64 {
    id_where(db, "kind = 'recent'").await
}

async fn space(db: &Db, project_id: &'static str) -> i64 {
    id_where(
        db,
        match project_id {
            NW => "kind = 'space' AND project_id = '10001'",
            NWS => "kind = 'space' AND project_id = '10002'",
            _ => "kind = 'space' AND project_id = '10003'",
        },
    )
    .await
}

async fn epics(db: &Db) -> i64 {
    id_where(db, "name = 'Epics' AND parent_id = (SELECT id FROM filter_list WHERE kind = 'space' AND project_id = '10001')").await
}

async fn views(db: &Db, list_id: i64) -> Vec<View> {
    views_of(db, list_id).await.unwrap()
}

async fn view_named(db: &Db, list_id: i64, name: &str) -> i64 {
    views(db, list_id)
        .await
        .into_iter()
        .find(|v| v.name == name)
        .unwrap_or_else(|| panic!("no view {name}"))
        .id
}

async fn default_view(db: &Db, list_id: i64) -> i64 {
    views(db, list_id)
        .await
        .into_iter()
        .find(|v| v.is_default)
        .unwrap()
        .id
}

async fn at_default(db: &Db, list_id: i64) -> Location {
    Location {
        list_id,
        view_id: default_view(db, list_id).await,
    }
}

fn place(nav: &Nav) -> Location {
    nav.location.expect("a place")
}

/// Three tracked Spaces in the sidebar order LUM, NW, NWS (not the order they were tracked in).
async fn lab() -> (TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    db.write(|tx, _| {
        tx.execute_batch(
            "INSERT INTO cache.space (id, key, name, raw) VALUES
               ('10001', 'NW', 'Website relaunch', '{}'),
               ('10002', 'NWS', 'Northwind support', '{}'),
               ('10003', 'LUM', 'Lumen iOS app', '{}')",
        )?;
        Ok(())
    })
    .await
    .unwrap();
    for id in [NW, NWS, LUM] {
        track_project(&db, id.into(), true).await.unwrap();
    }
    space_move(&db, LUM.into(), 0).await.unwrap();
    space_move(&db, NW.into(), 1).await.unwrap();
    space_move(&db, NWS.into(), 2).await.unwrap();
    (dir, db)
}

async fn bare_lab() -> (TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    (dir, db)
}

mod the_starting_place {
    use super::*;

    #[tokio::test]
    async fn a_fresh_store_opens_everything_at_its_default_view() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());

        let current = nav.current(&db).await.unwrap();

        let everything = everything(&db).await;
        assert_eq!(place(&current), at_default(&db, everything).await);
        assert_eq!(current.expanded, Vec::<i64>::new());
    }

    #[tokio::test]
    async fn the_place_serializes_for_the_page_in_camel_case() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        nav.set_expanded(&db, vec![7]).await.unwrap();

        let current = nav.current(&db).await.unwrap();

        let everything = everything(&db).await;
        assert_eq!(
            serde_json::to_value(&current).unwrap(),
            serde_json::json!({
                "location": {
                    "listId": everything,
                    "viewId": default_view(&db, everything).await,
                },
                "expanded": [7],
            })
        );
    }
}

mod going_to_a_level {
    use super::*;

    #[tokio::test]
    async fn a_level_opens_at_its_default_view() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;

        let current = nav.go(&db, nw, None).await.unwrap();

        assert_eq!(place(&current), at_default(&db, nw).await);
        assert_eq!(nav.current(&db).await.unwrap(), current);
    }

    #[tokio::test]
    async fn a_level_opens_at_the_requested_view() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        let table = view_named(&db, nw, "Table").await;

        let current = nav.go(&db, nw, Some(table)).await.unwrap();

        assert_eq!(
            place(&current),
            Location {
                list_id: nw,
                view_id: table
            }
        );
    }

    #[tokio::test]
    async fn a_filter_list_opens_at_its_own_view() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let epics = epics(&db).await;

        let current = nav.go(&db, epics, None).await.unwrap();

        assert_eq!(place(&current), at_default(&db, epics).await);
    }

    #[tokio::test]
    async fn a_view_of_another_level_is_ignored() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        let nws = space(&db, NWS).await;
        let foreign = view_named(&db, nws, "Board").await;

        let current = nav.go(&db, nw, Some(foreign)).await.unwrap();

        assert_eq!(place(&current), at_default(&db, nw).await);
    }

    #[tokio::test]
    async fn a_foreign_view_falls_back_to_the_levels_last_view() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        let nws = space(&db, NWS).await;
        let board = view_named(&db, nw, "Board").await;
        nav.go(&db, nw, Some(board)).await.unwrap();
        let foreign = view_named(&db, nws, "Table").await;

        let current = nav.go(&db, nw, Some(foreign)).await.unwrap();

        assert_eq!(
            place(&current),
            Location {
                list_id: nw,
                view_id: board
            }
        );
    }

    #[tokio::test]
    async fn an_unknown_list_is_not_found_and_the_place_stays() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        let before = nav.go(&db, nw, None).await.unwrap();

        let err = nav.go(&db, 9999, None).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(nav.current(&db).await.unwrap(), before);
    }
}

mod issues_read_here {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[tokio::test]
    async fn a_fresh_store_keeps_no_issue() {
        let (dir, _db) = lab().await;
        let nav = NavStore::open(dir.path());

        assert_eq!(nav.read_here_ids(), Vec::<String>::new());
    }

    #[tokio::test]
    async fn kept_issues_are_listed_once_in_order() {
        let (dir, _db) = lab().await;
        let nav = NavStore::open(dir.path());

        nav.keep_read(&ids(&["b", "a"]));
        nav.keep_read(&ids(&["b", "c"]));

        assert_eq!(nav.read_here_ids(), ids(&["a", "b", "c"]));
    }

    #[tokio::test]
    async fn going_to_the_same_place_keeps_them() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let recent = recent(&db).await;
        let at = nav.go(&db, recent, None).await.unwrap();
        nav.keep_read(&ids(&["a"]));

        nav.go(&db, recent, None).await.unwrap();
        nav.go(&db, recent, at.location.map(|l| l.view_id))
            .await
            .unwrap();

        assert_eq!(nav.read_here_ids(), ids(&["a"]));
    }

    #[tokio::test]
    async fn going_to_another_list_clears_them() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        nav.go(&db, recent(&db).await, None).await.unwrap();
        nav.keep_read(&ids(&["a"]));

        nav.go(&db, everything(&db).await, None).await.unwrap();

        assert_eq!(nav.read_here_ids(), Vec::<String>::new());
    }

    #[tokio::test]
    async fn going_to_another_view_of_the_same_list_clears_them() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        nav.go(&db, nw, Some(default_view(&db, nw).await))
            .await
            .unwrap();
        nav.keep_read(&ids(&["a"]));

        nav.go(&db, nw, Some(view_named(&db, nw, "Board").await))
            .await
            .unwrap();

        assert_eq!(nav.read_here_ids(), Vec::<String>::new());
    }

    #[tokio::test]
    async fn a_failed_go_keeps_them() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        nav.go(&db, recent(&db).await, None).await.unwrap();
        nav.keep_read(&ids(&["a"]));

        nav.go(&db, 9999, None).await.unwrap_err();

        assert_eq!(nav.read_here_ids(), ids(&["a"]));
    }
}

mod remembering_views {
    use super::*;

    #[tokio::test]
    async fn each_level_returns_to_the_view_it_showed_last() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let a = space(&db, NW).await;
        let b = space(&db, NWS).await;
        let board = view_named(&db, a, "Board").await;
        nav.go(&db, a, Some(board)).await.unwrap();
        nav.go(&db, b, None).await.unwrap();

        let back = nav.go(&db, a, None).await.unwrap();

        assert_eq!(
            place(&back),
            Location {
                list_id: a,
                view_id: board
            }
        );
        let other = nav.go(&db, b, None).await.unwrap();
        assert_eq!(place(&other), at_default(&db, b).await);
    }

    #[tokio::test]
    async fn the_place_and_the_last_views_survive_a_restart() {
        let (dir, db) = lab().await;
        let a = space(&db, NW).await;
        let b = space(&db, NWS).await;
        let board = view_named(&db, a, "Board").await;
        let table = view_named(&db, b, "Table").await;
        {
            let nav = NavStore::open(dir.path());
            nav.go(&db, a, Some(board)).await.unwrap();
            nav.go(&db, b, Some(table)).await.unwrap();
        }

        let reopened = NavStore::open(dir.path());

        assert_eq!(
            place(&reopened.current(&db).await.unwrap()),
            Location {
                list_id: b,
                view_id: table
            }
        );
        let back = reopened.go(&db, a, None).await.unwrap();
        assert_eq!(
            place(&back),
            Location {
                list_id: a,
                view_id: board
            }
        );
    }

    #[tokio::test]
    async fn the_expanded_spaces_survive_a_restart() {
        let (dir, db) = lab().await;
        let a = space(&db, NW).await;
        let b = space(&db, LUM).await;
        let nav = NavStore::open(dir.path());

        let set = nav.set_expanded(&db, vec![a, b]).await.unwrap();

        assert_eq!(set.expanded, [a, b]);
        let reopened = NavStore::open(dir.path());
        assert_eq!(reopened.current(&db).await.unwrap().expanded, [a, b]);
    }

    #[tokio::test]
    async fn setting_the_expanded_spaces_leaves_the_place_alone() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        let at = nav.go(&db, nw, None).await.unwrap();

        let set = nav.set_expanded(&db, vec![nw]).await.unwrap();

        assert_eq!(set.location, at.location);
    }
}

mod vanished_places {
    use super::*;

    #[tokio::test]
    async fn a_deleted_list_sends_the_place_back_to_everything() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let epics = epics(&db).await;
        nav.go(&db, epics, None).await.unwrap();

        list_delete(&db, epics).await.unwrap();

        let current = nav.current(&db).await.unwrap();
        let everything = everything(&db).await;
        assert_eq!(place(&current), at_default(&db, everything).await);
    }

    #[tokio::test]
    async fn a_deleted_view_falls_back_to_the_levels_default() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        let board = view_named(&db, nw, "Board").await;
        nav.go(&db, nw, Some(board)).await.unwrap();

        view_delete(&db, board).await.unwrap();

        let current = nav.current(&db).await.unwrap();
        assert_eq!(place(&current), at_default(&db, nw).await);
    }

    #[tokio::test]
    async fn going_to_a_level_after_its_remembered_view_was_deleted_opens_the_default() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        let nws = space(&db, NWS).await;
        let board = view_named(&db, nw, "Board").await;
        nav.go(&db, nw, Some(board)).await.unwrap();
        nav.go(&db, nws, None).await.unwrap();
        view_delete(&db, board).await.unwrap();

        let current = nav.go(&db, nw, None).await.unwrap();

        assert_eq!(place(&current), at_default(&db, nw).await);
    }
}

async fn list_in(db: &Db, space: i64, name: &'static str) -> i64 {
    id_where(db, format!("name = '{name}' AND parent_id = {space}")).await
}

async fn home_list(db: &Db, name: &str) -> i64 {
    let viewer = Viewer {
        me: None,
        time_zone: None,
        now: Utc::now(),
    };
    let everything = everything(db).await;
    list_create(db, viewer, everything, name.into(), None)
        .await
        .unwrap()
}

mod stepping_through_lists {
    use super::*;

    async fn walk(nav: &NavStore, db: &Db, step: i64, times: usize) -> Vec<i64> {
        let mut visited = Vec::new();
        for _ in 0..times {
            visited.push(place(&nav.step_list(db, step).await.unwrap()).list_id);
        }
        visited
    }

    async fn nw_section(db: &Db) -> [i64; 4] {
        let nw = space(db, NW).await;
        [
            nw,
            list_in(db, nw, "Current sprint").await,
            list_in(db, nw, "Epics").await,
            list_in(db, nw, "Open bugs").await,
        ]
    }

    #[tokio::test]
    async fn stepping_forward_in_a_space_visits_its_lists_in_order_and_wraps_to_the_space() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let [nw, sprint, epics, bugs] = nw_section(&db).await;
        nav.go(&db, nw, None).await.unwrap();

        let visited = walk(&nav, &db, 1, 5).await;

        assert_eq!(visited, [sprint, epics, bugs, nw, sprint]);
    }

    #[tokio::test]
    async fn stepping_backward_in_a_space_wraps_from_the_space_to_its_last_list() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let [nw, sprint, epics, bugs] = nw_section(&db).await;
        nav.go(&db, nw, None).await.unwrap();

        let visited = walk(&nav, &db, -1, 5).await;

        assert_eq!(visited, [bugs, epics, sprint, nw, bugs]);
    }

    #[tokio::test]
    async fn stepping_never_leaves_the_space() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let section = nw_section(&db).await;
        nav.go(&db, section[2], None).await.unwrap();

        for step in [1, -1] {
            for list_id in walk(&nav, &db, step, 9).await {
                assert!(section.contains(&list_id), "{step}: {list_id}");
            }
        }
    }

    #[tokio::test]
    async fn a_step_opens_the_list_at_the_view_it_showed_last() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let [nw, sprint, ..] = nw_section(&db).await;
        let table = view_named(&db, nw, "Table").await;
        nav.go(&db, nw, Some(table)).await.unwrap();
        nav.go(&db, sprint, None).await.unwrap();

        let current = nav.step_list(&db, -1).await.unwrap();

        assert_eq!(
            place(&current),
            Location {
                list_id: nw,
                view_id: table
            }
        );
    }

    #[tokio::test]
    async fn from_a_home_level_it_steps_among_the_levels_outside_the_spaces() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let everything = everything(&db).await;
        let recent = recent(&db).await;
        let bookmarks = id_where(&db, "kind = 'pinned'").await;
        let weekly = home_list(&db, "Weekly review").await;
        nav.go(&db, everything, None).await.unwrap();

        let forward = walk(&nav, &db, 1, 5).await;
        let backward = walk(&nav, &db, -1, 5).await;

        assert_eq!(forward, [recent, bookmarks, weekly, everything, recent]);
        assert_eq!(
            backward,
            [everything, weekly, bookmarks, recent, everything]
        );
    }

    #[tokio::test]
    async fn a_home_list_is_stepped_like_the_other_home_levels() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let weekly = home_list(&db, "Weekly review").await;
        nav.go(&db, weekly, None).await.unwrap();

        let next = walk(&nav, &db, 1, 1).await;

        assert_eq!(next, [everything(&db).await]);
    }

    #[tokio::test]
    async fn without_spaces_the_home_levels_still_step() {
        let (dir, db) = bare_lab().await;
        let nav = NavStore::open(dir.path());

        let visited = walk(&nav, &db, 1, 1).await;

        assert_eq!(visited, [recent(&db).await]);
    }
}

mod moving_notifies {
    use super::*;

    fn counted(nav: &NavStore) -> Arc<AtomicUsize> {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        nav.on_move(Box::new(move || {
            seen.fetch_add(1, Ordering::SeqCst);
        }));
        count
    }

    #[tokio::test]
    async fn going_to_another_level_notifies_once() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        nav.go(&db, everything(&db).await, None).await.unwrap();
        let count = counted(&nav);

        nav.go(&db, nw, None).await.unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn going_to_another_view_of_the_same_level_notifies() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        nav.go(&db, nw, Some(default_view(&db, nw).await))
            .await
            .unwrap();
        let count = counted(&nav);

        nav.go(&db, nw, Some(view_named(&db, nw, "Board").await))
            .await
            .unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn going_to_the_current_place_does_not_notify() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        let at = nav.go(&db, nw, None).await.unwrap();
        let count = counted(&nav);

        nav.go(&db, nw, None).await.unwrap();
        nav.go(&db, nw, at.location.map(|l| l.view_id))
            .await
            .unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_failed_go_does_not_notify() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        nav.go(&db, space(&db, NW).await, None).await.unwrap();
        let count = counted(&nav);

        nav.go(&db, 9999, None).await.unwrap_err();

        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn stepping_to_another_level_notifies() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        nav.go(&db, space(&db, NW).await, None).await.unwrap();
        let count = counted(&nav);

        nav.step_list(&db, 1).await.unwrap();
        nav.step_space(&db, 1).await.unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_clone_of_the_store_notifies_the_same_listener() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let count = counted(&nav);

        nav.clone()
            .go(&db, space(&db, NW).await, None)
            .await
            .unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}

mod stepping_through_spaces {
    use super::*;

    async fn stepped(nav: &NavStore, db: &Db, step: i64) -> i64 {
        place(&nav.step_space(db, step).await.unwrap()).list_id
    }

    #[tokio::test]
    async fn stepping_forward_follows_the_sidebar_order_and_wraps() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let order = [
            space(&db, LUM).await,
            space(&db, NW).await,
            space(&db, NWS).await,
        ];
        nav.go(&db, order[0], None).await.unwrap();

        let mut visited = Vec::new();
        for _ in 0..3 {
            visited.push(stepped(&nav, &db, 1).await);
        }

        assert_eq!(visited, [order[1], order[2], order[0]]);
    }

    #[tokio::test]
    async fn stepping_backward_follows_the_sidebar_order_and_wraps() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let order = [
            space(&db, LUM).await,
            space(&db, NW).await,
            space(&db, NWS).await,
        ];
        nav.go(&db, order[0], None).await.unwrap();

        let mut visited = Vec::new();
        for _ in 0..3 {
            visited.push(stepped(&nav, &db, -1).await);
        }

        assert_eq!(visited, [order[2], order[1], order[0]]);
    }

    #[tokio::test]
    async fn from_a_filter_list_it_steps_relative_to_its_space() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let nw = space(&db, NW).await;
        let epics = epics(&db).await;
        nav.go(&db, epics, None).await.unwrap();

        let next = stepped(&nav, &db, 1).await;
        nav.go(&db, epics, None).await.unwrap();
        let previous = stepped(&nav, &db, -1).await;

        assert_eq!(next, space(&db, NWS).await);
        assert_eq!(previous, space(&db, LUM).await);
        assert_ne!(next, nw);
    }

    #[tokio::test]
    async fn from_everything_or_home_forward_is_the_first_space_and_backward_the_last() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let first = space(&db, LUM).await;
        let last = space(&db, NWS).await;
        let outside = [everything(&db).await, recent(&db).await];

        for from in outside {
            nav.go(&db, from, None).await.unwrap();
            assert_eq!(stepped(&nav, &db, 1).await, first, "forward from {from}");
            nav.go(&db, from, None).await.unwrap();
            assert_eq!(stepped(&nav, &db, -1).await, last, "backward from {from}");
        }
    }

    #[tokio::test]
    async fn a_step_opens_the_space_at_the_view_it_showed_last() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        let lum = space(&db, LUM).await;
        let nw = space(&db, NW).await;
        let table = view_named(&db, nw, "Table").await;
        nav.go(&db, nw, Some(table)).await.unwrap();
        nav.go(&db, lum, None).await.unwrap();

        let current = nav.step_space(&db, 1).await.unwrap();

        assert_eq!(
            place(&current),
            Location {
                list_id: nw,
                view_id: table
            }
        );
    }

    #[tokio::test]
    async fn untracked_spaces_are_skipped() {
        let (dir, db) = lab().await;
        let nav = NavStore::open(dir.path());
        track_project(&db, NW.into(), false).await.unwrap();
        let lum = space(&db, LUM).await;
        let nws = space(&db, NWS).await;
        nav.go(&db, lum, None).await.unwrap();

        let first = stepped(&nav, &db, 1).await;
        let second = stepped(&nav, &db, 1).await;

        assert_eq!((first, second), (nws, lum));
    }

    #[tokio::test]
    async fn with_no_tracked_space_the_place_stays() {
        let (dir, db) = bare_lab().await;
        let nav = NavStore::open(dir.path());
        let before = nav.current(&db).await.unwrap();

        let forward = nav.step_space(&db, 1).await.unwrap();
        let backward = nav.step_space(&db, -1).await.unwrap();

        assert_eq!(forward, before);
        assert_eq!(backward, before);
        assert_eq!(place(&before), at_default(&db, everything(&db).await).await);
    }
}
