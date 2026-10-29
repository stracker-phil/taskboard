//! The command line's parsing: which words become which request.
use super::*;

fn parse_args(args: &[&str]) -> CliOutcome {
    parse(std::iter::once("taskboard").chain(args.iter().copied()))
}

fn ask(message: IpcMessage, json: bool) -> CliOutcome {
    CliOutcome::Ask(CliRequest { message, json })
}

fn remote(message: IpcMessage, json: bool) -> CliOutcome {
    CliOutcome::Remote(CliRequest { message, json })
}

fn exit_code(outcome: CliOutcome) -> i32 {
    match outcome {
        CliOutcome::Exit { code, .. } => code,
        other => panic!("expected an exit, got {other:?}"),
    }
}

#[test]
fn no_arguments_start_the_app_visibly_with_the_default_log_level() {
    assert_eq!(
        parse_args(&[]),
        CliOutcome::Run {
            hidden: false,
            log_level: None
        }
    );
}

#[test]
fn hidden_starts_the_app_without_a_window() {
    assert_eq!(
        parse_args(&["--hidden"]),
        CliOutcome::Run {
            hidden: true,
            log_level: None
        }
    );
}

#[test]
fn every_log_level_is_accepted() {
    for (word, level) in [
        ("error", LogLevel::Error),
        ("warn", LogLevel::Warn),
        ("info", LogLevel::Info),
        ("debug", LogLevel::Debug),
        ("trace", LogLevel::Trace),
    ] {
        assert_eq!(
            parse_args(&["--log-level", word]),
            CliOutcome::Run {
                hidden: false,
                log_level: Some(level)
            },
            "{word}"
        );
    }
}

#[test]
fn hidden_and_log_level_combine() {
    assert_eq!(
        parse_args(&["--hidden", "--log-level", "debug"]),
        CliOutcome::Run {
            hidden: true,
            log_level: Some(LogLevel::Debug)
        }
    );
}

#[test]
fn an_unknown_log_level_is_a_usage_error() {
    assert_eq!(exit_code(parse_args(&["--log-level", "loud"])), 2);
}

#[test]
fn log_levels_map_to_log_filters() {
    for (level, filter) in [
        (LogLevel::Error, log::LevelFilter::Error),
        (LogLevel::Warn, log::LevelFilter::Warn),
        (LogLevel::Info, log::LevelFilter::Info),
        (LogLevel::Debug, log::LevelFilter::Debug),
        (LogLevel::Trace, log::LevelFilter::Trace),
    ] {
        assert_eq!(log::LevelFilter::from(level), filter, "{level:?}");
    }
    assert_eq!(LogLevel::default(), LogLevel::Info);
}

#[test]
fn version_prints_the_name_and_version_and_exits_cleanly() {
    match parse_args(&["--version"]) {
        CliOutcome::Exit { message, code } => {
            assert_eq!(code, 0);
            assert_eq!(
                message.trim_end(),
                format!("taskboard {}", env!("CARGO_PKG_VERSION"))
            );
        }
        other => panic!("expected an exit, got {other:?}"),
    }
}

#[test]
fn help_exits_cleanly_and_lists_the_subcommands() {
    match parse_args(&["--help"]) {
        CliOutcome::Exit { message, code } => {
            assert_eq!(code, 0);
            for word in [
                "status",
                "show",
                "focus",
                "hide",
                "open",
                "quit",
                "sync",
                "spaces",
                "filters",
                "views",
                "go",
                "me",
                "layout",
                "search",
                "reset-cache",
                "--hidden",
                "--json",
            ] {
                assert!(message.contains(word), "{word}");
            }
        }
        other => panic!("expected an exit, got {other:?}"),
    }
}

#[test]
fn help_ends_with_the_examples() {
    match parse_args(&["--help"]) {
        CliOutcome::Exit { message, .. } => {
            for line in EXAMPLES.lines() {
                assert!(message.contains(line.trim_end()), "{line}");
            }
        }
        other => panic!("expected an exit, got {other:?}"),
    }
}

