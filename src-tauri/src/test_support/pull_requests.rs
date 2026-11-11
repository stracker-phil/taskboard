//! Demo pull request statuses for the debug build and tests: the demo site's GitHub pull requests, answered
//! without running `gh` (ADR 0061).

use crate::pull_requests::{
    Checks, ChecksState, GithubPr, PrState, PrStatus, ReviewState, StatusAnswer, StatusSource,
};

pub struct FakePullRequests;

impl StatusSource for FakePullRequests {
    fn status<'a>(&'a self, pr: &'a GithubPr) -> StatusAnswer<'a> {
        let answer = demo_status(pr).ok_or_else(|| "GitHub has no such pull request".to_owned());
        Box::pin(async move { answer })
    }
}

fn demo_status(pr: &GithubPr) -> Option<PrStatus> {
    if pr.owner != "example" || pr.repo != "shop" {
        return None;
    }
    let (title, state, checks, review) = match pr.number {
        481 => (
            "Validate DACH postcodes",
            PrState::Open,
            (ChecksState::Pending, 9, 12),
            ReviewState::Waiting,
        ),
        476 => (
            "Refetch shipping rates on address change",
            PrState::Merged,
            (ChecksState::Passing, 12, 12),
            ReviewState::Approved,
        ),
        _ => return None,
    };
    let (state_of_checks, passed, total) = checks;
    Some(PrStatus {
        title: title.into(),
        state,
        checks: Checks {
            state: state_of_checks,
            passed,
            total,
        },
        review,
    })
}
