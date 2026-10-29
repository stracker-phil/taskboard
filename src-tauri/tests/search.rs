//! Full-text search on the demo site synced from the in-process fake Jira into a temp database: what the
//! command palette finds and ranks, and the `text` filter field that shows the same issues in a list.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use tempfile::TempDir;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, fetch_versions, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::error::ErrorKind;
use taskboard_lib::filter::{Condition, Field, Filter, Op, Value};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::{Viewer, list_create, list_match_count, list_set_filter};
use taskboard_lib::search::{MatchedIn, PaletteResults, TextPart, palette_query};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::{Emit, SyncContext, full_sync};
use taskboard_lib::test_support::demo::{DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::{Narrowing, view_rows, views_of};

const ME_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const NW: &str = "10001";
const NWS: &str = "10002";
const INT: &str = "10005";

fn viewer() -> Viewer {
    Viewer {
        me: Some(ME_ACCOUNT_ID.into()),
        time_zone: Some("Europe/Berlin".into()),
        now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
    }
}

fn text_contains(text: &str) -> Filter {
    Filter {
        conditions: vec![Condition::new(
            Field::Text,
            Op::Contains,
            Some(Value::One(text.into())),
        )],
    }
}

fn plain(text: &str) -> TextPart {
    TextPart {
        text: text.into(),
        strong: false,
    }
}

fn strong(text: &str) -> TextPart {
    TextPart {
        text: text.into(),
        strong: true,
    }
}

fn keys(results: &PaletteResults) -> Vec<&str> {
    results.issues.iter().map(|hit| hit.key.as_str()).collect()
}

fn sorted(mut keys: Vec<String>) -> Vec<String> {
    keys.sort();
    keys
}

struct Site {
    _fake: FakeJira,
    _dir: TempDir,
    ctx: SyncContext,
}

impl Site {
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
            NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(),
        )
        .await
        .unwrap();
        let client = fake.client(TokenKind::Classic);
        refresh_projects(&ctx.db, &client).await.unwrap();
        for id in PROJECT_IDS {
            fetch_versions(&ctx.db, &client, id.into()).await.unwrap();
            track_project(&ctx.db, id.into(), true).await.unwrap();
        }
        let emit: Emit = Arc::new(|_| {});
        full_sync(&ctx, &*emit).await.unwrap();
        Self {
            _fake: fake,
            _dir: dir,
            ctx,
        }
    }

    fn db(&self) -> &Db {
        &self.ctx.db
    }

    async fn palette(&self, text: &str) -> PaletteResults {
        palette_query(self.db(), text.to_string())
            .await
            .unwrap_or_else(|e| panic!("{text:?}: {e}"))
    }

    async fn execute(&self, sql: &'static str) {
        self.db()
            .write(move |tx, _| {
                tx.execute_batch(sql)?;
                Ok(())
            })
            .await
            .unwrap();
    }

    async fn space_id(&self, project_id: &str) -> i64 {
        let project_id = project_id.to_string();
        self.db()
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT id FROM filter_list WHERE kind = 'space' AND project_id = ?1",
                    [&project_id],
                    |row| row.get::<_, i64>(0),
                )?)
            })
            .await
            .unwrap()
    }

    /// A filter list under the Website relaunch Space, and its first view.
    async fn text_list(&self, filter: Filter) -> (i64, i64) {
        let space = self.space_id(NW).await;
        let list = list_create(self.db(), viewer(), space, "Search".into(), Some(filter))
            .await
            .unwrap();
        let view = views_of(self.db(), list).await.unwrap()[0].id;
        (list, view)
    }

    async fn list_keys(&self, list: i64, view: i64) -> Vec<String> {
        let data = view_rows(self.db(), viewer(), list, view, Narrowing::default())
            .await
            .unwrap();
        sorted(data.rows.values().map(|row| row.key.clone()).collect())
    }

    async fn match_total(&self, filter: Filter) -> i64 {
        let space = self.space_id(NW).await;
        list_match_count(self.db(), viewer(), space, filter, false)
            .await
            .unwrap()
            .total
    }
}

mod issues {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_exact_key_ranks_first_in_any_case() {
        let site = Site::new().await;

