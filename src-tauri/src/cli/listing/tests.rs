//! The list replies as the command prints them, through `reply::render`.
use serde_json::{Value, json};

use crate::cli::reply::render;
use crate::ipc::IpcMessage;

fn printed(message: &IpcMessage, reply: Value) -> String {
    render(message, &reply).unwrap().unwrap()
}

fn filters() -> IpcMessage {
    IpcMessage::Filters { space: None }
}

fn views() -> IpcMessage {
    IpcMessage::Views { level: None }
}

fn go() -> IpcMessage {
    IpcMessage::Go {
        level: "NW".into(),
        view: None,
    }
}

fn search() -> IpcMessage {
    IpcMessage::Search {
        text: "login".into(),
    }
}

fn space(id: i64, key: &str, name: &str, client: Option<&str>, open: i64, current: bool) -> Value {
    json!({
        "id": id, "key": key, "name": name, "client": client, "open": open, "current": current
    })
}

fn level(
    id: i64,
    kind: &str,
    name: &str,
    space: Option<&str>,
    open: Option<i64>,
    current: bool,
) -> Value {
    json!({
        "id": id, "kind": kind, "name": name, "space": space, "open": open, "current": current
    })
}

fn view(id: i64, name: &str, layout: &str, is_default: bool, current: bool) -> Value {
    json!({
        "id": id, "name": name, "layout": layout, "isDefault": is_default, "current": current
    })
}

fn issue(key: &str, status: &str, summary: &str, gone: bool) -> Value {
    json!({ "key": key, "status": status, "summary": summary, "gone": gone })
}

#[test]
fn spaces_align_ids_and_keys_mark_the_current_one_and_show_client_and_open_count() {
    let reply = json!([
        space(2, "NW", "Website relaunch", Some("Northwind Ltd"), 21, true),
        space(10, "NWS", "Northwind support", None, 6, false),
    ]);
    assert_eq!(
        printed(&IpcMessage::Spaces, reply),
        "*  2  NW   Website relaunch (Northwind Ltd) · 21 open\n  10  NWS  Northwind support · 6 open"
    );
}

#[test]
fn no_spaces_say_where_to_pick_them() {
    assert_eq!(
        printed(&IpcMessage::Spaces, json!([])),
        "No Spaces are tracked; pick projects in Settings → Projects."
    );
}

#[test]
fn filters_prefix_a_spaces_lists_with_its_key_and_leave_home_lists_plain() {
    let reply = json!([
        level(3, "recent", "Unread", None, Some(4), false),
        level(12, "list", "Open bugs", Some("NW"), Some(12), true),
        level(13, "list", "Broken", Some("NWS"), None, false),
    ]);
    assert_eq!(
        printed(&filters(), reply),
        "   3  Unread · 4 open\n\
         * 12  NW/Open bugs · 12 open\n\
         \u{20} 13  NWS/Broken · filter can't be used"
    );
}

#[test]
fn no_filter_lists_say_so() {
    assert_eq!(printed(&filters(), json!([])), "No filter lists.");
}

#[test]
fn views_start_with_their_level_and_mark_the_current_and_default_ones() {
    let reply = json!({
        "level": "NW Website relaunch",
        "views": [
            view(5, "List", "list", true, true),
            view(6, "Board", "board", false, false),
            view(10, "Sprint table", "table", false, false),
        ]
    });
    assert_eq!(
        printed(&views(), reply),
        "NW Website relaunch:\n\
         *  5  List (list, default)\n\
         \u{20}  6  Board (board)\n\
         \u{20} 10  Sprint table (table)"
    );
}

#[test]
fn a_level_without_views_prints_only_its_header() {
    let reply = json!({ "level": "Pinned", "views": [] });
    assert_eq!(printed(&views(), reply), "Pinned:");
}

#[test]
fn go_prints_where_the_window_went() {
    let reply =
        json!({ "listId": 2, "level": "NW Website relaunch", "viewId": 6, "view": "Board" });
    assert_eq!(printed(&go(), reply), "NW Website relaunch › Board");
}

#[test]
fn go_prints_nothing_when_the_app_reports_no_place() {
    assert_eq!(render(&go(), &json!(null)), Ok(None));
}

#[test]
fn search_pads_keys_and_statuses_into_columns_and_flags_gone_issues() {
    let reply = json!([
        issue("NW-12", "In Progress", "Fix login page", false),
        issue("NW-3", "Prêt", "Old banner", true),
    ]);
    assert_eq!(
        printed(&search(), reply),
        "NW-12  In Progress  Fix login page\n\
         NW-3   Prêt         Old banner (gone)"
    );
}

#[test]
fn search_without_a_match_says_so() {
    assert_eq!(printed(&search(), json!([])), "No cached issue matches.");
}

#[test]
fn reset_cache_prints_the_apps_text_as_it_is() {
    assert_eq!(
        printed(
            &IpcMessage::ResetCache,
            json!("Emptied the cache; fetching again")
        ),
        "Emptied the cache; fetching again"
    );
}

#[test]
fn a_list_reply_of_the_wrong_shape_is_an_error() {
    for message in [IpcMessage::Spaces, filters(), views(), go(), search()] {
        let error = render(&message, &json!("nope")).unwrap_err();
        assert!(
            error.starts_with("The running Taskboard sent an unexpected reply"),
            "{message:?}: {error}"
        );
    }
}
