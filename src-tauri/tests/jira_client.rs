//! The Jira client against the loopback fake Jira: paging, error mapping, authentication, retries, the
//! typed answers on the demo site, and the fake's knobs.

use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use taskboard_lib::error::{AppError, ErrorKind};
use taskboard_lib::jira::dto::{Issue, parse_jira_time};
use taskboard_lib::jira::jql::{and_updated_since, date_literal, default_scope};
use taskboard_lib::jira::{JiraClient, Limiter, SearchRequest, TokenKind};
use taskboard_lib::test_support::FakeSecrets;
use taskboard_lib::test_support::demo::{self, CLOUD_ID, DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};

const SEARCH_PATH: &str = "/rest/api/3/search/jql";
const NW_1042_ID: &str = "101042";
const NW_1020_ID: &str = "101020";
const TRACKED: [&str; 5] = ["NW", "NWS", "LUM", "FBA", "INT"];

fn demo_now() -> DateTime<Utc> {
    DEMO_NOW.parse().unwrap()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn request(jql: &str, fields: &[&str]) -> SearchRequest {
    SearchRequest {
        jql: jql.into(),
        fields: strings(fields),
        expand: None,
    }
}

fn tracked_scope() -> String {
    default_scope(&strings(&TRACKED), 90).unwrap()
}

fn search_bodies(fake: &FakeJira) -> Vec<Value> {
    fake.requests()
        .into_iter()
        .filter(|r| r.path == SEARCH_PATH)
        .map(|r| r.body)
        .collect()
}

async fn all_issues(client: &JiraClient, jql: &str) -> Vec<Issue> {
    let mut pages = client.search_jql(request(jql, &["summary", "updated", "project"]));
    let mut issues = Vec::new();
    while let Some(page) = pages.next_page().await {
        issues.extend(page.unwrap().issues);
    }
    issues
}

async fn keys(client: &JiraClient, jql: &str) -> Vec<String> {
    all_issues(client, jql)
        .await
        .into_iter()
        .map(|issue| issue.key)
        .collect()
}

mod paging {
    use super::*;

    const INT_OPEN: &str = r#"project = "INT" AND statusCategory != Done"#;

    #[tokio::test]
    async fn search_pages_follow_the_next_page_token_until_the_last_page() {
        let fake = FakeJira::start_demo().await;
        fake.set_page_size(3);
        let client = fake.client(TokenKind::Classic);

        let mut pages = client.search_jql(request(INT_OPEN, &["summary", "status"]));
        let mut sizes = Vec::new();
        let mut tokens = Vec::new();
        while let Some(page) = pages.next_page().await {
            let page = page.unwrap();
            sizes.push(page.issues.len());
            tokens.push(page.next_page_token);
        }

        assert_eq!(sizes, [3, 3, 2]);
        assert_eq!(fake.paths(), [SEARCH_PATH; 3]);
        let bodies = search_bodies(&fake);
        assert!(bodies[0]["nextPageToken"].is_null());
        assert_eq!(bodies[1]["nextPageToken"], json!(tokens[0]));
        assert_eq!(bodies[2]["nextPageToken"], json!(tokens[1]));
        assert_eq!(tokens[2], None);
        for body in &bodies {
            assert_eq!(body["maxResults"], json!(100));
            assert_eq!(body["fields"], json!(["summary", "status"]));
            assert_eq!(body["jql"], json!(INT_OPEN));
        }
    }

    #[tokio::test]
    async fn the_cursor_ends_after_the_last_page() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let mut pages = client.search_jql(request(INT_OPEN, &["summary"]));

        assert_eq!(pages.next_page().await.unwrap().unwrap().issues.len(), 8);
        assert!(pages.next_page().await.is_none());
        assert!(pages.next_page().await.is_none());
        assert_eq!(fake.paths().len(), 1);
    }

    #[tokio::test]
    async fn a_failed_page_is_returned_once_and_then_the_cursor_ends() {
        let fake = FakeJira::start_demo().await;
        fake.set_page_size(3);
        let client = fake.client(TokenKind::Classic);
        let mut pages = client.search_jql(request(INT_OPEN, &["summary"]));

        assert_eq!(pages.next_page().await.unwrap().unwrap().issues.len(), 3);
        fake.fail_next(1, 500, None);
        let failure = pages.next_page().await.unwrap().unwrap_err();

        assert_eq!(failure.kind(), ErrorKind::Internal);
        assert!(pages.next_page().await.is_none());
        assert_eq!(fake.paths().len(), 2);
    }

    #[tokio::test]
    async fn a_search_asking_for_few_fields_gets_only_those_fields() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);

        let issues = all_issues_with(&client, request(r#"key = "NW-1042""#, &["summary"])).await;

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].key, "NW-1042");
        assert_eq!(issues[0].id, NW_1042_ID);
        assert_eq!(
            issues[0].fields,
            json!({ "summary": "Checkout: address validation for DACH postcodes" })
        );
        assert!(issues[0].rendered_fields.is_null());
    }

    #[tokio::test]
    async fn expanding_rendered_fields_adds_them_to_the_issue() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let mut asked = request(r#"key = "NW-1042""#, &["summary"]);
        asked.expand = Some("renderedFields".into());

        let issues = all_issues_with(&client, asked).await;

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].rendered_fields["comment"]["total"], json!(4));
        assert_eq!(search_bodies(&fake)[0]["expand"], json!("renderedFields"));
    }

    async fn all_issues_with(client: &JiraClient, asked: SearchRequest) -> Vec<Issue> {
        let mut pages = client.search_jql(asked);
        let mut issues = Vec::new();
        while let Some(page) = pages.next_page().await {
            issues.extend(page.unwrap().issues);
        }
        issues
    }
}

