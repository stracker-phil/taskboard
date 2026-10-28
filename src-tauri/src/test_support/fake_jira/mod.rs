//! A loopback Jira Cloud for tests and (later) the debug build: serves a `JiraData` set over plain HTTP on
//! `127.0.0.1`, records every request, and has knobs for failures, paging and changes to the data.

mod http;
pub mod jql;
mod routes;

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::watch;
use url::Url;

pub use http::Recorded;
use http::{Response, read_request, write_response};
use routes::{Content, is_issue, route};

use crate::jira::{JiraClient, Limiter, Site, TokenKind};
use crate::test_support::FakeSecrets;
use crate::test_support::demo::{self, DEMO_NOW, JiraData, ME_EMAIL};

/// The API token the fake's clients hold; the fake accepts any token.
pub const DEMO_TOKEN: &str = "demo-api-token";

pub struct FakeJira {
    port: u16,
    shared: Arc<Shared>,
}

struct Shared {
    content: Mutex<Content>,
    /// Answered instead of the route, oldest first.
    canned: Mutex<VecDeque<Response>>,
    latency: Mutex<Duration>,
    requests: Mutex<Vec<Recorded>>,
    online: watch::Sender<bool>,
    listening: watch::Sender<bool>,
}

impl FakeJira {
    /// Serves `data` on a free port with `now` as the clock for relative JQL dates (`-90d`).
    pub async fn start(data: JiraData, now: DateTime<Utc>) -> Self {
        Self::start_on(0, data, now).await.expect("bind loopback")
    }

