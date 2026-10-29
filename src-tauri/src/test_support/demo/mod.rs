//! The demo Jira site: made-up people, projects and issues shaped exactly like Jira Cloud's REST API v3
//! responses. The fake Jira serves it to tests and to the debug build.

mod adf;
mod catalog;
mod clock;
mod fields;
mod issues_more;
mod issues_nw;
mod people;
mod render;
mod spec;
mod synthetic;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde_json::Value;

pub use catalog::{SpaceHint, demo_spaces};
pub use fields::{EPIC_COLOR, REVIEWER, SPRINT, STORY_POINTS};
pub use people::ME_EMAIL;
pub use synthetic::synthetic;

use clock::Clock;
use spec::{CommentSpec, IssueSpec};

/// The fixed "now" the demo tables are written against (Wednesday, 12:00 in Berlin).
pub const DEMO_NOW: &str = "2026-10-28T11:00:00Z";
/// The placeholder site URL in `self` links.
pub const SITE_URL: &str = "https://example.atlassian.net";
pub const CLOUD_ID: &str = "3f9c2e71-5b8a-4d06-9e1f-7a2b4c6d8e01";

/// A whole Jira site's content, as Jira's REST API v3 shapes it.
#[derive(Clone, Debug)]
pub struct JiraData {
    /// `GET /myself`
    pub myself: Value,
    /// `cloudId` of `/_edge/tenant_info`
    pub cloud_id: String,
    pub users: Vec<Value>,
    /// `GET /field`
    pub fields: Vec<Value>,
    /// `GET /project/search` `values`
    pub projects: Vec<Value>,
    /// `GET /status`
    pub statuses: Vec<Value>,
    /// Project key to `GET /project/{key}/versions`
    pub versions: BTreeMap<String, Vec<Value>>,
    /// Full issue objects as `POST /search/jql` returns them with `expand=renderedFields`
    pub issues: Vec<Value>,
    /// Issue id to `GET /issue/{key}/comment?expand=renderedBody` `comments`, oldest first
    pub comments: BTreeMap<String, Vec<Value>>,
}

/// The mockup's site, with every time as it was at [`DEMO_NOW`].
pub fn demo() -> JiraData {
    demo_at(clock::demo_now())
}

/// The mockup's site with every time shifted so that it lies as far before `now` as before [`DEMO_NOW`].
pub fn demo_at(now: DateTime<Utc>) -> JiraData {
    let mut specs = issues_nw::issues();
    specs.extend(issues_more::issues());
    let mut comments = issues_nw::comments();
    comments.extend(issues_more::comments());
    site(&specs, &comments, &Clock::new(now))
}

fn site(specs: &[IssueSpec], comments: &[CommentSpec], clock: &Clock) -> JiraData {
    let (issues, comments) = render::render(specs, comments, clock);
    let versions = catalog::PROJECTS
        .iter()
        .map(|project| {
            let list = catalog::VERSIONS
                .iter()
                .filter(|version| version.project == project.key)
                .map(|version| catalog::version_json(version, clock))
                .collect();
            (project.key.to_owned(), list)
        })
        .collect();
    JiraData {
        myself: people::myself(),
        cloud_id: CLOUD_ID.to_owned(),
        users: people::users(),
        fields: fields::fields(),
        projects: catalog::projects(),
        statuses: catalog::statuses(),
        versions,
        issues,
        comments,
    }
}