#[test]
fn json_is_accepted_before_or_after_the_subcommand() {
    for args in [&["status", "--json"][..], &["--json", "status"]] {
        assert_eq!(parse_args(args), ask(IpcMessage::Status, true), "{args:?}");
    }
    assert_eq!(
        parse_args(&["spaces", "--json"]),
        ask(IpcMessage::Spaces, true)
    );
    assert_eq!(
        parse_args(&["search", "--json", "login"]),
        ask(
            IpcMessage::Search {
                text: "login".into()
            },
            true
        )
    );
    assert_eq!(
        parse_args(&["--json", "go", "NW"]),
        remote(
            IpcMessage::Go {
                level: "NW".into(),
                view: None
            },
            true
        )
    );
}

#[test]
fn only_show_focus_open_and_go_may_start_the_app() {
    let starting = [
        &["show"][..],
        &["focus"],
        &["open", "NW-12"],
        &["go", "NW"],
    ];
    for args in starting {
        assert!(
            matches!(parse_args(args), CliOutcome::Remote(_)),
            "{args:?}"
        );
    }
    let asking = [
        &["status"][..],
        &["hide"],
        &["quit"],
        &["spaces"],
        &["filters"],
        &["views"],
        &["me"],
        &["layout", "board"],
        &["search", "login"],
        &["sync"],
        &["reset-cache", "--yes"],
    ];
    for args in asking {
        assert!(matches!(parse_args(args), CliOutcome::Ask(_)), "{args:?}");
    }
}

#[test]
fn spaces_asks_for_the_tracked_spaces() {
    assert_eq!(parse_args(&["spaces"]), ask(IpcMessage::Spaces, false));
}

#[test]
fn filters_takes_an_optional_space() {
    assert_eq!(
        parse_args(&["filters"]),
        ask(IpcMessage::Filters { space: None }, false)
    );
    assert_eq!(
        parse_args(&["filters", "NW"]),
        ask(
            IpcMessage::Filters {
                space: Some("NW".into())
            },
            false
        )
    );
}

#[test]
fn views_takes_an_optional_level() {
    assert_eq!(
        parse_args(&["views"]),
        ask(IpcMessage::Views { level: None }, false)
    );
    assert_eq!(
        parse_args(&["views", "recent"]),
        ask(
            IpcMessage::Views {
                level: Some("recent".into())
            },
            false
        )
    );
}

#[test]
fn go_takes_a_level_and_an_optional_view_and_may_start_the_app() {
    assert_eq!(
        parse_args(&["go", "NW"]),
        remote(
            IpcMessage::Go {
                level: "NW".into(),
                view: None
            },
            false
        )
    );
    assert_eq!(
        parse_args(&["go", "NW/Open bugs", "Board"]),
        remote(
            IpcMessage::Go {
                level: "NW/Open bugs".into(),
                view: Some("Board".into())
            },
            false
        )
    );
}

#[test]
fn go_without_a_level_is_a_usage_error() {
    assert_eq!(exit_code(parse_args(&["go"])), 2);
}

#[test]
fn me_sets_or_flips_the_chip() {
    for (args, on) in [
        (&["me"][..], None),
        (&["me", "on"], Some(true)),
        (&["me", "ON"], Some(true)),
        (&["me", "off"], Some(false)),
        (&["me", "Off"], Some(false)),
    ] {
        assert_eq!(parse_args(args), ask(IpcMessage::Me { on }, false), "{args:?}");
    }
}

#[test]
fn me_with_anything_but_on_or_off_is_a_usage_error() {
    for word in ["maybe", "true", ""] {
        assert_eq!(exit_code(parse_args(&["me", word])), 2, "{word:?}");
    }
}

#[test]
fn layout_takes_list_board_or_table() {
    for (word, layout) in [
        ("list", Layout::List),
        ("board", Layout::Board),
        ("table", Layout::Table),
    ] {
        assert_eq!(
            parse_args(&["layout", word]),
            ask(IpcMessage::Layout { layout }, false),
            "{word}"
        );
    }
}

#[test]
fn layout_without_a_known_layout_is_a_usage_error() {
    for args in [&["layout"][..], &["layout", "grid"]] {
        assert_eq!(exit_code(parse_args(args)), 2, "{args:?}");
    }
}

#[test]
fn search_joins_its_words_with_a_space() {
    for (args, text) in [
        (&["search", "login"][..], "login"),
        (&["search", "login", "page"], "login page"),
        (&["search", "login page"], "login page"),
    ] {
        assert_eq!(
            parse_args(args),
            ask(IpcMessage::Search { text: text.into() }, false),
            "{args:?}"
        );
    }
}

#[test]
fn search_without_words_is_a_usage_error() {
    assert_eq!(exit_code(parse_args(&["search"])), 2);
}