mod errors {
    use super::*;

    #[tokio::test]
    async fn statuses_map_to_error_kinds() {
        for (status, expected) in [
            (401, ErrorKind::Auth),
            (403, ErrorKind::Auth),
            (404, ErrorKind::NotFound),
            (400, ErrorKind::Internal),
            (500, ErrorKind::Internal),
            (502, ErrorKind::Internal),
        ] {
            let fake = FakeJira::start_demo().await;
            fake.fail_next(1, status, None);
            let err = fake.client(TokenKind::Classic).myself().await.unwrap_err();
            assert_eq!(err.kind(), expected, "HTTP {status}");
            assert_eq!(fake.paths().len(), 1, "HTTP {status} is not retried");
        }
    }

    #[tokio::test]
    async fn an_unknown_project_is_not_found() {
        let fake = FakeJira::start_demo().await;

        let err = fake
            .client(TokenKind::Classic)
            .versions("NOPE")
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn a_rejected_search_carries_jiras_messages() {
        let fake = FakeJira::start_demo().await;
        fake.answer_next(
            400,
            json!({ "errorMessages": ["Field 'foo' does not exist"], "errors": {} }),
        );
        let client = fake.client(TokenKind::Classic);
        let mut pages = client.search_jql(request("foo = 1", &["summary"]));

        let err = pages.next_page().await.unwrap().unwrap_err();

        assert_eq!(
            err,
            AppError::InvalidJql(vec!["Field 'foo' does not exist".into()])
        );
        assert!(pages.next_page().await.is_none());
    }

    #[tokio::test]
    async fn field_errors_of_a_rejected_query_are_listed_after_the_general_messages() {
        let fake = FakeJira::start_demo().await;
        fake.answer_next(
            400,
            json!({ "errorMessages": ["Bad query"], "errors": { "jql": "Unexpected end" } }),
        );

        let err = fake
            .client(TokenKind::Classic)
            .approximate_count("project = (")
            .await
            .unwrap_err();

        assert_eq!(
            err,
            AppError::InvalidJql(vec!["Bad query".into(), "Unexpected end".into()])
        );
    }

    #[tokio::test]
    async fn a_rejected_query_without_messages_still_says_what_happened() {
        let fake = FakeJira::start_demo().await;
        fake.answer_next(400, json!({}));

        let err = fake
            .client(TokenKind::Classic)
            .check_jql("project = NW")
            .await
            .unwrap_err();

        assert_eq!(
            err,
            AppError::InvalidJql(vec!["Jira rejected the query".into()])
        );
    }

    #[tokio::test]
    async fn a_refused_connection_is_offline() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        fake.go_offline().await;

        let err = client.myself().await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Offline);
    }

    #[tokio::test]
    async fn the_client_works_again_once_the_site_is_back() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        fake.go_offline().await;
        assert_eq!(
            client.myself().await.unwrap_err().kind(),
            ErrorKind::Offline
        );

        fake.go_online().await;

        assert_eq!(client.myself().await.unwrap().display_name, "Alex Morgan");
    }

    #[tokio::test]
    async fn without_a_token_the_client_is_not_connected_and_sends_nothing() {
        let fake = FakeJira::start_demo().await;
        let client = JiraClient::new(
            fake.site(TokenKind::Classic),
            Arc::new(FakeSecrets::default()),
            Arc::new(Limiter::new(1000, 1000)),
        );

        let err = client.myself().await.unwrap_err();
        let mut pages = client.search_jql(request("project = NW", &["summary"]));
        let search_err = pages.next_page().await.unwrap().unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotConnected);
        assert_eq!(search_err.kind(), ErrorKind::NotConnected);
        assert!(fake.requests().is_empty());
    }
}

