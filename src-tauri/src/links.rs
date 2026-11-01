//! Taskboard's own links (`taskboard://…`, `taskboard-debug://…` from debug builds): what each one asks for,
//! and the links the app hands out. Opening them is `remote::links`.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use serde::Deserialize;
use ts_rs::TS;
use url::Url;

use crate::LINK_SCHEME;

/// What a link asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Link {
    /// `issue/NW-12`
    Issue { key: String },
    /// `go/<level>[/<view>]`: a level and view as the command line names them (`remote::levels`).
    Go { level: String, view: Option<String> },
    /// `search?q=<text>`: the palette with the text typed in.
    Search { text: String },
    /// `command/<id>`: only a command `linkable` allows.
    Command { id: String },
}

/// Commands a link may run: none changes data in Jira or on the Mac, since any web page can open a link.
const LINKABLE_COMMANDS: &[&str] = &["sync:refresh", "app:show", "app:settings"];
/// Families a link may run with any argument (a Settings page).
const LINKABLE_FAMILIES: &[&str] = &["app:settings:"];

/// What a link of this build asks for, or why it is no link the app follows.
pub fn parse(link: &str) -> Result<Link, String> {
    let url = Url::parse(link).map_err(|_| format!("“{link}” isn't a link."))?;
    if url.scheme() != LINK_SCHEME {
        return Err(format!("“{link}” isn't a {LINK_SCHEME}:// link."));
    }
    let parts: Vec<String> = url
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .map(|part| percent_decode_str(part).decode_utf8_lossy().into_owned())
        .collect();
    let unknown = || format!("Taskboard doesn't know the link “{link}”.");
    match (url.host_str().unwrap_or_default(), parts.as_slice()) {
        ("issue", [key]) if is_issue_key(key) => Ok(Link::Issue {
            key: key.to_uppercase(),
        }),
        ("go", [level]) => Ok(Link::Go {
            level: level.clone(),
            view: None,
        }),
        ("go", [level, view]) => Ok(Link::Go {
            level: level.clone(),
            view: Some(view.clone()),
        }),
        ("search", []) => {
            let text = url
                .query_pairs()
                .find(|(name, _)| name == "q")
                .map(|(_, text)| text.trim().to_string())
                .unwrap_or_default();
            Ok(Link::Search { text })
        }
        ("command", [id]) if linkable(id) => Ok(Link::Command { id: id.clone() }),
        ("command", [id]) => Err(format!("A link can't run “{id}”.")),
        _ => Err(unknown()),
    }
}

fn is_issue_key(text: &str) -> bool {
    let Some((project, number)) = text.split_once('-') else {
        return false;
    };
    project.starts_with(|c: char| c.is_ascii_alphabetic())
        && project
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
}

fn linkable(id: &str) -> bool {
    LINKABLE_COMMANDS.contains(&id)
        || LINKABLE_FAMILIES
            .iter()
            .any(|family| id.len() > family.len() && id.starts_with(family))
}

/// What the app hands out a link to.
#[derive(Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LinkTarget {
    Issue {
        key: String,
    },
    /// A sidebar level, at one of its views or at the last one shown. Ids, so renaming keeps the link working.
    Level {
        #[serde(rename = "listId")]
        #[ts(type = "number")]
        list_id: i64,
        #[serde(rename = "viewId")]
        #[ts(type = "number | null")]
        view_id: Option<i64>,
    },
}

pub fn link_to(target: &LinkTarget) -> String {
    match target {
        LinkTarget::Issue { key } => {
            format!("{LINK_SCHEME}://issue/{}", encode(&key.to_uppercase()))
        }
        LinkTarget::Level {
            list_id,
            view_id: None,
        } => format!("{LINK_SCHEME}://go/{list_id}"),
        LinkTarget::Level {
            list_id,
            view_id: Some(view_id),
        } => format!("{LINK_SCHEME}://go/{list_id}/{view_id}"),
    }
}

/// Keeps an issue key's `-` and `_` readable.
const PATH_PART: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_');

fn encode(part: &str) -> String {
    utf8_percent_encode(part, PATH_PART).to_string()
}

