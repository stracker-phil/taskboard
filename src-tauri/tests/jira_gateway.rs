//! The Jira gateway against the loopback fake Jira: answers it refuses, requests that never reach the
//! wire, queries that stay out of the URL, secrets in errors, and an audit that every request the client
//! makes is on the allowlist.

use std::time::Duration;

use reqwest::Method;
use serde_json::json;

use taskboard_lib::error::{AppError, AppResult, ErrorKind};
use taskboard_lib::jira::gateway::{self, Access, BadRequest, MAX_ANSWER_BYTES, Planned};
use taskboard_lib::jira::{SearchRequest, TokenKind};
use taskboard_lib::test_support::demo::ME_EMAIL;
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira, Recorded};

const SEARCH_PATH: &str = "/rest/api/3/search/jql";
const NASTY_JQL: &str = "summary ~ \"a&b?c#d\" AND reporter = 'x'\nORDER BY created";

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn request(jql: &str, fields: &[&str], expand: Option<&str>) -> SearchRequest {
    SearchRequest {
        jql: jql.into(),
        fields: strings(fields),
        expand: expand.map(Into::into),
    }
}

fn failure<T>(what: &str, result: AppResult<T>) -> AppError {
    match result {
        Ok(_) => panic!("{what} should have failed"),
        Err(err) => err,
    }
}

fn leaks_secrets(err: &AppError) -> bool {
    let shown = format!("{err} {err:?}");
    shown.contains(ME_EMAIL) || shown.contains(DEMO_TOKEN)
}

mod refused_answers {
    use super::*;

    #[tokio::test]
    async fn a_redirect_is_not_followed_and_fails() {
        for status in [301, 302] {
            let fake = FakeJira::start_demo().await;
            fake.answer_raw(
                status,
                &[("Location", "http://127.0.0.2:9/steal")],
                "text/plain",
                String::new(),
            );

            let err = fake.client(TokenKind::Classic).myself().await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::Internal, "HTTP {status}");
            assert_eq!(fake.requests().len(), 1, "HTTP {status}");
        }
    }

    #[tokio::test]
    async fn a_successful_answer_that_is_not_json_fails() {
        let fake = FakeJira::start_demo().await;
        fake.answer_raw(
            200,
            &[],
            "text/html",
            "<html><body>Sign in</body></html>".into(),
        );

        let err = fake.client(TokenKind::Classic).myself().await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Internal);
    }

    #[tokio::test]
    async fn an_answer_over_the_size_cap_fails_without_hanging() {
        let fake = FakeJira::start_demo().await;
        fake.answer_raw(
            200,
            &[],
            "application/json",
            format!("\"{}\"", "a".repeat(MAX_ANSWER_BYTES)),
        );

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            fake.client(TokenKind::Classic).myself(),
        )
        .await
        .expect("the oversized answer is refused in time");

        assert_eq!(result.unwrap_err().kind(), ErrorKind::Internal);
    }

    #[tokio::test]
    async fn answers_of_the_wrong_content_fail_as_internal_errors() {
        for (label, body) in [
            ("malformed JSON", "{\"accountId\": "),
            ("the wrong shape", "{\"unexpected\": true}"),
            ("a list", "[1, 2, 3]"),
            ("an empty body", ""),
            ("null", "null"),
        ] {
            let fake = FakeJira::start_demo().await;
            fake.answer_raw(200, &[], "application/json", body.into());

            let err = fake.client(TokenKind::Classic).myself().await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::Internal, "{label}");
        }
    }

    #[tokio::test]
    async fn a_wrong_shaped_search_answer_ends_the_cursor() {
        let fake = FakeJira::start_demo().await;
        fake.answer_raw(
            200,
            &[],
            "application/json",
            r#"{"issues": "not a list"}"#.into(),
        );
        let client = fake.client(TokenKind::Classic);
        let mut pages = client.search_jql(request("project = NW", &["summary"], None));

        let err = pages.next_page().await.unwrap().unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Internal);
        assert!(pages.next_page().await.is_none());
    }

    #[tokio::test]
    async fn auth_and_not_found_statuses_keep_their_kinds() {
        for (status, expected) in [
            (401, ErrorKind::Auth),
            (403, ErrorKind::Auth),
            (404, ErrorKind::NotFound),
        ] {
            let fake = FakeJira::start_demo().await;
            fake.fail_next(1, status, None);

            let err = fake.client(TokenKind::Classic).myself().await.unwrap_err();

            assert_eq!(err.kind(), expected, "HTTP {status}");
        }
    }
}