mod authentication {
    use super::*;

    #[tokio::test]
    async fn every_rest_request_carries_the_basic_credentials() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let expected = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{ME_EMAIL}:{DEMO_TOKEN}"))
        );

        client.myself().await.unwrap();
        client.fields().await.unwrap();
        client.versions("NW").await.unwrap();
        client.comments("NW-1042").await.unwrap();
        client.check_jql("project = NW").await.unwrap();
        client
            .search_jql(request("project = NW", &["summary"]))
            .next_page()
            .await
            .unwrap()
            .unwrap();

        let requests = fake.requests();
        assert_eq!(requests.len(), 6);
        for recorded in requests {
            assert_eq!(
                recorded.authorization.as_deref(),
                Some(expected.as_str()),
                "{}",
                recorded.path
            );
        }
    }

    #[tokio::test]
    async fn the_tenant_info_request_carries_no_credentials_and_returns_the_cloud_id() {
        let fake = FakeJira::start_demo().await;

        let info = fake.client(TokenKind::Classic).tenant_info().await.unwrap();

        assert_eq!(info.cloud_id, CLOUD_ID);
        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].path, "/_edge/tenant_info");
        assert_eq!(requests[0].authorization, None);
    }

    #[tokio::test]
    async fn classic_and_scoped_tokens_use_their_own_paths() {
        for (kind, expected) in [
            (TokenKind::Classic, "/rest/api/3/myself".to_string()),
            (
                TokenKind::Scoped,
                format!("/ex/jira/{CLOUD_ID}/rest/api/3/myself"),
            ),
        ] {
            let fake = FakeJira::start_demo().await;

            let me = fake.client(kind).myself().await.unwrap();

            assert_eq!(me.display_name, "Alex Morgan", "{kind:?}");
            assert_eq!(fake.paths(), [expected], "{kind:?}");
        }
    }
}

mod retries {
    use super::*;

    #[tokio::test]
    async fn rate_limited_requests_are_retried_until_they_succeed() {
        let fake = FakeJira::start_demo().await;
        fake.fail_next(2, 429, Some("0"));

        let me = fake.client(TokenKind::Classic).myself().await.unwrap();

        assert_eq!(me.display_name, "Alex Morgan");
        assert_eq!(fake.paths().len(), 3);
    }

    #[tokio::test]
    async fn a_503_without_retry_after_is_not_retried() {
        let fake = FakeJira::start_demo().await;
        fake.fail_next(1, 503, None);

        let err = fake.client(TokenKind::Classic).myself().await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Internal);
        assert_eq!(fake.paths().len(), 1);
    }

    #[tokio::test]
    async fn a_503_with_retry_after_is_retried() {
        let fake = FakeJira::start_demo().await;
        fake.fail_next(1, 503, Some("0"));

        fake.client(TokenKind::Classic).myself().await.unwrap();

        assert_eq!(fake.paths().len(), 2);
    }
}

mod demo_answers {
    use super::*;

    #[tokio::test]
    async fn myself_reads_the_demo_user() {
        let fake = FakeJira::start_demo().await;

        let me = fake.client(TokenKind::Classic).myself().await.unwrap();

        assert_eq!(me.account_id, "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01");
        assert_eq!(me.display_name, "Alex Morgan");
        assert_eq!(me.time_zone, "Europe/Vienna");
    }

    #[tokio::test]
    async fn the_sprint_field_is_a_custom_greenhopper_field() {
        let fake = FakeJira::start_demo().await;

        let fields = fake.client(TokenKind::Classic).fields().await.unwrap();

        let sprint = fields
            .iter()
            .find(|field| field.id == "customfield_10020")
            .expect("the sprint field");
        assert!(sprint.custom);
        assert_eq!(
            sprint.schema.as_ref().and_then(|s| s.custom.as_deref()),
            Some("com.pyxis.greenhopper.jira:gh-sprint")
        );
    }