#[test]
fn reset_cache_needs_the_yes_flag() {
    assert_eq!(
        parse_args(&["reset-cache", "--yes"]),
        ask(IpcMessage::ResetCache, false)
    );
    assert_eq!(exit_code(parse_args(&["reset-cache"])), 2);
}

#[test]
fn status_asks_the_running_app_in_text_or_json() {
    assert_eq!(parse_args(&["status"]), ask(IpcMessage::Status, false));
    assert_eq!(
        parse_args(&["status", "--json"]),
        ask(IpcMessage::Status, true)
    );
}

#[test]
fn show_does_not_ask_for_focus_and_may_start_the_app() {
    assert_eq!(
        parse_args(&["show"]),
        CliOutcome::Remote(CliRequest {
            message: IpcMessage::Show { focus: false },
            json: false
        })
    );
}

#[test]
fn focus_activates_the_visible_window_and_may_start_the_app() {
    assert_eq!(
        parse_args(&["focus"]),
        CliOutcome::Remote(CliRequest {
            message: IpcMessage::Activate { hidden: false },
            json: false
        })
    );
}

#[test]
fn hide_and_quit_ask_the_running_app() {
    assert_eq!(parse_args(&["hide"]), ask(IpcMessage::Hide, false));
    assert_eq!(parse_args(&["quit"]), ask(IpcMessage::Quit, false));
}

#[test]
fn sync_asks_the_running_app_for_a_sync_and_full_asks_for_the_whole_scope() {
    assert_eq!(
        parse_args(&["sync"]),
        ask(IpcMessage::Sync { full: false }, false)
    );
    assert_eq!(
        parse_args(&["sync", "--full"]),
        ask(IpcMessage::Sync { full: true }, false)
    );
}

#[test]
fn sync_rejects_unknown_flags() {
    assert_eq!(exit_code(parse_args(&["sync", "--fully"])), 2);
}

#[cfg(debug_assertions)]
#[test]
fn dev_drift_asks_the_running_app_to_change_the_demo_site() {
    assert_eq!(
        parse_args(&["dev", "drift"]),
        ask(IpcMessage::DevDrift, false)
    );
}

#[cfg(debug_assertions)]
#[test]
fn dev_seed_synthetic_asks_the_running_app_for_that_many_issues() {
    assert_eq!(
        parse_args(&["dev", "seed-synthetic", "5000"]),
        ask(IpcMessage::DevSeedSynthetic { count: 5000 }, false)
    );
}

#[cfg(debug_assertions)]
#[test]
fn dev_seed_synthetic_without_a_numeric_count_is_a_usage_error() {
    for args in [
        &["dev", "seed-synthetic"][..],
        &["dev", "seed-synthetic", "many"],
        &["dev", "seed-synthetic", "-5"],
    ] {
        assert_eq!(exit_code(parse_args(args)), 2, "{args:?}");
    }
}

#[cfg(debug_assertions)]
#[test]
fn dev_without_a_known_subcommand_is_a_usage_error() {
    for args in [&["dev"][..], &["dev", "explode"]] {
        assert_eq!(exit_code(parse_args(args)), 2, "{args:?}");
    }
}

#[test]
fn open_upper_cases_the_issue_key() {
    for (input, key) in [
        ("nw-12", "NW-12"),
        ("NW-12", "NW-12"),
        ("Ab_c2-7", "AB_C2-7"),
    ] {
        assert_eq!(
            parse_args(&["open", input]),
            remote(IpcMessage::Open { key: key.into() }, false),
            "{input}"
        );
    }
}

#[test]
fn open_rejects_malformed_issue_keys() {
    for input in [
        "12", "NW-", "NW-012", "-12", "N W-1", "NW-1a", "1W-5", "NW-0", "",
    ] {
        assert_eq!(exit_code(parse_args(&["open", input])), 2, "{input:?}");
    }
}

#[test]
fn open_without_a_key_is_a_usage_error() {
    assert_eq!(exit_code(parse_args(&["open"])), 2);
}

#[test]
fn unknown_subcommands_and_flags_are_usage_errors() {
    for args in [
        &["frobnicate"][..],
        &["--frobnicate"],
        &["status", "--frobnicate"],
    ] {
        assert_eq!(exit_code(parse_args(args)), 2, "{args:?}");
    }
}