mod refused_requests {
    use super::*;

    #[tokio::test]
    async fn requests_outside_the_allowlist_fail_as_invalid_input_and_send_nothing() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let many_ids: Vec<String> = (0..101).map(|n| (100_000 + n).to_string()).collect();

        let mut refused = vec![
            (
                "a project key with path segments",
                failure("versions", client.versions("nw/../../issue/1").await),
            ),
            (
                "a project key with a query",
                failure("versions", client.versions("NW?x=1").await),
            ),
            (
                "an issue key that climbs out",
                failure("comments", client.comments("../myself").await),
            ),
            (
                "a bulk fetch of nothing",
                failure(
                    "bulk_fetch",
                    client.bulk_fetch(&[], &strings(&["summary"])).await,
                ),
            ),
            (
                "a bulk fetch of 101 issues",
                failure(
                    "bulk_fetch",
                    client.bulk_fetch(&many_ids, &strings(&["summary"])).await,
                ),
            ),
            (
                "a bulk fetch of every field",
                failure(
                    "bulk_fetch",
                    client
                        .bulk_fetch(&strings(&["101042"]), &strings(&["*all"]))
                        .await,
                ),
            ),
            (
                "a count of an empty query",
                failure("approximate_count", client.approximate_count("").await),
            ),
            (
                "a check of an empty query",
                failure("check_jql", client.check_jql("").await),
            ),
        ];
        for (label, asked) in [
            (
                "a search of every field",
                request("project = NW", &["*all"], None),
            ),
            (
                "a search expanding the changelog",
                request("project = NW", &["summary"], Some("changelog")),
            ),
            (
                "a search of an empty query",
                request(" ", &["summary"], None),
            ),
        ] {
            let mut pages = client.search_jql(asked);
            refused.push((label, failure(label, pages.next_page().await.unwrap())));
        }

        for (label, err) in &refused {
            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{label}: {err}");
        }
        assert!(fake.requests().is_empty());
    }
}

mod queries_stay_out_of_the_url {
    use super::*;

    fn assert_clean_url(asked: &Recorded) {
        let mut shown = vec![asked.path.clone()];
        for (key, value) in &asked.query {
            shown.push(key.clone());
            shown.push(value.clone());
        }
        for text in shown {
            for forbidden in ['&', '?', '#', '"', '\'', '\n', '~'] {
                assert!(!text.contains(forbidden), "{text:?} contains {forbidden:?}");
            }
        }
    }

    #[tokio::test]
    async fn an_approximate_count_sends_the_query_in_the_body_only() {
        let fake = FakeJira::start_demo().await;

        let _ = fake
            .client(TokenKind::Classic)
            .approximate_count(NASTY_JQL)
            .await;

        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert_clean_url(&requests[0]);
        assert_eq!(requests[0].body, json!({ "jql": NASTY_JQL }));
    }

    #[tokio::test]
    async fn a_query_check_sends_the_query_in_the_body_only() {
        let fake = FakeJira::start_demo().await;

        let _ = fake.client(TokenKind::Classic).check_jql(NASTY_JQL).await;

        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert_clean_url(&requests[0]);
        assert_eq!(requests[0].body, json!({ "queries": [NASTY_JQL] }));
    }

    #[tokio::test]
    async fn a_search_sends_the_query_in_the_body_only() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);

        let _ = client
            .search_jql(request(NASTY_JQL, &["summary"], None))
            .next_page()
            .await;

        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].path, SEARCH_PATH);
        assert_clean_url(&requests[0]);
        assert_eq!(requests[0].body["jql"], json!(NASTY_JQL));
    }
}

