//! Sums up a `gh pr view --json` answer for one row. Pure, so its rules are tested alone.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Every field `GhView` reads must be listed here.
pub const GH_FIELDS: &str =
    "title,state,isDraft,reviewDecision,reviewRequests,latestReviews,statusCheckRollup";

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum PrState {
    Open,
    Draft,
    Merged,
    Closed,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum ChecksState {
    Passing,
    Failing,
    Pending,
    None,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Checks {
    pub state: ChecksState,
    /// Finished without failing (skipped and neutral count as passed).
    pub passed: u32,
    pub total: u32,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    /// A review is required or requested and none approved it yet.
    Waiting,
    /// No review asked for and none given.
    None,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct PrStatus {
    pub title: String,
    pub state: PrState,
    pub checks: Checks,
    pub review: ReviewState,
}

/// `gh pr view --json` with `GH_FIELDS`.
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GhView {
    title: String,
    state: String,
    is_draft: bool,
    review_decision: Option<String>,
    review_requests: Vec<serde::de::IgnoredAny>,
    latest_reviews: Vec<GhReview>,
    status_check_rollup: Vec<GhCheck>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct GhReview {
    state: String,
}

/// A check run (`status`, `conclusion`) or a commit status (`state`).
#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct GhCheck {
    status: Option<String>,
    conclusion: Option<String>,
    state: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Outcome {
    Passed,
    Failed,
    Pending,
}

pub fn summarize(view: GhView) -> PrStatus {
    PrStatus {
        state: state(&view),
        checks: checks(&view.status_check_rollup),
        review: review(&view),
        title: view.title,
    }
}

fn state(view: &GhView) -> PrState {
    match view.state.as_str() {
        "MERGED" => PrState::Merged,
        "CLOSED" => PrState::Closed,
        _ if view.is_draft => PrState::Draft,
        _ => PrState::Open,
    }
}

fn outcome(check: &GhCheck) -> Outcome {
    if let Some(state) = check.state.as_deref() {
        return match state {
            "SUCCESS" => Outcome::Passed,
            "FAILURE" | "ERROR" => Outcome::Failed,
            _ => Outcome::Pending,
        };
    }
    if check.status.as_deref() != Some("COMPLETED") {
        return Outcome::Pending;
    }
    match check.conclusion.as_deref() {
        Some("SUCCESS" | "NEUTRAL" | "SKIPPED") => Outcome::Passed,
        _ => Outcome::Failed,
    }
}

fn checks(rollup: &[GhCheck]) -> Checks {
    let outcomes: Vec<Outcome> = rollup.iter().map(outcome).collect();
    let count = |wanted: Outcome| outcomes.iter().filter(|o| **o == wanted).count() as u32;
    let state = if outcomes.is_empty() {
        ChecksState::None
    } else if count(Outcome::Failed) > 0 {
        ChecksState::Failing
    } else if count(Outcome::Pending) > 0 {
        ChecksState::Pending
    } else {
        ChecksState::Passing
    };
    Checks {
        state,
        passed: count(Outcome::Passed),
        total: outcomes.len() as u32,
    }
}

/// GitHub's review decision when branch protection asks for one; else the latest reviews and open requests.
fn review(view: &GhView) -> ReviewState {
    match view.review_decision.as_deref() {
        Some("APPROVED") => return ReviewState::Approved,
        Some("CHANGES_REQUESTED") => return ReviewState::ChangesRequested,
        Some("REVIEW_REQUIRED") => return ReviewState::Waiting,
        _ => {}
    }
    let given = |state: &str| view.latest_reviews.iter().any(|r| r.state == state);
    if given("CHANGES_REQUESTED") {
        ReviewState::ChangesRequested
    } else if !view.review_requests.is_empty() {
        ReviewState::Waiting
    } else if given("APPROVED") {
        ReviewState::Approved
    } else {
        ReviewState::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn summary_of(view: Value) -> PrStatus {
        summarize(serde_json::from_value(view).unwrap())
    }

    fn run(status: &str, conclusion: Option<&str>) -> Value {
        json!({ "__typename": "CheckRun", "name": "build", "status": status, "conclusion": conclusion })
    }

    fn context(state: &str) -> Value {
        json!({ "__typename": "StatusContext", "context": "ci/legacy", "state": state })
    }

    fn checks_of(rollup: Vec<Value>) -> Checks {
        summary_of(json!({ "statusCheckRollup": rollup })).checks
    }

    fn review_of(view: Value) -> ReviewState {
        summary_of(view).review
    }

    #[test]
    fn the_state_follows_gh_and_a_draft_only_counts_while_open() {
        for (state, is_draft, expected) in [
            ("OPEN", false, PrState::Open),
            ("OPEN", true, PrState::Draft),
            ("MERGED", false, PrState::Merged),
            ("MERGED", true, PrState::Merged),
            ("CLOSED", false, PrState::Closed),
            ("CLOSED", true, PrState::Closed),
        ] {
            let status = summary_of(json!({ "state": state, "isDraft": is_draft }));

            assert_eq!(status.state, expected, "{state} draft={is_draft}");
        }
    }

    #[test]
    fn the_title_comes_through() {
        let status = summary_of(json!({ "title": "Validate DACH postcodes", "state": "OPEN" }));

        assert_eq!(status.title, "Validate DACH postcodes");
    }

    #[test]
    fn a_pull_request_without_checks_has_none() {
        for view in [json!({}), json!({ "statusCheckRollup": [] })] {
            assert_eq!(
                summary_of(view).checks,
                Checks {
                    state: ChecksState::None,
                    passed: 0,
                    total: 0
                }
            );
        }
    }

    #[test]
    fn finished_checks_that_did_not_fail_are_passing_and_all_count_as_passed() {
        let checks = checks_of(vec![
            run("COMPLETED", Some("SUCCESS")),
            run("COMPLETED", Some("NEUTRAL")),
            run("COMPLETED", Some("SKIPPED")),
            context("SUCCESS"),
        ]);

        assert_eq!(
            checks,
            Checks {
                state: ChecksState::Passing,
                passed: 4,
                total: 4
            }
        );
    }

    #[test]
    fn a_failed_check_makes_the_checks_failing() {
        let failures = [
            run("COMPLETED", Some("FAILURE")),
            run("COMPLETED", Some("TIMED_OUT")),
            run("COMPLETED", Some("CANCELLED")),
            run("COMPLETED", Some("ACTION_REQUIRED")),
            context("FAILURE"),
            context("ERROR"),
        ];
        for failure in failures {
            let checks = checks_of(vec![failure.clone()]);

            assert_eq!(
                checks,
                Checks {
                    state: ChecksState::Failing,
                    passed: 0,
                    total: 1
                },
                "{failure}"
            );
        }
    }

    #[test]
    fn an_unfinished_check_makes_the_checks_pending() {
        let pending = [
            run("IN_PROGRESS", None),
            run("QUEUED", None),
            run("PENDING", None),
            context("PENDING"),
            context("EXPECTED"),
        ];
        for check in pending {
            let checks = checks_of(vec![check.clone()]);

            assert_eq!(
                checks,
                Checks {
                    state: ChecksState::Pending,
                    passed: 0,
                    total: 1
                },
                "{check}"
            );
        }
    }

    #[test]
    fn a_failure_wins_over_pending_checks_and_pending_wins_over_passed() {
        let failing = checks_of(vec![
            run("COMPLETED", Some("SUCCESS")),
            run("IN_PROGRESS", None),
            context("FAILURE"),
        ]);
        let pending = checks_of(vec![run("COMPLETED", Some("SUCCESS")), run("QUEUED", None)]);

        assert_eq!(
            failing,
            Checks {
                state: ChecksState::Failing,
                passed: 1,
                total: 3
            }
        );
        assert_eq!(
            pending,
            Checks {
                state: ChecksState::Pending,
                passed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn the_review_decision_maps_directly_whatever_else_is_known() {
        for (decision, expected) in [
            ("APPROVED", ReviewState::Approved),
            ("CHANGES_REQUESTED", ReviewState::ChangesRequested),
            ("REVIEW_REQUIRED", ReviewState::Waiting),
        ] {
            let review = review_of(json!({
                "reviewDecision": decision,
                "reviewRequests": [{ "login": "marta" }],
                "latestReviews": [{ "state": "COMMENTED" }],
            }));

            assert_eq!(review, expected, "{decision}");
        }
    }

    #[test]
    fn without_a_decision_the_latest_reviews_and_open_requests_decide() {
        let requested = json!([{ "login": "marta" }]);
        for (label, view, expected) in [
            (
                "changes requested beats a request and an approval",
                json!({
                    "reviewDecision": "",
                    "reviewRequests": requested,
                    "latestReviews": [{ "state": "APPROVED" }, { "state": "CHANGES_REQUESTED" }],
                }),
                ReviewState::ChangesRequested,
            ),
            (
                "an open request beats an approval",
                json!({
                    "reviewDecision": "",
                    "reviewRequests": requested,
                    "latestReviews": [{ "state": "APPROVED" }],
                }),
                ReviewState::Waiting,
            ),
            (
                "an approval alone",
                json!({ "reviewDecision": "", "latestReviews": [{ "state": "APPROVED" }] }),
                ReviewState::Approved,
            ),
            (
                "a comment alone",
                json!({ "reviewDecision": "", "latestReviews": [{ "state": "COMMENTED" }] }),
                ReviewState::None,
            ),
            (
                "nothing at all",
                json!({ "reviewDecision": "" }),
                ReviewState::None,
            ),
            (
                "a missing decision",
                json!({ "reviewRequests": requested }),
                ReviewState::Waiting,
            ),
            (
                "a null decision",
                json!({ "reviewDecision": null, "latestReviews": [{ "state": "APPROVED" }] }),
                ReviewState::Approved,
            ),
        ] {
            assert_eq!(review_of(view), expected, "{label}");
        }
    }

    #[test]
    fn unknown_and_missing_fields_do_not_fail_parsing() {
        let status = summary_of(json!({
            "title": "Refetch shipping rates",
            "state": "SOMETHING_NEW",
            "futureField": { "nested": true },
            "latestReviews": [{ "state": "APPROVED", "author": { "login": "marta" } }],
            "statusCheckRollup": [{
                "__typename": "CheckRun",
                "name": "build",
                "status": "COMPLETED",
                "conclusion": "SUCCESS",
                "detailsUrl": "https://example.com/runs/1",
            }],
        }));

        assert_eq!(
            status,
            PrStatus {
                title: "Refetch shipping rates".into(),
                state: PrState::Open,
                checks: Checks {
                    state: ChecksState::Passing,
                    passed: 1,
                    total: 1
                },
                review: ReviewState::Approved,
            }
        );
    }

    #[test]
    fn an_empty_answer_is_an_open_pull_request_with_no_checks_and_no_review() {
        assert_eq!(
            summary_of(json!({})),
            PrStatus {
                title: String::new(),
                state: PrState::Open,
                checks: Checks {
                    state: ChecksState::None,
                    passed: 0,
                    total: 0
                },
                review: ReviewState::None,
            }
        );
    }

    #[test]
    fn the_status_serializes_with_camel_case_names() {
        let status = PrStatus {
            title: "Validate DACH postcodes".into(),
            state: PrState::Draft,
            checks: Checks {
                state: ChecksState::None,
                passed: 0,
                total: 0,
            },
            review: ReviewState::ChangesRequested,
        };

        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({
                "title": "Validate DACH postcodes",
                "state": "draft",
                "checks": { "state": "none", "passed": 0, "total": 0 },
                "review": "changesRequested",
            })
        );
    }
}
