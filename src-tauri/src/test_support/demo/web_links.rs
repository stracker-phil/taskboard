//! Web links (Jira's remote links) on a few demo issues: pull requests on GitHub, GitLab and Bitbucket, and a
//! document, shaped like `GET /issue/{id}/remotelink`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::SITE_URL;
use super::render::issue_id;

const FIRST_LINK_ID: u64 = 70_001;

struct LinkSpec {
    issue: &'static str,
    url: &'static str,
    title: &'static str,
    application: Option<(&'static str, &'static str)>,
    relationship: &'static str,
}

const GITHUB: Option<(&str, &str)> = Some(("com.github.integration", "GitHub"));

const LINKS: &[LinkSpec] = &[
    LinkSpec {
        issue: "NW-1042",
        url: "https://github.com/example/shop/pull/481",
        title: "Validate DACH postcodes",
        application: GITHUB,
        relationship: "mentioned in",
    },
    LinkSpec {
        issue: "NW-1042",
        url: "https://docs.example.com/checkout/postcode-rules",
        title: "Postcode rules for the checkout",
        application: None,
        relationship: "relates to",
    },
    LinkSpec {
        issue: "NW-1038",
        url: "https://github.com/example/shop/pull/476",
        title: "Refetch shipping rates on address change",
        application: GITHUB,
        relationship: "mentioned in",
    },
    LinkSpec {
        issue: "NW-1047",
        url: "https://gitlab.example.com/shop/store-api/-/merge_requests/58",
        title: "Cart totals from the Store API",
        application: Some(("com.gitlab", "GitLab")),
        relationship: "mentioned in",
    },
    LinkSpec {
        issue: "NW-1044",
        url: "https://bitbucket.org/example/shop-media/pull-requests/12",
        title: "Lazy-load the product gallery",
        application: Some(("com.atlassian.bitbucket", "Bitbucket")),
        relationship: "mentioned in",
    },
];

/// Issue id to its remote links, in Jira's order.
pub fn remote_links() -> BTreeMap<String, Vec<Value>> {
    let mut by_issue: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for (offset, spec) in (0_u64..).zip(LINKS) {
        let id = FIRST_LINK_ID + offset;
        let mut link = json!({
            "id": id,
            "self": format!("{SITE_URL}/rest/api/3/issue/{}/remotelink/{id}", spec.issue),
            "relationship": spec.relationship,
            "object": {
                "url": spec.url,
                "title": spec.title,
                "icon": { "url16x16": "https://example.com/favicon.ico", "title": spec.title },
            },
        });
        if let Some((kind, name)) = spec.application {
            link["application"] = json!({ "type": kind, "name": name });
        }
        by_issue.entry(issue_id(spec.issue)).or_default().push(link);
    }
    by_issue
}