mod secrets {
    use super::*;

    #[tokio::test]
    async fn the_tenant_info_request_carries_no_authorization() {
        for kind in [TokenKind::Classic, TokenKind::Scoped] {
            let fake = FakeJira::start_demo().await;

            fake.client(kind).tenant_info().await.unwrap();

            let requests = fake.requests();
            assert_eq!(requests.len(), 1, "{kind:?}");
            assert_eq!(requests[0].authorization, None, "{kind:?}");
        }
    }

    #[tokio::test]
    async fn errors_do_not_contain_the_email_or_the_token() {
        let mut errors: Vec<(&str, AppError)> = Vec::new();

        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        errors.push((
            "a refused request",
            failure("versions", client.versions("NW/../x").await),
        ));

        let fake = FakeJira::start_demo().await;
        fake.fail_next(1, 401, None);
        errors.push((
            "a 401",
            failure("myself", fake.client(TokenKind::Classic).myself().await),
        ));

        let fake = FakeJira::start_demo().await;
        fake.answer_raw(
            302,
            &[("Location", "http://127.0.0.2:9/steal")],
            "text/plain",
            String::new(),
        );
        errors.push((
            "a redirect",
            failure("myself", fake.client(TokenKind::Classic).myself().await),
        ));

        for (label, err) in &errors {
            assert!(!leaks_secrets(err), "{label}: {err}");
        }
    }
}

mod writes {
    use super::*;
    use taskboard_lib::jira::JiraClient;
    use taskboard_lib::jira::dto::parse_jira_time;
    use taskboard_lib::test_support::demo;

    const ISSUE: &str = "NW-1042";
    const PATH: &str = "/rest/api/3/issue/NW-1042/assignee";

    fn demo_account(index: usize) -> String {
        demo::demo().users[index]["accountId"]
            .as_str()
            .unwrap()
            .to_string()
    }

    async fn issue_state(client: &JiraClient) -> (serde_json::Value, String) {
        let found = client
            .bulk_fetch(
                &strings(&[ISSUE]),
                &strings(&["summary", "assignee", "updated"]),
            )
            .await
            .unwrap();
        let fields = found.issues[0].fields.clone();
        let updated = fields["updated"].as_str().unwrap().to_string();
        (fields["assignee"].clone(), updated)
    }

    fn message(err: AppError) -> String {
        match err {
            AppError::InvalidInput(message) => message,
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn assigning_an_issue_sets_its_assignee_and_makes_it_newer() {
        for kind in [TokenKind::Classic, TokenKind::Scoped] {
            let fake = FakeJira::start_demo().await;
            let client = fake.client(kind);
            let who = demo_account(1);
            let (_, updated_before) = issue_state(&client).await;

            client.assign(ISSUE, Some(&who)).await.unwrap();

            let (assignee, updated_after) = issue_state(&client).await;
            assert_eq!(assignee["accountId"], json!(who), "{kind:?}");
            assert!(
                parse_jira_time(&updated_after).unwrap()
                    > parse_jira_time(&updated_before).unwrap(),
                "{kind:?}: {updated_before} -> {updated_after}"
            );
        }
    }

    #[tokio::test]
    async fn assigning_sends_one_put_with_the_account_id() {
        let fake = FakeJira::start_demo().await;
        let who = demo_account(2);

        fake.client(TokenKind::Classic)
            .assign(ISSUE, Some(&who))
            .await
            .unwrap();

        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "PUT");
        assert_eq!(requests[0].path, PATH);
        assert!(requests[0].query.is_empty());
        assert_eq!(requests[0].body, json!({ "accountId": who }));
        assert!(requests[0].authorization.is_some());
    }

    #[tokio::test]
    async fn unassigning_clears_the_assignee_and_sends_a_null_account_id() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        client.assign(ISSUE, Some(&demo_account(1))).await.unwrap();

        client.assign(ISSUE, None).await.unwrap();

        let (assignee, _) = issue_state(&client).await;
        assert!(assignee.is_null(), "{assignee}");
        let requests = fake.requests();
        let last_put = requests.iter().rfind(|r| r.method == "PUT").unwrap();
        assert_eq!(last_put.body, json!({ "accountId": null }));
    }