        for text in ["NW-1042", "nw-1042", "  Nw-1042 "] {
            let results = site.palette(text).await;

            let first = &results.issues[0];
            assert_eq!(first.key, "NW-1042", "{text:?}");
            assert_eq!(first.matched_in, MatchedIn::Key, "{text:?}");
            assert_eq!(
                first.summary,
                [plain("Checkout: address validation for DACH postcodes")],
                "{text:?}"
            );
            assert_eq!(first.project_id, NW, "{text:?}");
            assert_eq!(first.status_name, "In Progress", "{text:?}");
            assert!(!first.gone, "{text:?}");
            let mut ids: Vec<&str> = results.issues.iter().map(|h| h.id.as_str()).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), results.issues.len(), "{text:?}");
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_bare_number_finds_the_issue_with_that_number_and_its_project() {
        let site = Site::new().await;

        for (text, key, project_id) in [("1042", "NW-1042", NW), ("47", "INT-47", INT)] {
            let results = site.palette(text).await;

            let first = &results.issues[0];
            assert_eq!(first.key, key, "{text}");
            assert_eq!(first.matched_in, MatchedIn::Key, "{text}");
            assert_eq!(first.project_id, project_id, "{text}");
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_prefix_finds_summaries_and_marks_the_matched_word_strong() {
        let site = Site::new().await;

        let results = site.palette("checko").await;

        let mut found = keys(&results);
        found.sort();
        assert_eq!(
            found,
            [
                "NW-1020", "NW-1021", "NW-1024", "NW-1042", "NW-1051", "NW-977"
            ]
        );
        assert!(
            results
                .issues
                .iter()
                .all(|hit| hit.matched_in == MatchedIn::Summary)
        );
        let layout = results
            .issues
            .iter()
            .find(|hit| hit.key == "NW-1021")
            .unwrap();
        assert_eq!(
            layout.summary,
            [plain("One-page "), strong("checkout"), plain(" layout")]
        );
        let epic = results
            .issues
            .iter()
            .find(|hit| hit.key == "NW-1020")
            .unwrap();
        assert_eq!(epic.summary, [strong("Checkout")]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn every_word_has_to_match_somewhere_in_the_issue() {
        let site = Site::new().await;

        let results = site.palette("checkout totals").await;

        let mut found = keys(&results);
        found.sort();
        assert_eq!(found, ["NW-1020", "NW-977"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_summary_hit_ranks_above_a_description_only_hit() {
        let site = Site::new().await;

        let results = site.palette("guest").await;

        assert_eq!(keys(&results), ["NW-1051", "NW-1042"]);
        assert_eq!(results.issues[0].matched_in, MatchedIn::Summary);
        assert_eq!(
            results.issues[0].summary,
            [
                strong("Guest"),
                plain(" checkout: remember shipping country")
            ]
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_description_only_match_has_no_comment_snippet() {
        let site = Site::new().await;

        let results = site.palette("guest").await;

        let hit = results.issues.iter().find(|h| h.key == "NW-1042").unwrap();
        assert_eq!(hit.matched_in, MatchedIn::Description);
        assert_eq!(hit.comment, None);
        assert_eq!(
            hit.summary,
            [plain("Checkout: address validation for DACH postcodes")]
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_comment_only_match_says_so_and_shows_the_words_around_it() {
        let site = Site::new().await;

        let results = site.palette("legal").await;

        assert_eq!(keys(&results), ["NW-1033"]);
        let hit = &results.issues[0];
        assert_eq!(hit.matched_in, MatchedIn::Comment);
        assert_eq!(hit.summary, [plain("Klarna invoice for B2B customers")]);
        let snippet = hit.comment.as_ref().expect("a comment snippet");
        assert!(snippet.contains(&strong("legal")), "{snippet:?}");
        assert!(snippet.iter().any(|part| !part.strong), "{snippet:?}");
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_summary_match_has_no_comment_snippet() {
        let site = Site::new().await;

        let results = site.palette("klarna").await;

        assert_eq!(results.issues[0].key, "NW-1033");
        assert_eq!(results.issues[0].matched_in, MatchedIn::Summary);
        assert_eq!(results.issues[0].comment, None);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn text_without_a_match_finds_nothing() {
        let site = Site::new().await;

        for text in ["", "   ", "zzzzqx", "checkout zzzzqx"] {
            let results = site.palette(text).await;

            assert_eq!(results.issues, vec![], "{text:?}");
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn diacritics_and_case_are_folded() {
        let site = Site::new().await;
        site.execute(
            "UPDATE cache.issue_fts SET summary = 'Zählung der Warenkörbe'
             WHERE rowid = (SELECT CAST(id AS INTEGER) FROM cache.issue WHERE key = 'NW-1056')",
        )
        .await;

        for text in ["zahlung", "ZÄHLUNG", "zählung"] {
            let results = site.palette(text).await;

            assert_eq!(keys(&results), ["NW-1056"], "{text}");
            assert_eq!(
                results.issues[0].summary,
                [strong("Zählung"), plain(" der Warenkörbe")],
                "{text}"
            );
        }
        assert_eq!(keys(&site.palette("warenkorbe").await), ["NW-1056"]);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn hostile_input_never_fails() {
        let site = Site::new().await;

        for text in [
            "\"",
            "\"\"\"",
            "*",
            ")",
            "(",
            "NEAR(",
            "NEAR(a b)",
            "AND OR",
            "NOT",
            "a OR",
            "-checkout",
            "^checkout",
            "summary:checkout",
            "check*out",
            "x' OR 1=1 --",
            "NW-",
            "-12",
        ] {
            site.palette(text).await;
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn at_most_eight_issues_are_listed() {
        let site = Site::new().await;

        let results = site.palette("e").await;

        assert_eq!(results.issues.len(), 8);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn issues_of_untracked_projects_are_left_out() {
        let site = Site::new().await;
        assert_eq!(keys(&site.palette("wildcard").await), ["INT-42"]);

        track_project(site.db(), INT.into(), false).await.unwrap();

        for text in ["wildcard", "INT-42", "42", "int-47"] {
            let results = site.palette(text).await;

            assert_eq!(results.issues, vec![], "{text}");
        }
        assert_eq!(keys(&site.palette("NW-1042").await)[0], "NW-1042");
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn issues_gone_from_jira_are_still_found_and_marked() {
        let site = Site::new().await;
        site.execute("UPDATE cache.issue SET gone = 1 WHERE key = 'NW-1051'")
            .await;

        for text in ["NW-1051", "remember shipping"] {
            let results = site.palette(text).await;

            let hit = &results.issues[0];
            assert_eq!(hit.key, "NW-1051", "{text}");
            assert!(hit.gone, "{text}");
        }
        let results = site.palette("guest").await;
        let gone: Vec<(&str, bool)> = results
            .issues
            .iter()
            .map(|hit| (hit.key.as_str(), hit.gone))
            .collect();
        assert_eq!(gone, [("NW-1051", true), ("NW-1042", false)]);
    }
}

mod lists {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_space_is_found_by_part_of_its_name_ignoring_case_and_accents() {
        let site = Site::new().await;

        for text in ["website", "WEBSITE", "wébsite", "  launch ", "Site Re"] {
            let results = site.palette(text).await;

            let names: Vec<(&str, bool, Option<&str>)> = results
                .lists
                .iter()
                .map(|l| (l.name.as_str(), l.is_space, l.project_id.as_deref()))
                .collect();
            assert_eq!(names, [("Website relaunch", true, Some(NW))], "{text:?}");
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_filter_list_is_found_with_the_project_of_its_space() {
        let site = Site::new().await;

        let results = site.palette("open bugs").await;

        assert!(!results.lists.is_empty());
        let mut projects: Vec<&str> = Vec::new();
        for list in &results.lists {
            assert_eq!(list.name, "Open bugs");
            assert!(!list.is_space);
            projects.push(list.project_id.as_deref().expect("a Space's project"));
        }
        projects.sort();
        projects.dedup();
        assert_eq!(projects.len(), results.lists.len());
        assert!(projects.contains(&NW));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn spaces_come_before_filter_lists() {
        let site = Site::new().await;

        let results = site.palette("e").await;

        assert!(results.lists[0].is_space);
        assert!(
            results
                .lists
                .windows(2)
                .all(|pair| pair[0].is_space || !pair[1].is_space),
            "{:?}",
            results.lists
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn at_most_six_lists_are_listed() {
        let site = Site::new().await;

        let results = site.palette("e").await;

        assert_eq!(results.lists.len(), 6);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn empty_text_lists_no_filter_lists() {
        let site = Site::new().await;

        for text in ["", "  \t "] {
            assert_eq!(site.palette(text).await.lists, vec![], "{text:?}");
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn untracked_spaces_and_their_lists_are_left_out() {
        let site = Site::new().await;
        assert!(
            site.palette("northwind")
                .await
                .lists
                .iter()
                .any(|l| l.is_space && l.project_id.as_deref() == Some(NWS))
        );

        track_project(site.db(), NWS.into(), false).await.unwrap();

        assert_eq!(site.palette("northwind").await.lists, vec![]);
        let open_bugs = site.palette("open bugs").await;
        assert!(!open_bugs.lists.is_empty());
        assert!(
            open_bugs
                .lists
                .iter()
                .all(|l| l.project_id.as_deref() != Some(NWS))
        );
    }
}

mod text_filter {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_text_list_shows_exactly_the_issues_the_palette_finds() {
        let site = Site::new().await;
        let (list, view) = site.text_list(text_contains("checkout")).await;

        for text in ["checkout", "checko", "guest", "legal", "checkout totals"] {
            list_set_filter(site.db(), viewer(), list, text_contains(text))
                .await
                .unwrap();

            let palette = site.palette(text).await;

            assert_eq!(
                site.list_keys(list, view).await,
                sorted(palette.issues.iter().map(|h| h.key.clone()).collect()),
                "{text}"
            );
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_text_list_with_no_match_is_empty() {
        let site = Site::new().await;

        let count = site.match_total(text_contains("zzzzqx")).await;

        assert_eq!(count, 0);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_empty_or_unsearchable_value_matches_everything() {
        let site = Site::new().await;
        let everything = site.match_total(Filter::default()).await;
        assert!(everything > 0);

        for text in ["", "   ", "* - \""] {
            assert_eq!(
                site.match_total(text_contains(text)).await,
                everything,
                "{text:?}"
            );
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_text_condition_narrows_like_any_other_condition() {
        let site = Site::new().await;
        let narrowed = Filter {
            conditions: vec![
                Condition::new(
                    Field::Text,
                    Op::Contains,
                    Some(Value::One("checkout".into())),
                ),
                Condition::is(Field::Type, "Bug"),
            ],
        };

        assert_eq!(site.match_total(text_contains("checkout")).await, 6);
        assert_eq!(site.match_total(narrowed).await, 1);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn only_contains_is_allowed_on_text() {
        let site = Site::new().await;
        let space = site.space_id(NW).await;

        for op in [
            Op::Is,
            Op::IsNot,
            Op::In,
            Op::NotIn,
            Op::IsEmpty,
            Op::IsNotEmpty,
            Op::Before,
            Op::After,
            Op::WithinLastDays,
            Op::WithinNextDays,
        ] {
            let filter = Filter {
                conditions: vec![Condition::new(
                    Field::Text,
                    op,
                    Some(Value::One("checkout".into())),
                )],
            };

            let err = list_match_count(site.db(), viewer(), space, filter.clone(), false)
                .await
                .unwrap_err();
            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{op:?}");

            let err = list_create(site.db(), viewer(), space, "Text".into(), Some(filter))
                .await
                .unwrap_err();
            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{op:?}");
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_text_value_of_the_wrong_shape_is_invalid_input() {
        let site = Site::new().await;
        let space = site.space_id(NW).await;

        for value in [
            None,
            Some(Value::Many(vec!["checkout".into()])),
            Some(Value::Days(3)),
        ] {
            let filter = Filter {
                conditions: vec![Condition::new(Field::Text, Op::Contains, value.clone())],
            };

            let err = list_match_count(site.db(), viewer(), space, filter, false)
                .await
                .unwrap_err();

            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{value:?}");
        }
    }
}
