//! An issue's pull requests and their GitHub status (ADR 0061).

pub mod commands;
mod gh;
mod github;
mod status;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use futures_util::future::join_all;
use serde::Serialize;
use ts_rs::TS;

use crate::db::Db;
use crate::error::AppResult;
use crate::issues::{WebLink, WebLinkKind, issue_web_links};

pub use gh::{GhCli, Problem, StatusAnswer, StatusSource};
pub use github::GithubPr;
pub use status::{Checks, ChecksState, PrState, PrStatus, ReviewState};

/// How long a pull request's status is kept before `gh` is asked again, and how often an open issue asks
/// again while a pull request isn't merged.
const FRESH_FOR: Duration = Duration::minutes(5);

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub id: String,
    pub url: String,
    pub number: u32,
    /// `owner/repo` on GitHub, the repository's name elsewhere.
    pub repo: String,
    /// GitHub's title, else the link's.
    pub title: String,
    pub openable: bool,
    /// What GitHub says; none for GitLab and Bitbucket, and when `problem` says why not.
    pub status: Option<PrStatus>,
    pub problem: Option<Problem>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct IssuePullRequests {
    pub items: Vec<PullRequest>,
    /// When to sync again: while a GitHub pull request isn't merged, the time until its status is due (0 when it
    /// is due now or was never fetched); none when nothing needs asking.
    #[ts(type = "number | null")]
    pub refresh_in_ms: Option<u64>,
}

struct Entry {
    fetched_at: DateTime<Utc>,
    answer: Result<PrStatus, Problem>,
}

/// Pull request statuses, kept in memory for `FRESH_FOR` (failures too, so a missing `gh` isn't asked on
/// every move).
pub struct PullRequestStatuses {
    source: Arc<dyn StatusSource>,
    cache: Mutex<HashMap<GithubPr, Entry>>,
}

impl PullRequestStatuses {
    pub fn new(source: Arc<dyn StatusSource>) -> Self {
        Self {
            source,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// The issue's pull requests from its cached web links, in their order, with the statuses kept in memory;
    /// with `fetch`, the due ones are fetched first.
    pub async fn of_issue(
        &self,
        db: &Db,
        id: String,
        now: DateTime<Utc>,
        fetch: bool,
    ) -> AppResult<IssuePullRequests> {
        let links: Vec<WebLink> = issue_web_links(db, id)
            .await?
            .into_iter()
            .filter(|link| link.kind == WebLinkKind::PullRequest)
            .collect();
        let github: Vec<Option<GithubPr>> = links
            .iter()
            .map(|link| GithubPr::parse(&link.url))
            .collect();
        if fetch {
            self.fetch_due(&github, now).await;
        }
        let cache = self.lock();
        let refresh_at = github
            .iter()
            .flatten()
            .filter_map(|pr| due_at(cache.get(pr), now))
            .min();
        let items = links
            .into_iter()
            .zip(github)
            .filter_map(|(link, pr)| {
                let entry = pr.as_ref().and_then(|pr| cache.get(pr));
                pull_request(link, pr.as_ref(), entry)
            })
            .collect();
        let refresh_in_ms = refresh_at.map(|at| (at - now).num_milliseconds().max(0) as u64);
        Ok(IssuePullRequests {
            items,
            refresh_in_ms,
        })
    }

    /// Fetches the statuses older than `FRESH_FOR` at `now`, or never fetched, all at once.
    async fn fetch_due(&self, prs: &[Option<GithubPr>], now: DateTime<Utc>) {
        let due: Vec<&GithubPr> = prs
            .iter()
            .flatten()
            .filter(|pr| self.is_due(pr, now))
            .collect();
        let answers = join_all(due.iter().map(|pr| self.source.status(pr))).await;
        let mut cache = self.lock();
        for (pr, answer) in due.into_iter().zip(answers) {
            if let Err(problem) = &answer {
                log::info!("No status for {}#{}: {problem}", pr.repo_path(), pr.number);
            }
            cache.insert(
                pr.clone(),
                Entry {
                    fetched_at: now,
                    answer,
                },
            );
        }
    }

    fn is_due(&self, pr: &GithubPr, now: DateTime<Utc>) -> bool {
        self.lock()
            .get(pr)
            .is_none_or(|entry| now - entry.fetched_at >= FRESH_FOR)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<GithubPr, Entry>> {
        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// When a GitHub pull request's status is due again; never once it is merged.
fn due_at(entry: Option<&Entry>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match entry {
        None => Some(now),
        Some(Entry {
            answer: Ok(status), ..
        }) if status.state == PrState::Merged => None,
        Some(entry) => Some(entry.fetched_at + FRESH_FOR),
    }
}

/// None for a link whose number can't be read (it isn't shown as a pull request then).
fn pull_request(
    link: WebLink,
    github: Option<&GithubPr>,
    entry: Option<&Entry>,
) -> Option<PullRequest> {
    let (repo, number) = match github {
        Some(pr) => (pr.repo_path(), pr.number),
        None => repo_and_number(&link)?,
    };
    let (status, problem) = match entry.map(|entry| &entry.answer) {
        Some(Ok(status)) => (Some(status.clone()), None),
        Some(Err(problem)) => (None, Some(problem.clone())),
        None => (None, None),
    };
    let title = status
        .as_ref()
        .map(|status| status.title.clone())
        .filter(|title| !title.trim().is_empty())
        .or_else(|| (link.title != link.url).then(|| link.title.clone()))
        .unwrap_or_else(|| repo.clone());
    Some(PullRequest {
        id: link.id,
        url: link.url,
        number,
        repo,
        title,
        openable: link.openable,
        status,
        problem,
    })
}

/// From `repo#58`, the classified reference of a GitLab or Bitbucket link.
fn repo_and_number(link: &WebLink) -> Option<(String, u32)> {
    let (repo, number) = link
        .reference
        .as_deref()
        .unwrap_or(&link.title)
        .rsplit_once('#')?;
    Some((repo.to_owned(), number.parse().ok()?))
}

/// The release build asks `gh`; the debug build answers from demo statuses and never runs it.
pub fn default_source() -> Arc<dyn StatusSource> {
    #[cfg(debug_assertions)]
    {
        Arc::new(crate::test_support::FakePullRequests)
    }
    #[cfg(not(debug_assertions))]
    {
        Arc::new(GhCli)
    }
}

#[cfg(test)]
mod tests;