    /// Serves on `port` (0: any free one).
    pub async fn start_on(port: u16, data: JiraData, now: DateTime<Utc>) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port)).await?;
        let port = listener.local_addr()?.port();
        let shared = Arc::new(Shared {
            content: Mutex::new(Content {
                data,
                now,
                hidden: HashSet::new(),
                page_size: None,
                max_results_cap: 100,
            }),
            canned: Mutex::new(VecDeque::new()),
            latency: Mutex::new(Duration::ZERO),
            requests: Mutex::new(Vec::new()),
            online: watch::channel(true).0,
            listening: watch::channel(true).0,
        });
        tokio::spawn(serve(listener, port, shared.clone()));
        Ok(Self { port, shared })
    }

    /// The demo site at its fixed date.
    pub async fn start_demo() -> Self {
        let now = DEMO_NOW.parse().expect("DEMO_NOW is an RFC 3339 instant");
        Self::start(demo::demo(), now).await
    }

    pub fn url(&self) -> Url {
        Url::parse(&format!("http://127.0.0.1:{}/", self.port)).expect("a loopback URL")
    }

    /// The demo user's connection to this fake.
    pub fn site(&self, token_kind: TokenKind) -> Site {
        let cloud_id = self.content().data.cloud_id.clone();
        Site::new(self.url().as_str(), ME_EMAIL, token_kind, Some(cloud_id))
            .expect("the fake's URL is a valid site")
    }

    /// A client for `site(token_kind)` holding `DEMO_TOKEN`, with a rate limit tests don't notice.
    pub fn client(&self, token_kind: TokenKind) -> JiraClient {
        JiraClient::new(
            self.site(token_kind),
            Arc::new(FakeSecrets::with(ME_EMAIL, DEMO_TOKEN)),
            Arc::new(Limiter::new(1000, 1000)),
        )
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.shared.requests.lock().unwrap().clone()
    }

    /// The paths asked, in order, without the query (`/rest/api/3/search/jql`).
    pub fn paths(&self) -> Vec<String> {
        self.requests().into_iter().map(|r| r.path).collect()
    }

    /// Search pages of `size` issues, whatever `maxResults` asks for.
    pub fn set_page_size(&self, size: usize) {
        self.content().page_size = Some(size);
    }

    /// The most issues a search page holds when `maxResults` asks for more.
    pub fn cap_max_results(&self, cap: usize) {
        self.content().max_results_cap = cap;
    }

    /// The next `count` requests get `status` (with `Retry-After` when given) instead of their answer.
    pub fn fail_next(&self, count: usize, status: u16, retry_after: Option<&str>) {
        for _ in 0..count {
            let mut response = Response::json(
                status,
                json!({ "errorMessages": [format!("HTTP {status}")] }),
            );
            if let Some(value) = retry_after {
                response.headers.push(("Retry-After".into(), value.into()));
                response
                    .headers
                    .push(("RateLimit-Reason".into(), "jira-burst-based".into()));
            }
            self.shared.canned.lock().unwrap().push_back(response);
        }
    }

    /// The next request gets exactly this answer.
    pub fn answer_next(&self, status: u16, body: Value) {
        self.shared
            .canned
            .lock()
            .unwrap()
            .push_back(Response::json(status, body));
    }

    /// The next request gets exactly this answer: any status, headers, content type and body.
    pub fn answer_raw(
        &self,
        status: u16,
        headers: &[(&str, &str)],
        content_type: &str,
        body: String,
    ) {
        let mut response = Response::raw(status, content_type, body);
        response.headers = headers
            .iter()
            .map(|(name, value)| ((*name).into(), (*value).into()))
            .collect();
        self.shared.canned.lock().unwrap().push_back(response);
    }

    pub fn set_latency(&self, latency: Duration) {
        *self.shared.latency.lock().unwrap() = latency;
    }

    /// Closes the port, so connections are refused, until `go_online`.
    pub async fn go_offline(&self) {
        self.shared.online.send_replace(false);
        let _ = self.shared.listening.subscribe().wait_for(|on| !on).await;
    }

    pub async fn go_online(&self) {
        self.shared.online.send_replace(true);
        let _ = self.shared.listening.subscribe().wait_for(|on| *on).await;
    }

    /// Moves the fake's clock; edits are stamped with it.
    pub fn advance(&self, by: Duration) {
        let mut content = self.content();
        content.now += chrono::Duration::from_std(by).expect("a short duration");
    }

    /// Serves `data` from now on.
    pub fn replace_data(&self, data: JiraData) {
        self.content().data = data;
    }

    /// Sets the fake's clock.
    pub fn set_now(&self, now: DateTime<Utc>) {
        self.content().now = now;
    }

    /// Adds a comment by `author` (a user object) and stamps `updated`.
    pub fn add_comment(&self, id_or_key: &str, author: Value, text: &str) {
        let mut content = self.content();
        let now = content.now;
        let stamp = jira_time(now);
        let next_id = content.data.comments.values().flatten().count() + 900_000;
        let issue = issue_mut(&mut content.data, id_or_key);
        let comment = json!({
            "id": next_id.to_string(),
            "author": author,
            "body": { "type": "doc", "version": 1, "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": text }] }
            ] },
            "renderedBody": format!("<p>{text}</p>"),
            "created": stamp,
            "updated": stamp,
        });
        push_comment(&mut issue["fields"]["comment"], comment.clone());
        let mut rendered = comment.clone();
        rendered["body"] = json!(format!("<p>{text}</p>"));
        push_comment(&mut issue["renderedFields"]["comment"], rendered);
        stamp_issue(issue, now);
        let id = issue["id"].as_str().unwrap_or_default().to_string();
        content.data.comments.entry(id).or_default().push(comment);
    }

    /// Adds an issue to `project_key`, shaped like `template`, with a new id, key and summary.
    pub fn add_issue(&self, template: &str, project_key: &str, summary: &str) -> String {
        let mut content = self.content();
        let now = content.now;
        let mut issue = issue_mut(&mut content.data, template).clone();
        let id = content
            .data
            .issues
            .iter()
            .filter_map(|i| i["id"].as_str()?.parse::<u64>().ok())
            .max()
            .unwrap_or(0)
            + 1;
        let key = next_key(&content.data.issues, project_key);
        issue["id"] = json!(id.to_string());
        issue["key"] = json!(key);
        issue["fields"]["summary"] = json!(summary);
        issue["fields"]["created"] = json!(jira_time(now));
        issue["fields"]["comment"] =
            json!({ "comments": [], "maxResults": 0, "total": 0, "startAt": 0 });
        issue["renderedFields"]["comment"] = json!({ "comments": [] });
        stamp_issue(&mut issue, now);
        content.data.issues.push(issue);
        key
    }

    /// Changes an issue's fields and stamps `updated` with the fake's clock.
    pub fn edit_issue(&self, id_or_key: &str, edit: impl FnOnce(&mut Value)) {
        let mut content = self.content();
        let now = content.now;
        let issue = issue_mut(&mut content.data, id_or_key);
        edit(&mut issue["fields"]);
        stamp_issue(issue, now);
    }

    /// Moves an issue to another project: same id, a new key there.
    pub fn move_issue(&self, id_or_key: &str, project_key: &str) {
        let mut content = self.content();
        let now = content.now;
        let project = content
            .data
            .projects
            .iter()
            .find(|p| p["key"] == project_key)
            .cloned()
            .unwrap_or_else(|| panic!("no project {project_key} in the fake"));
        let key = next_key(&content.data.issues, project_key);
        let issue = issue_mut(&mut content.data, id_or_key);
        issue["key"] = json!(key);
        issue["fields"]["project"] = project;
        stamp_issue(issue, now);
    }

    pub fn delete_issue(&self, id_or_key: &str) {
        let mut content = self.content();
        let id = issue_mut(&mut content.data, id_or_key)["id"].clone();
        content.data.issues.retain(|issue| issue["id"] != id);
    }

    /// The user loses access: the issue still exists but no endpoint shows it.
    pub fn revoke_access(&self, id_or_key: &str) {
        let mut content = self.content();
        let id = issue_mut(&mut content.data, id_or_key)["id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        content.hidden.insert(id);
    }

    fn content(&self) -> std::sync::MutexGuard<'_, Content> {
        self.shared.content.lock().unwrap()
    }
}