    #[tokio::test]
    async fn the_recorded_write_passes_the_guard_as_a_write_and_not_as_a_read() {
        for kind in [TokenKind::Classic, TokenKind::Scoped] {
            let fake = FakeJira::start_demo().await;
            fake.client(kind)
                .assign(ISSUE, Some(&demo_account(1)))
                .await
                .unwrap();
            let recorded = fake.requests().remove(0);
            let site = fake.site(kind);
            let mut url = fake.url().join(&recorded.path).unwrap();
            url.query_pairs_mut().extend_pairs(&recorded.query);
            let mut planned = Planned {
                method: Method::PUT,
                url,
                body: Some(recorded.body.clone()),
                authenticated: recorded.authorization.is_some(),
                access: Access::Write,
                bad_request: BadRequest::Plain,
            };

            assert!(gateway::check(&planned, &site).is_ok(), "{kind:?}");
            planned.access = Access::Read;
            assert!(gateway::check(&planned, &site).is_err(), "{kind:?}");
        }
    }

    #[tokio::test]
    async fn an_account_the_site_does_not_know_is_refused_with_jiras_reason() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let (before, _) = issue_state(&client).await;

        let err = client
            .assign(ISSUE, Some("5b10ac8d82e05b22cc7d4ef5"))
            .await
            .unwrap_err();