    #[tokio::test]
    async fn projects_lists_all_six_demo_projects() {
        let fake = FakeJira::start_demo().await;

        let projects = fake.client(TokenKind::Classic).projects().await.unwrap();

        let keys: Vec<&str> = projects.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys, ["NW", "NWS", "LUM", "FBA", "INT", "ARC"]);
        assert_eq!(projects[0].id, "10001");
        assert_eq!(projects[0].name, "Website relaunch");
        let asked = fake.requests();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].query_value("startAt"), Some("0"));
        assert_eq!(asked[0].query_value("maxResults"), Some("50"));
    }

    #[tokio::test]
    async fn statuses_carry_their_category_keys() {
        let fake = FakeJira::start_demo().await;

        let statuses = fake.client(TokenKind::Classic).statuses().await.unwrap();

        let listed: Vec<(&str, &str)> = statuses
            .iter()
            .map(|s| (s.name.as_str(), s.status_category.key.as_str()))
            .collect();
        assert_eq!(
            listed,
            [
                ("To Do", "new"),
                ("In Progress", "indeterminate"),
                ("In Review", "indeterminate"),
                ("Blocked", "indeterminate"),
                ("Done", "done"),
            ]
        );
    }

    #[tokio::test]
    async fn versions_tell_which_are_released() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);

        let versions = client.versions("NW").await.unwrap();

        let listed: Vec<(&str, bool)> = versions
            .iter()
            .map(|v| (v.name.as_str(), v.released))
            .collect();
        assert_eq!(
            listed,
            [("2.3.1", true), ("2.4.0", false), ("2.5.0", false)]
        );
        assert!(versions[0].release_date.is_some());
        assert!(client.versions("NWS").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn bulk_fetch_returns_existing_issues_and_reports_unknown_ids() {
        let fake = FakeJira::start_demo().await;

        let fetched = fake
            .client(TokenKind::Classic)
            .bulk_fetch(
                &strings(&[NW_1042_ID, NW_1020_ID, "999999"]),
                &strings(&["summary"]),
            )
            .await
            .unwrap();

        let keys: Vec<&str> = fetched.issues.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(keys, ["NW-1042", "NW-1020"]);
        assert_eq!(fetched.issue_errors.len(), 1);
        assert_eq!(fetched.issue_errors[0]["issueIdsOrKeys"], json!(["999999"]));
        assert_eq!(
            fake.requests()[0].body,
            json!({ "issueIdsOrKeys": ["101042", "101020", "999999"], "fields": ["summary"] })
        );
    }

    #[tokio::test]
    async fn the_default_scope_counts_open_work_and_work_finished_in_the_last_90_days() {
        let fake = FakeJira::start_demo().await;

        let count = fake
            .client(TokenKind::Classic)
            .approximate_count(&tracked_scope())
            .await
            .unwrap();

        assert_eq!(count, 21 + 12 + 6 + 2 + 9 + 2 + 11 + 2 + 8 + 1);
    }

    #[tokio::test]
    async fn comments_come_with_their_rendered_html() {
        let fake = FakeJira::start_demo().await;

        let comments = fake
            .client(TokenKind::Classic)
            .comments("NW-1042")
            .await
            .unwrap();

        assert_eq!(comments.len(), 4);
        assert!(comments.iter().all(|c| c.rendered_body.is_some()));
        assert!(comments.iter().all(|c| c.author.is_some()));
        let asked = &fake.requests()[0];
        assert_eq!(asked.query_value("expand"), Some("renderedBody"));
        assert_eq!(asked.query_value("orderBy"), Some("created"));
    }

    #[tokio::test]
    async fn a_valid_query_has_no_problems_and_a_broken_one_has_some() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);

        assert_eq!(
            client.check_jql("project = NW").await.unwrap(),
            Vec::<String>::new()
        );
        assert!(!client.check_jql("project = (").await.unwrap().is_empty());
        let asked = &fake.requests()[0];
        assert_eq!(asked.query_value("validation"), Some("strict"));
        assert_eq!(asked.body, json!({ "queries": ["project = NW"] }));
    }
}

mod fake_knobs {
    use super::*;

    #[tokio::test]
    async fn a_moved_issue_keeps_its_id_gets_a_new_key_and_a_newer_updated_time() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let fields = strings(&["summary", "project", "updated"]);
        let before = client
            .bulk_fetch(&strings(&[NW_1042_ID]), &fields)
            .await
            .unwrap()
            .issues;
        fake.advance(Duration::from_secs(60));