fn issue_mut<'a>(data: &'a mut JiraData, id_or_key: &str) -> &'a mut Value {
    data.issues
        .iter_mut()
        .find(|issue| is_issue(issue, id_or_key))
        .unwrap_or_else(|| panic!("no issue {id_or_key} in the fake"))
}

/// The key after the project's highest issue number.
fn next_key(issues: &[Value], project_key: &str) -> String {
    let prefix = format!("{project_key}-");
    let highest = issues
        .iter()
        .filter_map(|issue| {
            issue["key"]
                .as_str()?
                .strip_prefix(&prefix)?
                .parse::<u32>()
                .ok()
        })
        .max()
        .unwrap_or(0);
    format!("{prefix}{}", highest + 1)
}

fn jira_time(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%S%.3f+0000").to_string()
}

fn push_comment(list: &mut Value, comment: Value) {
    if !list["comments"].is_array() {
        *list = json!({ "comments": [], "maxResults": 0, "total": 0, "startAt": 0 });
    }
    if let Some(comments) = list["comments"].as_array_mut() {
        comments.push(comment);
    }
    let total = list["comments"].as_array().map_or(0, Vec::len);
    list["total"] = json!(total);
    list["maxResults"] = json!(total);
}

/// Sets `updated` the way Jira writes it.
fn stamp_issue(issue: &mut Value, now: DateTime<Utc>) {
    issue["fields"]["updated"] = json!(jira_time(now));
}

/// Accepts while online; going offline drops the listener so the port refuses connections.
async fn serve(listener: TcpListener, port: u16, shared: Arc<Shared>) {
    let mut listener = Some(listener);
    let mut online = shared.online.subscribe();
    loop {
        let Some(open) = &listener else {
            if online.wait_for(|on| *on).await.is_err() {
                return;
            }
            let Ok(rebound) = TcpListener::bind(("127.0.0.1", port)).await else {
                return;
            };
            listener = Some(rebound);
            shared.listening.send_replace(true);
            continue;
        };
        tokio::select! {
            accepted = open.accept() => {
                if let Ok((socket, _)) = accepted {
                    tokio::spawn(answer(socket, shared.clone()));
                }
            }
            changed = online.wait_for(|on| !on) => {
                if changed.is_err() {
                    return;
                }
                listener = None;
                shared.listening.send_replace(false);
            }
        }
    }
}

async fn answer(mut socket: tokio::net::TcpStream, shared: Arc<Shared>) {
    let Some(request) = read_request(&mut socket).await else {
        return;
    };
    shared.requests.lock().unwrap().push(request.clone());
    let latency = *shared.latency.lock().unwrap();
    if !latency.is_zero() {
        tokio::time::sleep(latency).await;
    }
    let canned = shared.canned.lock().unwrap().pop_front();
    let response = canned.unwrap_or_else(|| route(&shared.content.lock().unwrap(), &request));
    write_response(&mut socket, response).await;
}