        assert_eq!(
            message(err),
            "Jira refused the change: User '5b10ac8d82e05b22cc7d4ef5' cannot be assigned issues."
        );
        let (after, _) = issue_state(&client).await;
        assert_eq!(after, before);
    }

    #[tokio::test]
    async fn an_issue_that_does_not_exist_or_is_hidden_is_not_found() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        fake.revoke_access("NW-1020");

        for target in ["NW-9999", "NW-1020"] {
            let err = client
                .assign(target, Some(&demo_account(1)))
                .await
                .unwrap_err();

            assert_eq!(err.kind(), ErrorKind::NotFound, "{target}");
        }
    }

    #[tokio::test]
    async fn a_refused_write_permission_is_invalid_input_naming_the_scope() {
        let fake = FakeJira::start_demo().await;
        fake.fail_next(1, 403, None);

        let err = fake
            .client(TokenKind::Scoped)
            .assign(ISSUE, None)
            .await
            .unwrap_err();

        assert_eq!(
            message(err),
            "Jira refused the change: you may lack the permission, or a scoped token lacks write:jira-work"
        );
    }

    #[tokio::test]
    async fn a_read_refused_with_403_is_still_an_auth_error() {
        let fake = FakeJira::start_demo().await;
        fake.fail_next(1, 403, None);

        let err = fake.client(TokenKind::Classic).myself().await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Auth);
    }

    #[tokio::test]
    async fn a_bad_request_shows_what_jira_said_and_falls_back_when_it_said_nothing() {
        for (body, expected) in [
            (
                json!({ "errors": { "assignee": "User 'x' cannot be assigned issues." } }),
                "Jira refused the change: User 'x' cannot be assigned issues.",
            ),
            (
                json!({ "errorMessages": ["Issue is archived."], "errors": {} }),
                "Jira refused the change: Issue is archived.",
            ),
            (
                json!({ "errorMessages": [], "errors": {} }),
                "Jira refused the change: it gave no reason",
            ),
        ] {
            let fake = FakeJira::start_demo().await;
            fake.answer_next(400, body.clone());

            let err = fake
                .client(TokenKind::Classic)
                .assign(ISSUE, None)
                .await
                .unwrap_err();

            assert_eq!(message(err), expected, "{body}");
        }
    }

    #[tokio::test]
    async fn a_write_that_is_rate_limited_or_unavailable_is_sent_once_and_fails() {
        for (status, retry_after) in [(429, Some("0")), (503, Some("0"))] {
            let fake = FakeJira::start_demo().await;
            fake.fail_next(1, status, retry_after);
            let client = fake.client(TokenKind::Classic);

            let err = client
                .assign(ISSUE, Some(&demo_account(1)))
                .await
                .unwrap_err();

            assert_eq!(err.kind(), ErrorKind::Internal, "HTTP {status}");
            assert_eq!(fake.requests().len(), 1, "HTTP {status}");
            let (assignee, _) = issue_state(&client).await;
            assert_ne!(
                assignee["accountId"],
                json!(demo_account(1)),
                "HTTP {status}"
            );
        }
    }

    #[tokio::test]
    async fn an_account_id_or_issue_that_could_change_the_url_sends_nothing() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);

        for (label, result) in [
            (
                "an account id with a slash",
                client.assign(ISSUE, Some("a/b")).await,
            ),
            ("an empty account id", client.assign(ISSUE, Some("")).await),
            (
                "an issue that climbs out",
                client.assign("../myself", None).await,
            ),
        ] {
            assert_eq!(
                result.unwrap_err().kind(),
                ErrorKind::InvalidInput,
                "{label}"
            );
        }
        assert!(fake.requests().is_empty());
    }

    async fn status_id_of(client: &JiraClient) -> String {
        let found = client
            .bulk_fetch(&strings(&[ISSUE]), &strings(&["status"]))
            .await
            .unwrap();
        found.issues[0].fields["status"]["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[tokio::test]
    async fn the_transitions_of_an_issue_are_one_move_to_every_other_status_in_the_sites_order() {
        let fake = FakeJira::start_demo().await;
        fake.require_screen("10001");

        let moves = fake
            .client(TokenKind::Classic)
            .transitions(ISSUE)
            .await
            .unwrap();

        let listed: Vec<(&str, &str, &str, bool)> = moves
            .iter()
            .map(|t| {
                (
                    t.id.as_str(),
                    t.name.as_str(),
                    t.to.id.as_str(),
                    t.has_screen,
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                ("10000", "To Do", "10000", false),
                ("10002", "In Review", "10002", false),
                ("10003", "Blocked", "10003", false),
                ("10001", "Done", "10001", true),
            ]
        );
        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].path, "/rest/api/3/issue/NW-1042/transitions");
    }

    #[tokio::test]
    async fn moving_an_issue_sends_one_post_sets_its_status_and_makes_it_newer() {
        for kind in [TokenKind::Classic, TokenKind::Scoped] {
            let fake = FakeJira::start_demo().await;
            let client = fake.client(kind);
            let (_, updated_before) = issue_state(&client).await;

            client.transition(ISSUE, "10001").await.unwrap();

            assert_eq!(status_id_of(&client).await, "10001", "{kind:?}");
            let (_, updated_after) = issue_state(&client).await;
            assert!(
                parse_jira_time(&updated_after).unwrap()
                    > parse_jira_time(&updated_before).unwrap(),
                "{kind:?}: {updated_before} -> {updated_after}"
            );
            let posts: Vec<_> = fake
                .requests()
                .into_iter()
                .filter(|r| r.method == "POST" && r.path.ends_with("/transitions"))
                .collect();
            assert_eq!(posts.len(), 1, "{kind:?}");
            let base = fake.site(kind).api_base().unwrap();
            assert_eq!(
                posts[0].path,
                format!("{}issue/NW-1042/transitions", base.path()),
                "{kind:?}"
            );
            assert_eq!(
                posts[0].body,
                json!({ "transition": { "id": "10001" } }),
                "{kind:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_move_the_issue_cannot_make_is_refused_and_leaves_the_status_alone() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let before = status_id_of(&client).await;

        for (label, id) in [("an unknown id", "99999"), ("the current status", "3")] {
            let err = client.transition(ISSUE, id).await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{label}");
            assert_eq!(status_id_of(&client).await, before, "{label}");
        }
    }

    #[tokio::test]
    async fn moving_an_issue_that_is_hidden_is_not_found() {
        let fake = FakeJira::start_demo().await;
        fake.revoke_access("NW-1020");

        let err = fake
            .client(TokenKind::Classic)
            .transition("NW-1020", "10001")
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn a_transition_id_that_could_change_the_url_or_body_sends_nothing() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);

        for (label, id) in [
            ("a slash", "1/2"),
            ("letters", "done"),
            ("empty", ""),
            ("a climb", "../x"),
        ] {
            let err = client.transition(ISSUE, id).await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{label}");
        }
        assert!(fake.requests().is_empty());
    }

    #[tokio::test]
    async fn a_move_is_sent_once_when_jira_is_rate_limited() {
        let fake = FakeJira::start_demo().await;
        fake.fail_next(1, 429, Some("0"));
        let client = fake.client(TokenKind::Classic);

        let err = client.transition(ISSUE, "10001").await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Internal);
        assert_eq!(fake.requests().len(), 1);
        assert_eq!(status_id_of(&client).await, "3");
    }

    fn comment_body(text: &str) -> serde_json::Value {
        taskboard_lib::adf::from_plain_text(text)
    }

    async fn comment_count(client: &JiraClient) -> usize {
        client.comments(ISSUE).await.unwrap().len()
    }

    #[tokio::test]
    async fn adding_a_comment_posts_the_body_once_and_returns_the_stored_comment() {
        for kind in [TokenKind::Classic, TokenKind::Scoped] {
            let fake = FakeJira::start_demo().await;
            let client = fake.client(kind);
            let before = comment_count(&client).await;
            let requests_before = fake.requests().len();

            let stored = client
                .add_comment(ISSUE, comment_body("Checked on staging\n\nAll green"))
                .await
                .unwrap();

            assert_eq!(
                stored.rendered_body.as_deref(),
                Some("<p>Checked on staging</p><p>All green</p>"),
                "{kind:?}"
            );
            assert_eq!(
                stored.author.as_ref().map(|a| a.display_name.as_str()),
                Some("Alex Morgan"),
                "{kind:?}"
            );
            assert_eq!(stored.created, stored.updated, "{kind:?}");
            assert_eq!(
                stored.body,
                comment_body("Checked on staging\n\nAll green"),
                "{kind:?}"
            );
            let posts: Vec<_> = fake
                .requests()
                .into_iter()
                .skip(requests_before)
                .filter(|r| r.method == "POST")
                .collect();
            assert_eq!(posts.len(), 1, "{kind:?}");
            let base = fake.site(kind).api_base().unwrap();
            assert_eq!(
                posts[0].path,
                format!("{}issue/NW-1042/comment", base.path()),
                "{kind:?}"
            );
            assert_eq!(
                posts[0].query,
                [("expand".to_string(), "renderedBody".to_string())],
                "{kind:?}"
            );
            assert_eq!(
                posts[0].body,
                json!({ "body": comment_body("Checked on staging\n\nAll green") }),
                "{kind:?}"
            );
            assert_eq!(comment_count(&client).await, before + 1, "{kind:?}");
        }
    }

    #[tokio::test]
    async fn adding_a_comment_makes_the_issue_newer() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let (_, updated_before) = issue_state(&client).await;

        client
            .add_comment(ISSUE, comment_body("Checked"))
            .await
            .unwrap();

        let (_, updated_after) = issue_state(&client).await;
        assert!(
            parse_jira_time(&updated_after).unwrap() > parse_jira_time(&updated_before).unwrap(),
            "{updated_before} -> {updated_after}"
        );
    }

    #[tokio::test]
    async fn a_body_that_is_not_an_adf_document_is_refused_and_adds_nothing() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);
        let before = comment_count(&client).await;

        let err = client
            .add_comment(ISSUE, json!("plain text"))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(comment_count(&client).await, before);
    }

    #[tokio::test]
    async fn commenting_on_a_hidden_issue_is_not_found() {
        let fake = FakeJira::start_demo().await;
        fake.revoke_access("NW-1020");

        let err = fake
            .client(TokenKind::Classic)
            .add_comment("NW-1020", comment_body("Checked"))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn a_comment_is_sent_once_when_jira_is_rate_limited() {
        let fake = FakeJira::start_demo().await;
        fake.answer_next_write(429, json!({ "errorMessages": ["Slow down"] }));
        let client = fake.client(TokenKind::Classic);
        let before = comment_count(&client).await;
        let requests_before = fake.requests().len();

        let err = client
            .add_comment(ISSUE, comment_body("Checked"))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Internal);
        assert_eq!(fake.requests().len(), requests_before + 1);
        assert_eq!(comment_count(&client).await, before);
    }

    #[tokio::test]
    async fn an_issue_that_could_change_the_url_gets_no_comment_sent() {
        let fake = FakeJira::start_demo().await;

        let err = fake
            .client(TokenKind::Classic)
            .add_comment("../myself", comment_body("Checked"))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(fake.requests().is_empty());
    }
}