        fake.move_issue("NW-1042", "LUM");

        let after = client
            .bulk_fetch(&strings(&[NW_1042_ID, "NW-1042"]), &fields)
            .await
            .unwrap();
        assert_eq!(after.issues.len(), 1);
        assert_eq!(after.issue_errors.len(), 1);
        let moved = &after.issues[0];
        assert_eq!(moved.id, NW_1042_ID);
        assert!(moved.key.starts_with("LUM-"), "{}", moved.key);
        assert_eq!(moved.project_id(), Some("10003"));
        assert_eq!(moved.summary(), before[0].summary());
        assert!(moved.updated().unwrap() > before[0].updated().unwrap());
    }

    #[tokio::test]
    async fn a_deleted_issue_is_reported_as_an_error_by_bulk_fetch() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);

        fake.delete_issue("NW-1042");

        let fetched = client
            .bulk_fetch(&strings(&[NW_1042_ID, NW_1020_ID]), &strings(&["summary"]))
            .await
            .unwrap();
        assert_eq!(fetched.issues.len(), 1);
        assert_eq!(fetched.issues[0].key, "NW-1020");
        assert_eq!(fetched.issue_errors.len(), 1);
        assert_eq!(
            fetched.issue_errors[0]["issueIdsOrKeys"],
            json!([NW_1042_ID])
        );
        assert!(
            !keys(&client, r#"project = "NW""#)
                .await
                .contains(&"NW-1042".to_string())
        );
    }

    #[tokio::test]
    async fn a_revoked_issue_disappears_from_search_bulk_fetch_and_comments() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        assert!(
            keys(&client, r#"project = "NW""#)
                .await
                .contains(&"NW-1042".to_string())
        );

        fake.revoke_access("NW-1042");

        assert!(
            !keys(&client, r#"project = "NW""#)
                .await
                .contains(&"NW-1042".to_string())
        );
        let fetched = client
            .bulk_fetch(&strings(&[NW_1042_ID]), &strings(&["summary"]))
            .await
            .unwrap();
        assert!(fetched.issues.is_empty());
        assert_eq!(fetched.issue_errors.len(), 1);
        assert_eq!(
            client.comments("NW-1042").await.unwrap_err().kind(),
            ErrorKind::NotFound
        );
    }

    #[tokio::test]
    async fn an_edit_is_found_by_an_updated_since_literal_in_the_users_time_zone() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        fake.advance(Duration::from_secs(3600));
        let edited_at = demo_now() + chrono::Duration::hours(1);

        fake.edit_issue("NW-1042", |fields| {
            fields["summary"] = json!("Checkout: address validation for all postcodes");
        });

        let just_before =
            date_literal(edited_at - chrono::Duration::minutes(1), "Europe/Vienna").unwrap();
        assert_eq!(just_before, "2026-10-28 12:59");
        let found = all_issues(&client, &and_updated_since(&tracked_scope(), &just_before)).await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "NW-1042");
        assert_eq!(
            found[0].summary(),
            "Checkout: address validation for all postcodes"
        );
        assert_eq!(
            found[0].updated(),
            parse_jira_time("2026-10-28T12:00:00.000+0000")
        );
        let after_edit =
            date_literal(edited_at + chrono::Duration::minutes(1), "Europe/Vienna").unwrap();
        assert!(
            all_issues(&client, &and_updated_since(&tracked_scope(), &after_edit))
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn advancing_the_clock_moves_relative_windows() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let updated_times: Vec<DateTime<Utc>> = demo::demo()
            .issues
            .iter()
            .filter_map(|issue| {
                issue["fields"]["updated"]
                    .as_str()
                    .and_then(parse_jira_time)
            })
            .map(|time| time.with_timezone(&Utc))
            .collect();
        let in_last_week = |now: DateTime<Utc>| {
            updated_times
                .iter()
                .filter(|&&time| time >= now - chrono::Duration::days(7))
                .count() as u64
        };

        let before = client.approximate_count("updated >= -7d").await.unwrap();
        fake.advance(Duration::from_secs(30 * 24 * 3600));
        let after = client.approximate_count("updated >= -7d").await.unwrap();

        assert_eq!(before, in_last_week(demo_now()));
        assert!(before > 0);
        assert_eq!(after, in_last_week(demo_now() + chrono::Duration::days(30)));
        assert_eq!(after, 0);
    }
}
