//! Asks the GitHub CLI for a pull request's status (ADR 0061); Taskboard holds no GitHub token.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use super::github::GithubPr;
use super::status::{GH_FIELDS, GhView, PrStatus, summarize};

/// Why a pull request has no status, worded for its row's tooltip.
pub type Problem = String;

pub type StatusAnswer<'a> = Pin<Box<dyn Future<Output = Result<PrStatus, Problem>> + Send + 'a>>;

/// Where pull request statuses come from: `gh` in the release build, demo statuses in the debug build.
pub trait StatusSource: Send + Sync {
    fn status<'a>(&'a self, pr: &'a GithubPr) -> StatusAnswer<'a>;
}

/// Homebrew's places: an app opened from the Finder doesn't get the shell's `PATH`.
const GH_PATHS: &[&str] = &["/opt/homebrew/bin/gh", "/usr/local/bin/gh"];
const MAX_REASON_CHARS: usize = 200;
const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Default)]
pub struct GhCli;

impl StatusSource for GhCli {
    fn status<'a>(&'a self, pr: &'a GithubPr) -> StatusAnswer<'a> {
        Box::pin(async move {
            let gh = GH_PATHS
                .iter()
                .map(Path::new)
                .find(|path| path.is_file())
                .ok_or_else(|| "The GitHub CLI (gh) isn't installed".to_owned())?;
            let output = run(gh, pr).await?;
            let view: GhView = serde_json::from_slice(&output)
                .map_err(|e| format!("gh answered something unexpected: {e}"))?;
            Ok(summarize(view))
        })
    }
}

async fn run(gh: &Path, pr: &GithubPr) -> Result<Vec<u8>, Problem> {
    let child = Command::new(gh)
        .args([
            "pr",
            "view",
            &pr.number.to_string(),
            "--repo",
            &pr.repo_path(),
            "--json",
            GH_FIELDS,
        ])
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("gh couldn't start: {e}"))?;
    let output = tokio::time::timeout(TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "gh took too long to answer".to_owned())?
        .map_err(|e| format!("gh failed: {e}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(problem(&String::from_utf8_lossy(&output.stderr)))
    }
}

/// gh's complaint as a short reason.
fn problem(stderr: &str) -> Problem {
    if stderr.contains("gh auth login") {
        return "The GitHub CLI isn't signed in (gh auth login)".into();
    }
    if stderr.contains("Could not resolve to a PullRequest") {
        return "GitHub has no such pull request".into();
    }
    if stderr.contains("Could not resolve to a Repository") {
        return "GitHub has no such repository, or gh can't see it".into();
    }
    let line = stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("gh failed");
    line.chars().take(MAX_REASON_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gh_asking_to_sign_in_says_it_is_not_signed_in() {
        assert_eq!(
            problem("To get started with GitHub CLI, please run:  gh auth login\n"),
            "The GitHub CLI isn't signed in (gh auth login)"
        );
    }

    #[test]
    fn a_missing_pull_request_or_repository_gets_its_own_reason() {
        for (stderr, expected) in [
            (
                "GraphQL: Could not resolve to a PullRequest with the number of 9. (repository.pullRequest)",
                "GitHub has no such pull request",
            ),
            (
                "GraphQL: Could not resolve to a Repository with the name 'example/shop'.",
                "GitHub has no such repository, or gh can't see it",
            ),
        ] {
            assert_eq!(problem(stderr), expected, "{stderr}");
        }
    }

    #[test]
    fn any_other_complaint_is_its_first_non_empty_line_trimmed() {
        assert_eq!(
            problem("\n   \n  connection reset by peer  \nsecond line\n"),
            "connection reset by peer"
        );
    }

    #[test]
    fn a_long_complaint_is_cut_at_two_hundred_characters() {
        let stderr = format!("{}\nnext", "x".repeat(300));

        assert_eq!(problem(&stderr), "x".repeat(200));
    }

    #[test]
    fn a_complaint_is_cut_by_characters_not_bytes() {
        let stderr = "ü".repeat(250);

        assert_eq!(problem(&stderr), "ü".repeat(200));
    }

    #[test]
    fn no_complaint_says_gh_failed() {
        for stderr in ["", "  \n\t\n"] {
            assert_eq!(problem(stderr), "gh failed", "{stderr:?}");
        }
    }
}
