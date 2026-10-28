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
