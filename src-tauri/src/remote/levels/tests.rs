//! How the command line names sidebar levels and views.
use super::*;
use crate::lists::SpaceNode;
use crate::views::{GroupBy, Layout, Sort};

fn node(id: i64, kind: Kind, name: &str, count: Option<i64>) -> ListNode {
    ListNode {
        id,
        kind,
        name: name.into(),
        count,
        own_conditions: 0,
        description: String::new(),
        error: None,
        icon: None,
    }
}

fn space(
    list_id: i64,
    key: &str,
    name: &str,
    client: Option<&str>,
    open_count: i64,
    lists: Vec<ListNode>,
) -> SpaceNode {
    SpaceNode {
        list_id,
        project_id: format!("{}", list_id * 1000),
        key: key.into(),
        name: name.into(),
        project_name: name.into(),
        icon: None,
        color: None,
        client: client.map(String::from),
        open_count,
        lists,
    }
}

fn tree() -> SidebarTree {
    SidebarTree {
        everything: node(1, Kind::Everything, "Everything", Some(40)),
        home: vec![
            node(2, Kind::Recent, "Recently changed", Some(4)),
            node(3, Kind::Pinned, "Pinned", Some(2)),
            node(4, Kind::List, "Weekly review", Some(9)),
        ],
        pinned_lists: vec![],
        spaces: vec![
            space(
                10,
                "NW",
                "Website relaunch",
                Some("Northwind Ltd"),
                21,
                vec![
                    node(11, Kind::List, "Open bugs", Some(12)),
                    node(12, Kind::List, "Launch blockers", None),
                ],
            ),
            space(
                20,
                "NWS",
                "Northwind support",
                None,
                6,
                vec![
                    node(21, Kind::List, "Open bugs", Some(3)),
                    node(22, Kind::List, "Waiting on customer", Some(1)),
                ],
            ),
        ],
    }
}

fn view(id: i64, name: &str, layout: Layout, is_default: bool) -> View {
    View {
        id,
        list_id: 10,
        name: name.into(),
        layout,
        layouts: vec![Layout::List, Layout::Board, Layout::Table],
        columns: vec![],
        group_by: GroupBy::None,
        sort: Sort::default(),
        widths: Default::default(),
        wrap_titles: false,
        collapsed: Default::default(),
        open_with: None,
        position: id,
        is_default,
    }
}

fn views() -> Vec<View> {
    vec![
        view(5, "List", Layout::List, true),
        view(6, "Board", Layout::Board, false),
        view(7, "Burndown table", Layout::Table, false),
    ]
}

#[test]
fn a_level_is_found_by_its_keyword_name_or_key_in_any_case() {
    for (word, id) in [
        ("everything", 1),
        ("EVERYTHING", 1),
        ("recent", 2),
        ("Recently changed", 2),
        ("pinned", 3),
        ("Pinned", 3),
        ("NW", 10),
        ("nw", 10),
        ("nws", 20),
        ("Website relaunch", 10),
        ("northwind support", 20),
        ("Weekly review", 4),
        ("  pinned  ", 3),
    ] {
        assert_eq!(find_level(&tree(), word), Ok(id), "{word:?}");
    }
}

#[test]
fn a_filter_list_is_found_by_its_name_inside_its_space() {
    for (word, id) in [
        ("NW/Open bugs", 11),
        ("nws/open bugs", 21),
        ("NW/Launch blockers", 12),
    ] {
        assert_eq!(find_level(&tree(), word), Ok(id), "{word}");
    }
}

#[test]
fn a_filter_list_name_used_in_one_space_needs_no_key() {
    assert_eq!(find_level(&tree(), "Launch blockers"), Ok(12));
    assert_eq!(find_level(&tree(), "waiting on customer"), Ok(22));
}

#[test]
fn a_filter_list_name_used_in_two_spaces_is_ambiguous_and_names_both_with_their_ids() {
    assert_eq!(
        find_level(&tree(), "Open bugs"),
        Err(
            "“Open bugs” could be NW/Open bugs (11), NWS/Open bugs (21); name one by its id."
                .into()
        )
    );
}

#[test]
fn a_unique_start_of_a_name_is_enough() {
    for (word, id) in [
        ("launch", 12),
        ("Weekly", 4),
        ("web", 10),
        ("northwind", 20),
    ] {
        assert_eq!(find_level(&tree(), word), Ok(id), "{word}");
    }
}

#[test]
fn a_start_shared_by_several_levels_is_ambiguous() {
    assert_eq!(
        find_level(&tree(), "w"),
        Err(
            "“w” could be Weekly review (4), NW (10), NWS/Waiting on customer (22); \
             name one by its id."
                .into()
        )
    );
}

#[test]
fn an_exact_name_beats_a_longer_name_it_starts() {
    assert_eq!(find_level(&tree(), "NW"), Ok(10));
}

#[test]
fn a_level_is_found_by_its_numeric_id_at_any_depth() {
    for (word, id) in [("1", 1), ("4", 4), ("10", 10), ("12", 12), (" 21 ", 21)] {
        assert_eq!(find_level(&tree(), word), Ok(id), "{word:?}");
    }
}