mod audit {
    use super::*;

    fn planned(fake: &FakeJira, recorded: &Recorded) -> Planned {
        let mut url = fake.url().join(&recorded.path).unwrap();
        if !recorded.query.is_empty() {
            url.query_pairs_mut().extend_pairs(&recorded.query);
        }
        Planned {
            method: Method::from_bytes(recorded.method.as_bytes()).unwrap(),
            url,
            body: (!recorded.body.is_null()).then(|| recorded.body.clone()),
            authenticated: recorded.authorization.is_some(),
            access: Access::Read,
            bad_request: BadRequest::Plain,
        }
    }

    #[tokio::test]
    async fn every_request_the_client_makes_is_on_the_allowlist() {
        for kind in [TokenKind::Classic, TokenKind::Scoped] {
            let fake = FakeJira::start_demo().await;
            fake.set_page_size(2);
            let client = fake.client(kind);

            client.myself().await.unwrap();
            client.tenant_info().await.unwrap();
            client.fields().await.unwrap();
            client.statuses().await.unwrap();
            client.projects().await.unwrap();
            client.versions("NW").await.unwrap();
            let mut pages = client.search_jql(request(
                "project = NW",
                &["summary", "status"],
                Some("renderedFields"),
            ));
            let mut search_pages = 0;
            while let Some(page) = pages.next_page().await {
                page.unwrap();
                search_pages += 1;
            }
            client
                .bulk_fetch(&strings(&["101042", "NW-1020"]), &strings(&["summary"]))
                .await
                .unwrap();
            client.approximate_count("project = NW").await.unwrap();
            client.comments("NW-1042").await.unwrap();
            client.check_jql("project = NW").await.unwrap();

            assert!(search_pages > 1, "{kind:?}");
            let site = fake.site(kind);
            let requests = fake.requests();
            assert!(!requests.is_empty(), "{kind:?}");
            for recorded in &requests {
                let planned = planned(&fake, recorded);
                assert!(
                    gateway::check(&planned, &site).is_ok(),
                    "{kind:?}: {} {}",
                    recorded.method,
                    recorded.path
                );
                assert!(
                    matches!(recorded.method.as_str(), "GET" | "POST"),
                    "{kind:?}: {}",
                    recorded.method
                );
            }
        }
    }

    #[tokio::test]
    async fn the_rebuilt_request_is_refused_once_its_method_or_path_leaves_the_list() {
        let fake = FakeJira::start_demo().await;
        fake.client(TokenKind::Classic).myself().await.unwrap();
        let recorded = fake.requests().remove(0);
        let site = fake.site(TokenKind::Classic);

        let mut deleted = planned(&fake, &recorded);
        deleted.method = Method::DELETE;
        let mut elsewhere = planned(&fake, &recorded);
        elsewhere.url = fake.url().join("/rest/api/3/user/anonymize").unwrap();

        assert!(gateway::check(&planned(&fake, &recorded), &site).is_ok());
        for (label, refused) in [("a DELETE", deleted), ("an unlisted path", elsewhere)] {
            assert!(gateway::check(&refused, &site).is_err(), "{label}");
        }
    }
}