/// The link to an issue or a sidebar level, for "Copy Taskboard Link".
#[tauri::command]
pub fn app_link(target: LinkTarget) -> String {
    link_to(&target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(rest: &str) -> String {
        format!("{LINK_SCHEME}://{rest}")
    }

    fn go(level: &str, view: Option<&str>) -> Link {
        Link::Go {
            level: level.to_string(),
            view: view.map(str::to_string),
        }
    }

    fn unknown(link: &str) -> String {
        format!("Taskboard doesn't know the link “{link}”.")
    }

    #[test]
    fn an_issue_link_names_the_issue() {
        assert_eq!(
            parse(&link("issue/NW-12")),
            Ok(Link::Issue {
                key: "NW-12".into()
            })
        );
    }

    #[test]
    fn a_lowercase_issue_key_is_uppercased() {
        assert_eq!(
            parse(&link("issue/nw-12")),
            Ok(Link::Issue {
                key: "NW-12".into()
            })
        );
    }

    #[test]
    fn a_malformed_issue_key_is_an_error_naming_the_link() {
        for rest in ["issue/12", "issue/NW-", "issue/NW-1x", "issue/"] {
            let text = link(rest);
            assert_eq!(parse(&text), Err(unknown(&text)), "{rest}");
        }
    }

    #[test]
    fn a_go_link_with_one_segment_names_only_the_level() {
        assert_eq!(parse(&link("go/NW")), Ok(go("NW", None)));
    }

    #[test]
    fn a_go_link_with_two_segments_names_level_and_view() {
        assert_eq!(parse(&link("go/4/9")), Ok(go("4", Some("9"))));
    }

    #[test]
    fn go_names_are_percent_decoded() {
        assert_eq!(
            parse(&link("go/NW%2FOpen%20bugs")),
            Ok(go("NW/Open bugs", None))
        );
        assert_eq!(
            parse(&link("go/NW/All%20issues")),
            Ok(go("NW", Some("All issues")))
        );
    }

    #[test]
    fn a_go_link_with_three_segments_is_an_error() {
        let text = link("go/NW/Open/Extra");
        assert_eq!(parse(&text), Err(unknown(&text)));
    }

    #[test]
    fn a_search_link_carries_the_decoded_text() {
        assert_eq!(
            parse(&link("search?q=checkout%20bug")),
            Ok(Link::Search {
                text: "checkout bug".into()
            })
        );
    }

    #[test]
    fn search_text_is_trimmed() {
        assert_eq!(
            parse(&link("search?q=%20%20checkout%20")),
            Ok(Link::Search {
                text: "checkout".into()
            })
        );
    }

    #[test]
    fn a_search_link_without_text_searches_for_nothing() {
        for rest in ["search", "search?x=1"] {
            assert_eq!(
                parse(&link(rest)),
                Ok(Link::Search {
                    text: String::new()
                }),
                "{rest}"
            );
        }
    }

    #[test]
    fn allowlisted_commands_may_run() {
        for id in [
            "sync:refresh",
            "app:show",
            "app:settings",
            "app:settings:general",
        ] {
            assert_eq!(
                parse(&link(&format!("command/{id}"))),
                Ok(Link::Command { id: id.into() }),
                "{id}"
            );
        }
    }

    #[test]
    fn other_commands_are_refused_naming_the_id() {
        for id in ["app:quit", "app:compactCache", "app:settings:"] {
            assert_eq!(
                parse(&link(&format!("command/{id}"))),
                Err(format!("A link can't run “{id}”.")),
                "{id}"
            );
        }
    }

    #[test]
    fn a_link_of_another_scheme_is_refused() {
        for text in [
            "https://example.com/issue/NW-1".to_string(),
            "taskboard-other://issue/NW-1".to_string(),
        ] {
            assert_eq!(
                parse(&text),
                Err(format!("“{text}” isn't a {LINK_SCHEME}:// link.")),
                "{text}"
            );
        }
    }

    #[test]
    fn an_unknown_host_is_an_error_naming_the_link() {
        let text = link("nowhere/NW-1");
        assert_eq!(parse(&text), Err(unknown(&text)));
    }

    #[test]
    fn text_that_is_no_url_is_an_error() {
        assert_eq!(
            parse("not a link"),
            Err("“not a link” isn't a link.".to_string())
        );
    }

    #[test]
    fn an_issue_target_links_to_its_key() {
        let target = LinkTarget::Issue {
            key: "NW-12".into(),
        };
        assert_eq!(link_to(&target), link("issue/NW-12"));
    }

    #[test]
    fn a_level_target_links_to_its_id_and_optional_view() {
        let level = |view_id| LinkTarget::Level {
            list_id: 4,
            view_id,
        };
        assert_eq!(link_to(&level(None)), link("go/4"));
        assert_eq!(link_to(&level(Some(9))), link("go/4/9"));
    }

    #[test]
    fn every_link_handed_out_parses_back_to_its_target() {
        let cases = [
            (
                LinkTarget::Issue {
                    key: "nw-12".into(),
                },
                Link::Issue {
                    key: "NW-12".into(),
                },
            ),
            (
                LinkTarget::Level {
                    list_id: 4,
                    view_id: None,
                },
                go("4", None),
            ),
            (
                LinkTarget::Level {
                    list_id: 4,
                    view_id: Some(9),
                },
                go("4", Some("9")),
            ),
        ];
        for (target, expected) in cases {
            assert_eq!(parse(&link_to(&target)), Ok(expected), "{target:?}");
        }
    }

    #[test]
    fn the_app_link_command_returns_the_link_to_the_target() {
        let target = LinkTarget::Issue { key: "NW-3".into() };
        assert_eq!(app_link(target), link("issue/NW-3"));
    }

    #[test]
    fn a_link_target_reads_the_pages_camel_case_json() {
        let cases = [
            (
                serde_json::json!({"kind": "issue", "key": "NW-1"}),
                LinkTarget::Issue { key: "NW-1".into() },
            ),
            (
                serde_json::json!({"kind": "level", "listId": 4, "viewId": null}),
                LinkTarget::Level {
                    list_id: 4,
                    view_id: None,
                },
            ),
            (
                serde_json::json!({"kind": "level", "listId": 4, "viewId": 9}),
                LinkTarget::Level {
                    list_id: 4,
                    view_id: Some(9),
                },
            ),
        ];
        for (json, expected) in cases {
            let target: LinkTarget = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(target, expected, "{json}");
        }
    }
}