#[test]
fn an_unknown_id_or_word_is_an_error_pointing_to_the_listings() {
    assert_eq!(
        find_level(&tree(), "99"),
        Err("No sidebar level has the id 99; see `taskboard filters`.".into())
    );
    assert_eq!(
        find_level(&tree(), "zzz"),
        Err("No Space or filter list is called “zzz”; see `taskboard spaces` and `taskboard filters`."
            .into())
    );
}

#[test]
fn a_view_is_found_by_id_or_by_name_in_any_case() {
    for (word, id) in [("6", 6), ("board", 6), ("LIST", 5), ("Burndown table", 7)] {
        assert_eq!(find_view(&views(), word), Ok(id), "{word}");
    }
}

#[test]
fn a_view_is_found_by_a_unique_start_of_its_name() {
    assert_eq!(find_view(&views(), "li"), Ok(5));
    assert_eq!(find_view(&views(), "Bo"), Ok(6));
}

#[test]
fn a_start_shared_by_two_views_is_ambiguous() {
    assert_eq!(
        find_view(&views(), "b"),
        Err("“b” could be Board (6), Burndown table (7); name one by its id.".into())
    );
}

#[test]
fn an_exact_view_name_beats_a_longer_name_it_starts() {
    let mut all = views();
    all.push(view(8, "Board archive", Layout::Board, false));
    assert_eq!(find_view(&all, "board"), Ok(6));
}

#[test]
fn an_unknown_view_is_an_error_pointing_to_the_listing() {
    for word in ["Gantt", "99"] {
        assert_eq!(
            find_view(&views(), word),
            Err(format!(
                "No view is called “{word}”; see `taskboard views`."
            )),
            "{word}"
        );
    }
}

#[test]
fn levels_are_labelled_the_way_messages_show_them() {
    for (id, label) in [
        (1, Some("Everything")),
        (2, Some("Recently changed")),
        (4, Some("Weekly review")),
        (10, Some("NW Website relaunch")),
        (11, Some("NW/Open bugs")),
        (22, Some("NWS/Waiting on customer")),
        (99, None),
    ] {
        assert_eq!(level_label(&tree(), id), label.map(String::from), "{id}");
    }
}

#[test]
fn space_rows_carry_each_spaces_fields_and_flag_the_current_one() {
    assert_eq!(
        space_rows(&tree(), Some(20)),
        vec![
            SpaceRow {
                id: 10,
                key: "NW".into(),
                name: "Website relaunch".into(),
                client: Some("Northwind Ltd".into()),
                open: 21,
                current: false,
            },
            SpaceRow {
                id: 20,
                key: "NWS".into(),
                name: "Northwind support".into(),
                client: None,
                open: 6,
                current: true,
            },
        ]
    );
}

#[test]
fn space_rows_flag_none_current_when_the_window_is_elsewhere() {
    assert!(space_rows(&tree(), None).iter().all(|row| !row.current));
    assert!(space_rows(&tree(), Some(11)).iter().all(|row| !row.current));
}

fn level_row(
    id: i64,
    kind: Kind,
    name: &str,
    space: Option<&str>,
    open: Option<i64>,
    current: bool,
) -> LevelRow {
    LevelRow {
        id,
        kind,
        name: name.into(),
        space: space.map(String::from),
        open,
        current,
    }
}

#[test]
fn level_rows_list_home_first_then_every_spaces_lists_under_their_key() {
    assert_eq!(
        level_rows(&tree(), None, Some(11)),
        vec![
            level_row(1, Kind::Everything, "Everything", None, Some(40), false),
            level_row(2, Kind::Recent, "Recently changed", None, Some(4), false),
            level_row(3, Kind::Pinned, "Pinned", None, Some(2), false),
            level_row(4, Kind::List, "Weekly review", None, Some(9), false),
            level_row(11, Kind::List, "Open bugs", Some("NW"), Some(12), true),
            level_row(12, Kind::List, "Launch blockers", Some("NW"), None, false),
            level_row(21, Kind::List, "Open bugs", Some("NWS"), Some(3), false),
            level_row(
                22,
                Kind::List,
                "Waiting on customer",
                Some("NWS"),
                Some(1),
                false
            ),
        ]
    );
}

#[test]
fn level_rows_for_one_space_leave_out_home_and_the_other_spaces() {
    assert_eq!(
        level_rows(&tree(), Some(20), None),
        vec![
            level_row(21, Kind::List, "Open bugs", Some("NWS"), Some(3), false),
            level_row(
                22,
                Kind::List,
                "Waiting on customer",
                Some("NWS"),
                Some(1),
                false
            ),
        ]
    );
}

#[test]
fn level_rows_for_an_unknown_space_are_empty() {
    assert_eq!(level_rows(&tree(), Some(99), None), vec![]);
}

#[test]
fn view_rows_carry_each_views_fields_and_flag_the_current_one() {
    assert_eq!(
        view_rows(&views(), Some(6)),
        vec![
            ViewRow {
                id: 5,
                name: "List".into(),
                layout: Layout::List,
                is_default: true,
                current: false,
            },
            ViewRow {
                id: 6,
                name: "Board".into(),
                layout: Layout::Board,
                is_default: false,
                current: true,
            },
            ViewRow {
                id: 7,
                name: "Burndown table".into(),
                layout: Layout::Table,
                is_default: false,
                current: false,
            },
        ]
    );
}
