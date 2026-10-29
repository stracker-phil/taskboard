//! The allowlist end to end: what each endpoint plans, what the guard accepts, and that nothing else passes.

use proptest::prelude::*;
use reqwest::Method;
use serde_json::{Value, json};
use url::Url;

use super::{
    Access, BadRequest, Endpoint, Expand, FieldIds, IssueRef, IssueRefs, Jql, PageToken, Planned,
    ProjectKey, check,
};
use crate::error::AppError;
use crate::jira::site::{Site, TokenKind};

const CLOUD_ID: &str = "11111111-2222-3333-4444-555555555555";
const EMAIL: &str = "alex.morgan@example.com";
const QUERY: &str = "project = NW AND status != Done ORDER BY created DESC";

fn classic() -> Site {
    Site::new("example", EMAIL, TokenKind::Classic, None).unwrap()
}

fn scoped() -> Site {
    Site::new("example", EMAIL, TokenKind::Scoped, Some(CLOUD_ID.into())).unwrap()
}

/// Both token kinds with the exact API base each one must call.
fn sites() -> [(String, Site); 2] {
    [
        (
            "https://example.atlassian.net/rest/api/3/".to_string(),
            classic(),
        ),
        (
            format!("https://api.atlassian.com/ex/jira/{CLOUD_ID}/rest/api/3/"),
            scoped(),
        ),
    ]
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn fields() -> FieldIds {
    FieldIds::parse(&strings(&["summary", "customfield_10016"])).unwrap()
}

fn jql(text: &str) -> Jql {
    Jql::parse(text).unwrap()
}

fn project(key: &str) -> ProjectKey {
    ProjectKey::parse(key).unwrap()
}

fn issue(text: &str) -> IssueRef {
    IssueRef::parse(text).unwrap()
}

fn search(jql: Jql) -> Endpoint {
    Endpoint::SearchJql {
        jql,
        fields: fields(),
        expand: None,
        page_token: None,
    }
}

/// Every endpoint (REST ones with their path and query below the API base) with its method and body.
fn allowlist() -> Vec<(Endpoint, Method, &'static str, Option<Value>)> {
    let fields_json = json!(["summary", "customfield_10016"]);
    vec![
        (Endpoint::Myself, Method::GET, "myself", None),
        (Endpoint::Fields, Method::GET, "field", None),
        (Endpoint::Statuses, Method::GET, "status", None),
        (
            Endpoint::ProjectSearch { start_at: 0 },
            Method::GET,
            "project/search?startAt=0&maxResults=50",
            None,
        ),
        (
            Endpoint::ProjectSearch { start_at: 150 },
            Method::GET,
            "project/search?startAt=150&maxResults=50",
            None,
        ),
        (
            Endpoint::ProjectVersions {
                project: project("NW"),
            },
            Method::GET,
            "project/NW/versions",
            None,
        ),
        (
            search(jql(QUERY)),
            Method::POST,
            "search/jql",
            Some(json!({ "jql": QUERY, "maxResults": 100, "fields": fields_json })),
        ),
        (
            Endpoint::SearchJql {
                jql: jql(QUERY),
                fields: fields(),
                expand: Some(Expand::parse("renderedFields").unwrap()),
                page_token: Some(PageToken::parse("tok_8f2a==").unwrap()),
            },
            Method::POST,
            "search/jql",
            Some(json!({
                "jql": QUERY,
                "maxResults": 100,
                "fields": fields_json,
                "expand": "renderedFields",
                "nextPageToken": "tok_8f2a==",
            })),
        ),
        (
            Endpoint::BulkFetch {
                issues: IssueRefs::parse(&strings(&["NW-1042", "10001"])).unwrap(),
                fields: fields(),
            },
            Method::POST,
            "issue/bulkfetch",
            Some(json!({ "issueIdsOrKeys": ["NW-1042", "10001"], "fields": fields_json })),
        ),
        (
            Endpoint::ApproximateCount { jql: jql(QUERY) },
            Method::POST,
            "search/approximate-count",
            Some(json!({ "jql": QUERY })),
        ),
        (
            Endpoint::IssueComments {
                issue: issue("NW-1042"),
                start_at: 200,
            },
            Method::GET,
            "issue/NW-1042/comment?startAt=200&maxResults=100&orderBy=created&expand=renderedBody",
            None,
        ),
        (
            Endpoint::IssueComments {
                issue: issue("10001"),
                start_at: 0,
            },
            Method::GET,
            "issue/10001/comment?startAt=0&maxResults=100&orderBy=created&expand=renderedBody",
            None,
        ),
        (
            Endpoint::JqlParse { jql: jql(QUERY) },
            Method::POST,
            "jql/parse?validation=strict",
            Some(json!({ "queries": [QUERY] })),
        ),
        (
            Endpoint::FieldUsage {
                project: project("NW"),
            },
            Method::POST,
            "search/jql",
            Some(json!({
                "jql": "project = \"NW\" ORDER BY updated DESC",
                "maxResults": 100,
                "fields": [
                    "*all",
                    "-description",
                    "-comment",
                    "-attachment",
                    "-worklog",
                    "-issuelinks",
                    "-subtasks",
                    "-environment",
                ],
            })),
        ),
    ]
}

fn plans(site: &Site) -> Vec<Planned> {
    let mut planned: Vec<Planned> = allowlist()
        .iter()
        .map(|(endpoint, ..)| endpoint.plan(site).unwrap())
        .collect();
    planned.push(Endpoint::TenantInfo.plan(site).unwrap());
    planned
}

/// A request built by hand, as a stored value or a bug might produce it.
fn planned(method: Method, url: &str) -> Planned {
    let body = (method == Method::POST).then(|| json!({}));
    Planned {
        method,
        url: Url::parse(url).unwrap(),
        body,
        authenticated: true,
        access: Access::Read,
        bad_request: BadRequest::Plain,
    }
}

/// `relative` hangs off the API base's origin or gateway prefix (before `rest/api/3/`).
fn url_for(site: &Site, relative: &str) -> String {
    let base = site.api_base().unwrap().to_string();
    let root = base.strip_suffix("rest/api/3/").unwrap().to_string();
    format!("{root}{relative}")
}

fn assert_refused(planned: &Planned, site: &Site, context: &str) {
    match check(planned, site) {
        Err(AppError::Internal(message)) => assert!(
            message.starts_with("Taskboard doesn't call"),
            "{context}: {message}"
        ),
        other => panic!("{context}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn every_endpoint_plans_its_exact_method_url_and_body() {
    for (prefix, site) in sites() {
        for (endpoint, method, path, body) in allowlist() {
            let plan = endpoint.plan(&site).unwrap();
            assert_eq!(plan.url.as_str(), format!("{prefix}{path}"), "{endpoint:?}");
            assert_eq!(plan.method, method, "{endpoint:?}");
            assert_eq!(plan.body, body, "{endpoint:?}");
            assert!(plan.authenticated, "{endpoint:?}");
            assert_eq!(plan.access, Access::Read, "{endpoint:?}");
        }
    }
}

#[test]
fn tenant_info_is_an_unauthenticated_get_on_the_site_for_both_token_kinds() {
    for (_, site) in sites() {
        let plan = Endpoint::TenantInfo.plan(&site).unwrap();
        assert_eq!(
            plan.url.as_str(),
            "https://example.atlassian.net/_edge/tenant_info"
        );
        assert_eq!(plan.method, Method::GET);
        assert_eq!(plan.body, None);
        assert!(!plan.authenticated);
        assert_eq!(plan.access, Access::Read);
    }
}

#[test]
fn only_the_query_endpoints_read_a_bad_request_as_a_jql_error() {
    for site in [classic(), scoped()] {
        for (endpoint, ..) in allowlist() {
            let expected = match endpoint {
                Endpoint::SearchJql { .. }
                | Endpoint::ApproximateCount { .. }
                | Endpoint::JqlParse { .. }
                | Endpoint::FieldUsage { .. } => BadRequest::Jql,
                _ => BadRequest::Plain,
            };
            assert_eq!(
                endpoint.plan(&site).unwrap().bad_request,
                expected,
                "{endpoint:?}"
            );
        }
    }
}

#[test]
fn a_field_usage_sample_reads_the_search_endpoint_and_passes_the_guard_for_both_token_kinds() {
    for (prefix, site) in sites() {
        let plan = Endpoint::FieldUsage {
            project: project("LUM"),
        }
        .plan(&site)
        .unwrap();

        assert_eq!(plan.url.as_str(), format!("{prefix}search/jql"));
        assert_eq!(plan.method, Method::POST);
        assert_eq!(plan.access, Access::Read);
        assert_eq!(
            plan.body.as_ref().unwrap()["jql"],
            json!("project = \"LUM\" ORDER BY updated DESC")
        );
        assert!(check(&plan, &site).is_ok(), "{prefix}");
    }
}

#[test]
fn a_project_key_that_could_change_the_sampling_query_is_refused_before_planning() {
    for key in [
        "",
        "nw",
        "NW\" OR project = \"INT",
        "NW ORDER BY created",
        "NW/versions",
        "NW?x=1",
    ] {
        assert!(
            matches!(ProjectKey::parse(key), Err(AppError::InvalidInput(_))),
            "{key:?}"
        );
    }
}

#[test]
fn a_scoped_site_without_a_usable_cloud_id_plans_nothing_on_the_rest_api() {
    for cloud_id in [None, Some("../x"), Some("a/b"), Some("a?b")] {
        let site = Site::new(
            "example",
            EMAIL,
            TokenKind::Scoped,
            cloud_id.map(Into::into),
        )
        .unwrap();
        for (endpoint, ..) in allowlist() {
            assert!(
                matches!(endpoint.plan(&site), Err(AppError::NotConnected(_))),
                "{cloud_id:?} {endpoint:?}"
            );
        }
    }
}

#[test]
fn the_guard_accepts_every_planned_request_of_every_endpoint_for_both_token_kinds() {
    for (prefix, site) in sites() {
        for plan in plans(&site) {
            assert!(check(&plan, &site).is_ok(), "{prefix} {plan:?}");
        }
    }
}

#[test]
fn the_guard_accepts_the_allowed_requests_when_built_by_hand() {
    for (_, site) in sites() {
        for (method, relative) in [
            (Method::GET, "rest/api/3/myself"),
            (
                Method::GET,
                "rest/api/3/project/search?maxResults=50&startAt=0",
            ),
            (Method::GET, "rest/api/3/project/LUM/versions"),
            (Method::GET, "rest/api/3/issue/NW-1/comment"),
            (Method::POST, "rest/api/3/search/jql"),
            (Method::POST, "rest/api/3/jql/parse"),
        ] {
            let plan = planned(method, &url_for(&site, relative));
            assert!(check(&plan, &site).is_ok(), "{relative}");
        }
    }
}

#[test]
fn paths_methods_and_queries_outside_the_table_are_refused() {
    let cases = [
        (Method::GET, "rest/api/3/issue/1"),
        (Method::GET, "rest/api/3/issue/1/transitions"),
        (Method::POST, "rest/api/3/issue/1/transitions"),
        (Method::POST, "rest/api/3/issue/1/comment"),
        (Method::POST, "rest/api/3/issue"),
        (Method::PUT, "rest/api/3/issue/NW-1"),
        (Method::DELETE, "rest/api/3/issue/NW-1"),
        (Method::POST, "rest/api/3/myself"),
        (Method::PUT, "rest/api/3/myself"),
        (Method::DELETE, "rest/api/3/myself"),
        (Method::PATCH, "rest/api/3/myself"),
        (Method::HEAD, "rest/api/3/myself"),
        (Method::OPTIONS, "rest/api/3/myself"),
        (Method::GET, "rest/api/3/search"),
        (Method::POST, "rest/api/3/search"),
        (Method::GET, "rest/api/2/myself"),
        (Method::POST, "rest/api/2/search"),
        (Method::GET, "rest/api/latest/myself"),
        (Method::GET, "rest/agile/1.0/board"),
        (Method::GET, "wiki/rest/api/content"),
        (Method::GET, "myself"),
        (Method::GET, "rest/api/3"),
        (Method::GET, "rest/api/3/"),
        (Method::GET, "rest/api/3//myself"),
        (Method::POST, "rest/api/3/field"),
        (Method::POST, "rest/api/3/status"),
        (Method::POST, "rest/api/3/project/search"),
        (Method::POST, "rest/api/3/project/NW/versions"),
        (Method::PUT, "rest/api/3/project/NW/versions"),
        (Method::GET, "rest/api/3/search/jql"),
        (Method::GET, "rest/api/3/search/approximate-count"),
        (Method::GET, "rest/api/3/issue/bulkfetch"),
        (Method::GET, "rest/api/3/jql/parse"),
        (Method::PUT, "rest/api/3/search/jql"),
        (Method::DELETE, "rest/api/3/issue/NW-1/comment"),
        (Method::GET, "rest/api/3/myself?x=1"),
        (Method::GET, "rest/api/3/field?startAt=0"),
        (Method::GET, "rest/api/3/project/search?startAt=0&bogus=1"),
        (Method::GET, "rest/api/3/project/search?startAt=0&startAt=1"),
        (
            Method::GET,
            "rest/api/3/project/NW/versions?expand=projectKey",
        ),
        (
            Method::GET,
            "rest/api/3/issue/NW-1/comment?startAt=0&fields=*all",
        ),
        (
            Method::GET,
            "rest/api/3/issue/NW-1/comment?expand=renderedBody&expand=x",
        ),
        (Method::POST, "rest/api/3/search/jql?jql=x"),
        (Method::POST, "rest/api/3/search/approximate-count?jql=x"),
        (
            Method::POST,
            "rest/api/3/jql/parse?validation=strict&validation=strict",
        ),
        (Method::POST, "rest/api/3/jql/parse?validation=strict&x=1"),
        (Method::POST, "rest/api/3/jql/parse?x=1"),
        (Method::GET, "rest/api/3/myself/"),
        (Method::GET, "rest/api/3/project/NW/versions/"),
        (Method::GET, "rest/api/3/issue/NW-1/comment/"),
        (Method::GET, "rest/api/3/project/NW%2F..%2Fx/versions"),
        (Method::GET, "rest/api/3/project/NW%2Fversions"),
        (Method::GET, "rest/api/3/issue/%2e%2e/comment"),
        (Method::GET, "rest/api/3/issue/NW-1%2Ftransitions/comment"),
        (Method::GET, "rest/api/3/issue/NW%2D1/comment"),
        (Method::GET, "rest/api/3/project/nw/versions"),
        (Method::GET, "rest/api/3/project/N-W/versions"),
        (Method::GET, "rest/api/3/project//versions"),
        (Method::GET, "rest/api/3/issue//comment"),
        (Method::GET, "rest/api/3/issue/NW-/comment"),
        (Method::GET, "rest/api/3/issue/-1/comment"),
        (Method::GET, "rest/api/3/issue/nw-1/comment"),
    ];
    for (label, site) in sites() {
        for (method, relative) in &cases {
            let plan = planned(method.clone(), &url_for(&site, relative));
            assert_refused(&plan, &site, &format!("{label} {method} {relative}"));
        }
    }
}

#[test]
fn a_request_with_another_origin_than_the_site_is_refused() {
    for (label, site) in sites() {
        let good = url_for(&site, "rest/api/3/myself");
        let host = Url::parse(&good).unwrap().host_str().unwrap().to_string();
        let variants = [
            ("fragment", format!("{good}#frag")),
            (
                "user and password",
                good.replacen("https://", "https://user:secret@", 1),
            ),
            ("user", good.replacen("https://", "https://user@", 1)),
            (
                "another port",
                good.replacen(&host, &format!("{host}:8443"), 1),
            ),
            ("another host", good.replacen(&host, "evil.example.com", 1)),
            (
                "lookalike host",
                good.replacen(&host, &format!("{host}.evil.example.com"), 1),
            ),
            (
                "host as subdomain",
                good.replacen(&host, &format!("evil.{host}"), 1),
            ),
            ("another scheme", good.replacen("https://", "http://", 1)),
            ("loopback", good.replacen(&host, "127.0.0.1", 1)),
        ];
        for (what, url) in variants {
            assert_refused(
                &planned(Method::GET, &url),
                &site,
                &format!("{label} {what}"),
            );
        }
    }
}

#[test]
fn a_request_for_the_other_token_kinds_base_is_refused() {
    let classic_url = url_for(&classic(), "rest/api/3/myself");
    let scoped_url = url_for(&scoped(), "rest/api/3/myself");
    let other_cloud =
        "https://api.atlassian.com/ex/jira/99999999-8888-7777-6666-555555555555/rest/api/3/myself";
    let without_cloud = "https://api.atlassian.com/rest/api/3/myself";
    let cloud_on_site =
        format!("https://example.atlassian.net/ex/jira/{CLOUD_ID}/rest/api/3/myself");
    let other_site = "https://other.atlassian.net/rest/api/3/myself";

    assert_refused(
        &planned(Method::GET, &classic_url),
        &scoped(),
        "classic url, scoped site",
    );
    for (what, url) in [
        ("scoped url", scoped_url.as_str()),
        ("another cloud id", other_cloud),
        ("gateway without cloud id", without_cloud),
        ("cloud path on the site", cloud_on_site.as_str()),
        ("another site", other_site),
    ] {
        assert_refused(
            &planned(Method::GET, url),
            &classic(),
            &format!("classic site, {what}"),
        );
    }
    for (what, url) in [
        ("another cloud id", other_cloud),
        ("gateway without cloud id", without_cloud),
        ("cloud path on the site", cloud_on_site.as_str()),
    ] {
        assert_refused(
            &planned(Method::GET, url),
            &scoped(),
            &format!("scoped site, {what}"),
        );
    }
}

#[test]
fn a_get_with_a_body_is_refused() {
    for (_, site) in sites() {
        let mut plan = planned(Method::GET, &url_for(&site, "rest/api/3/myself"));
        plan.body = Some(json!({ "jql": "x" }));
        assert_refused(&plan, &site, "GET with a body");
    }
}

#[test]
fn a_rest_request_without_credentials_is_refused() {
    for (_, site) in sites() {
        let mut plan = planned(Method::GET, &url_for(&site, "rest/api/3/myself"));
        plan.authenticated = false;
        assert_refused(&plan, &site, "unauthenticated REST request");
    }
}

#[test]
fn a_tenant_info_request_with_credentials_or_another_shape_is_refused() {
    for (_, site) in sites() {
        let mut with_credentials = planned(
            Method::GET,
            "https://example.atlassian.net/_edge/tenant_info",
        );
        with_credentials.authenticated = true;
        assert_refused(&with_credentials, &site, "authenticated tenant info");

        for (what, method, url) in [
            (
                "POST",
                Method::POST,
                "https://example.atlassian.net/_edge/tenant_info",
            ),
            (
                "query",
                Method::GET,
                "https://example.atlassian.net/_edge/tenant_info?x=1",
            ),
            (
                "other path",
                Method::GET,
                "https://example.atlassian.net/_edge/other",
            ),
            (
                "another host",
                Method::GET,
                "https://evil.example.com/_edge/tenant_info",
            ),
            (
                "gateway host",
                Method::GET,
                "https://api.atlassian.com/_edge/tenant_info",
            ),
        ] {
            let mut plan = planned(method, url);
            plan.authenticated = false;
            assert_refused(&plan, &site, what);
        }
    }
}

#[test]
fn a_write_is_refused_even_on_an_allowed_read_path() {
    for (_, site) in sites() {
        for mut plan in plans(&site) {
            plan.access = Access::Write;
            assert_refused(&plan, &site, &format!("write {}", plan.url));
        }
    }
}

#[test]
fn tampering_with_a_planned_request_makes_the_guard_refuse_it() {
    for (_, site) in sites() {
        for plan in plans(&site) {
            let context = plan.url.to_string();

            let mut other_method = plan.clone();
            other_method.method = if plan.method == Method::GET {
                Method::POST
            } else {
                Method::GET
            };
            assert_refused(&other_method, &site, &format!("swapped method {context}"));

            let mut extra_query = plan.clone();
            extra_query.url.query_pairs_mut().append_pair("extra", "1");
            assert_refused(&extra_query, &site, &format!("extra query {context}"));

            let mut fragment = plan.clone();
            fragment.url.set_fragment(Some("frag"));
            assert_refused(&fragment, &site, &format!("fragment {context}"));

            let mut flipped = plan.clone();
            flipped.authenticated = !plan.authenticated;
            assert_refused(&flipped, &site, &format!("flipped credentials {context}"));

            if plan.method == Method::GET {
                let mut with_body = plan.clone();
                with_body.body = Some(json!({}));
                assert_refused(&with_body, &site, &format!("GET body {context}"));
            }
        }
    }
}

fn url_special_text() -> impl Strategy<Value = String> {
    prop_oneof![
        any::<String>(),
        "[A-Za-z0-9_/?#%.&= \\-\u{e9}\u{202e}\u{200b}\n]{0,24}",
        "[A-Z][A-Z0-9_]{0,8}[/?#%.&=\\-][A-Z0-9_/?#%.&=\\-]{0,8}",
        "[A-Z][A-Z0-9_]{0,20}",
    ]
}

fn issue_text() -> impl Strategy<Value = String> {
    prop_oneof![
        url_special_text(),
        "[A-Z]{1,3}-[0-9]{1,6}",
        "[0-9]{1,22}",
        "[A-Z]{1,3}-[0-9/?#%.]{1,6}",
    ]
}

fn query_text() -> impl Strategy<Value = String> {
    prop_oneof![any::<String>(), "[ -~\n\t\"'&?#%=]{0,200}", "\\PC{0,50}",]
}

fn path_segments(url: &Url) -> Vec<String> {
    url.path_segments().unwrap().map(String::from).collect()
}

fn base_segments(site: &Site) -> Vec<String> {
    path_segments(&site.api_base().unwrap())
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .collect()
}

proptest! {
    #[test]
    fn any_text_is_a_project_key_that_stays_in_its_segment_or_is_refused(text in url_special_text()) {
        match ProjectKey::parse(&text) {
            Err(err) => prop_assert!(matches!(err, AppError::InvalidInput(_))),
            Ok(key) => {
                for (_, site) in sites() {
                    let plan = Endpoint::ProjectVersions { project: key.clone() }.plan(&site).unwrap();
                    let mut expected = base_segments(&site);
                    expected.extend(["project".into(), text.clone(), "versions".into()]);
                    prop_assert_eq!(path_segments(&plan.url), expected);
                    prop_assert!(plan.url.query().is_none());
                    prop_assert!(plan.url.fragment().is_none());
                    prop_assert!(check(&plan, &site).is_ok());
                }
            }
        }
    }

    #[test]
    fn any_text_is_an_issue_reference_that_stays_in_its_segment_or_is_refused(text in issue_text()) {
        match IssueRef::parse(&text) {
            Err(err) => prop_assert!(matches!(err, AppError::InvalidInput(_))),
            Ok(reference) => {
                for (_, site) in sites() {
                    let plan = Endpoint::IssueComments { issue: reference.clone(), start_at: 0 }
                        .plan(&site)
                        .unwrap();
                    let mut expected = base_segments(&site);
                    expected.extend(["issue".into(), text.clone(), "comment".into()]);
                    prop_assert_eq!(path_segments(&plan.url), expected);
                    prop_assert!(plan.url.fragment().is_none());
                    prop_assert_eq!(
                        plan.url.query_pairs().map(|(key, _)| key.into_owned()).collect::<Vec<_>>(),
                        ["startAt", "maxResults", "orderBy", "expand"]
                    );
                    prop_assert!(check(&plan, &site).is_ok());
                }
            }
        }
    }

    #[test]
    fn a_query_never_changes_the_url_and_only_appears_in_the_body(text in query_text()) {
        let Ok(parsed) = Jql::parse(&text) else {
            return Ok(());
        };
        for (_, site) in sites() {
            let baseline_search = search(jql("x")).plan(&site).unwrap();
            let baseline_count = Endpoint::ApproximateCount { jql: jql("x") }.plan(&site).unwrap();
            let baseline_parse = Endpoint::JqlParse { jql: jql("x") }.plan(&site).unwrap();

            let searched = search(parsed.clone()).plan(&site).unwrap();
            prop_assert_eq!(&searched.url, &baseline_search.url);
            prop_assert_eq!(searched.body.as_ref().unwrap()["jql"].as_str(), Some(text.as_str()));
            prop_assert!(check(&searched, &site).is_ok());

            let counted = Endpoint::ApproximateCount { jql: parsed.clone() }.plan(&site).unwrap();
            prop_assert_eq!(&counted.url, &baseline_count.url);
            prop_assert_eq!(counted.body.clone(), Some(json!({ "jql": text })));
            prop_assert!(check(&counted, &site).is_ok());

            let parsed_plan = Endpoint::JqlParse { jql: parsed.clone() }.plan(&site).unwrap();
            prop_assert_eq!(&parsed_plan.url, &baseline_parse.url);
            prop_assert_eq!(parsed_plan.body.clone(), Some(json!({ "queries": [text] })));
            prop_assert!(check(&parsed_plan, &site).is_ok());
        }
    }
}

#[test]
fn queries_with_quotes_symbols_line_breaks_and_10000_characters_leave_the_url_alone() {
    let longest = "a".repeat(10_000);
    for (_, site) in sites() {
        let baseline = search(jql("x")).plan(&site).unwrap();
        for text in [
            r#"summary ~ "a\"b" AND text ~ 'c'"#,
            "a&b=c?d#e",
            "project = NW\nORDER BY created",
            "../../issue/1/transitions",
            "%2e%2e/%2F",
            longest.as_str(),
        ] {
            let plan = search(jql(text)).plan(&site).unwrap();
            assert_eq!(plan.url, baseline.url, "{text:?}");
            assert_eq!(plan.body.as_ref().unwrap()["jql"], json!(text));
        }
    }
}
