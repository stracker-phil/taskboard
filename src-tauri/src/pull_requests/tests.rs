use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::test_support::{FakePullRequests, temp_config_dir};

const MIXED: &str = "20001";
const GITLAB_ONLY: &str = "20002";
const NO_LINKS: &str = "20003";
const MERGED_ONLY: &str = "20004";
const OPEN_ONLY: &str = "20005";

const SETUP: &str = "
    INSERT INTO cache.space (id, key, name, raw) VALUES ('10001', 'NW', 'Northwind', '{}');
    INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, status_category,
      created_at, updated_at, synced_at)
    VALUES
     ('20001', 'NW-1', '10001', 'One', 'Task', '1', 'new', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z'),
     ('20002', 'NW-2', '10001', 'Two', 'Task', '1', 'new', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z'),
     ('20003', 'NW-3', '10001', 'Three', 'Task', '1', 'new', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z'),
     ('20004', 'NW-4', '10001', 'Four', 'Task', '1', 'new', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z'),
     ('20005', 'NW-5', '10001', 'Five', 'Task', '1', 'new', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z');
    INSERT INTO cache.remote_link (issue_id, id, position, url, title) VALUES
     ('20001', '1', 0, 'https://github.com/example/shop/pull/481', 'Postcode checks'),
     ('20001', '2', 1, 'https://example.com/docs/postcodes', 'Postcode spec'),
     ('20001', '3', 2, 'https://gitlab.example.com/acme/store-api/-/merge_requests/58', 'Retry the export'),
     ('20001', '4', 3, 'https://github.com/Example/Shop/pull/476/files', 'Shipping rates'),
     ('20002', '1', 0, 'https://gitlab.example.com/acme/store-api/-/merge_requests/58', 'Retry the export'),
     ('20004', '1', 0, 'https://github.com/example/shop/pull/476', 'Shipping rates'),
     ('20005', '1', 0, 'https://github.com/example/shop/pull/481', 'Postcode checks');";

async fn seeded() -> (tempfile::TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    db.write(|tx, _| Ok(tx.execute_batch(SETUP)?))
        .await
        .unwrap();
    (dir, db)
}

fn at(minutes: i64, seconds: i64) -> DateTime<Utc> {
    "2026-10-28T09:00:00Z".parse::<DateTime<Utc>>().unwrap()
        + Duration::minutes(minutes)
        + Duration::seconds(seconds)
}

fn status(title: &str) -> PrStatus {
    PrStatus {
        title: title.into(),
        state: PrState::Open,
        checks: Checks {
            state: ChecksState::Passing,
            passed: 1,
            total: 1,
        },
        review: ReviewState::None,
    }
}

/// Answers each lookup with the next of `answers`; the last one repeats.
struct Scripted {
    answers: Vec<Result<PrStatus, Problem>>,
    asked: AtomicUsize,
}

impl Scripted {
    fn new(answers: Vec<Result<PrStatus, Problem>>) -> Arc<Self> {
        Arc::new(Self {
            answers,
            asked: AtomicUsize::new(0),
        })
    }
}

impl StatusSource for Scripted {
    fn status<'a>(&'a self, _pr: &'a GithubPr) -> StatusAnswer<'a> {
        let n = self.asked.fetch_add(1, Ordering::SeqCst);
        let answer = self.answers[n.min(self.answers.len() - 1)].clone();
        Box::pin(async move { answer })
    }
}

fn demo_statuses() -> PullRequestStatuses {
    PullRequestStatuses::new(Arc::new(FakePullRequests))
}

async fn titles(
    statuses: &PullRequestStatuses,
    db: &Db,
    id: &str,
    now: DateTime<Utc>,
    fetch: bool,
) -> Vec<String> {
    let pulls = statuses.of_issue(db, id.into(), now, fetch).await.unwrap();
    pulls.items.into_iter().map(|item| item.title).collect()
}

#[tokio::test]
async fn only_pull_requests_are_listed_in_link_order_with_repo_and_number() {
    let (_dir, db) = seeded().await;

    let pulls = demo_statuses()
        .of_issue(&db, MIXED.into(), at(0, 0), false)
        .await
        .unwrap();

    let listed: Vec<(&str, &str, u32, &str, bool)> = pulls
        .items
        .iter()
        .map(|p| {
            (
                p.id.as_str(),
                p.repo.as_str(),
                p.number,
                p.url.as_str(),
                p.openable,
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            (
                "1",
                "example/shop",
                481,
                "https://github.com/example/shop/pull/481",
                true
            ),
            (
                "3",
                "store-api",
                58,
                "https://gitlab.example.com/acme/store-api/-/merge_requests/58",
                true
            ),
            (
                "4",
                "example/shop",
                476,
                "https://github.com/Example/Shop/pull/476/files",
                true
            ),
        ]
    );
}

#[tokio::test]
async fn nothing_is_fetched_without_asking_and_titles_stay_the_links_own() {
    let (_dir, db) = seeded().await;
    let source = Scripted::new(vec![Ok(status("From GitHub"))]);
    let statuses = PullRequestStatuses::new(source);

    let pulls = statuses
        .of_issue(&db, MIXED.into(), at(0, 0), false)
        .await
        .unwrap();

    assert!(
        pulls
            .items
            .iter()
            .all(|p| p.status.is_none() && p.problem.is_none())
    );
    let titles: Vec<&str> = pulls.items.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(
        titles,
        ["Postcode checks", "Retry the export", "Shipping rates"]
    );
}

#[tokio::test]
async fn a_merge_request_elsewhere_has_neither_status_nor_problem_even_when_fetching() {
    let (_dir, db) = seeded().await;
    let source = Scripted::new(vec![Err("gh isn't here".into())]);
    let statuses = PullRequestStatuses::new(source);

    let pulls = statuses
        .of_issue(&db, GITLAB_ONLY.into(), at(0, 0), true)
        .await
        .unwrap();

    assert_eq!(
        pulls,
        IssuePullRequests {
            items: vec![PullRequest {
                id: "1".into(),
                url: "https://gitlab.example.com/acme/store-api/-/merge_requests/58".into(),
                number: 58,
                repo: "store-api".into(),
                title: "Retry the export".into(),
                openable: true,
                status: None,
                problem: None,
            }],
            refresh_in_ms: None,
        }
    );
}

#[tokio::test]
async fn a_github_pull_request_never_fetched_is_due_now() {
    let (_dir, db) = seeded().await;
    let statuses = demo_statuses();

    let mixed = statuses
        .of_issue(&db, MIXED.into(), at(0, 0), false)
        .await
        .unwrap();

    assert_eq!(mixed.refresh_in_ms, Some(0));
}

#[tokio::test]
async fn an_issue_without_github_pull_requests_needs_no_refresh() {
    let (_dir, db) = seeded().await;
    let statuses = demo_statuses();

    for id in [GITLAB_ONLY, NO_LINKS] {
        let pulls = statuses
            .of_issue(&db, id.into(), at(0, 0), false)
            .await
            .unwrap();

        assert_eq!(pulls.refresh_in_ms, None, "{id}");
    }
    let none = statuses
        .of_issue(&db, NO_LINKS.into(), at(0, 0), true)
        .await
        .unwrap();
    assert_eq!(
        none,
        IssuePullRequests {
            items: vec![],
            refresh_in_ms: None
        }
    );
}

#[tokio::test]
async fn an_issue_the_cache_does_not_hold_has_no_pull_requests() {
    let (_dir, db) = seeded().await;

    let pulls = demo_statuses()
        .of_issue(&db, "99999".into(), at(0, 0), true)
        .await
        .unwrap();

    assert_eq!(
        pulls,
        IssuePullRequests {
            items: vec![],
            refresh_in_ms: None
        }
    );
}

#[tokio::test]
async fn fetching_fills_the_statuses_and_the_title_comes_from_github() {
    let (_dir, db) = seeded().await;

    let pulls = demo_statuses()
        .of_issue(&db, MIXED.into(), at(0, 0), true)
        .await
        .unwrap();

    let github: Vec<(u32, Option<PrStatus>, Option<Problem>, &str)> = pulls
        .items
        .iter()
        .filter(|p| p.repo == "example/shop")
        .map(|p| {
            (
                p.number,
                p.status.clone(),
                p.problem.clone(),
                p.title.as_str(),
            )
        })
        .collect();
    assert_eq!(
        github,
        [
            (
                481,
                Some(PrStatus {
                    title: "Validate DACH postcodes".into(),
                    state: PrState::Open,
                    checks: Checks {
                        state: ChecksState::Pending,
                        passed: 9,
                        total: 12
                    },
                    review: ReviewState::Waiting,
                }),
                None,
                "Validate DACH postcodes"
            ),
            (
                476,
                Some(PrStatus {
                    title: "Refetch shipping rates on address change".into(),
                    state: PrState::Merged,
                    checks: Checks {
                        state: ChecksState::Passing,
                        passed: 12,
                        total: 12
                    },
                    review: ReviewState::Approved,
                }),
                None,
                "Refetch shipping rates on address change"
            ),
        ]
    );
}

#[tokio::test]
async fn an_open_pull_request_is_asked_again_after_five_minutes_and_a_merged_one_never() {
    let (_dir, db) = seeded().await;
    let statuses = demo_statuses();

    let open = statuses
        .of_issue(&db, OPEN_ONLY.into(), at(0, 0), true)
        .await
        .unwrap();
    let merged = statuses
        .of_issue(&db, MERGED_ONLY.into(), at(0, 0), true)
        .await
        .unwrap();

    assert_eq!(open.refresh_in_ms, Some(5 * 60 * 1000));
    assert_eq!(merged.refresh_in_ms, None);
}

#[tokio::test]
async fn a_merged_pull_request_beside_an_open_one_leaves_the_open_ones_due_time() {
    let (_dir, db) = seeded().await;
    let statuses = demo_statuses();

    let pulls = statuses
        .of_issue(&db, MIXED.into(), at(0, 0), true)
        .await
        .unwrap();

    assert_eq!(pulls.refresh_in_ms, Some(5 * 60 * 1000));
}

#[tokio::test]
async fn the_refresh_time_counts_down_as_the_clock_moves() {
    let (_dir, db) = seeded().await;
    let statuses = demo_statuses();
    statuses
        .of_issue(&db, OPEN_ONLY.into(), at(0, 0), true)
        .await
        .unwrap();

    let later = statuses
        .of_issue(&db, OPEN_ONLY.into(), at(2, 0), false)
        .await
        .unwrap();
    let overdue = statuses
        .of_issue(&db, OPEN_ONLY.into(), at(9, 0), false)
        .await
        .unwrap();

    assert_eq!(later.refresh_in_ms, Some(3 * 60 * 1000));
    assert_eq!(overdue.refresh_in_ms, Some(0));
}

#[tokio::test]
async fn within_five_minutes_the_cached_answer_stays() {
    let (_dir, db) = seeded().await;
    let statuses = PullRequestStatuses::new(Scripted::new(vec![
        Ok(status("First answer")),
        Ok(status("Second answer")),
    ]));
    titles(&statuses, &db, OPEN_ONLY, at(0, 0), true).await;

    let again = titles(&statuses, &db, OPEN_ONLY, at(4, 59), true).await;

    assert_eq!(again, ["First answer"]);
}

#[tokio::test]
async fn at_five_minutes_the_status_is_fetched_again() {
    let (_dir, db) = seeded().await;
    let statuses = PullRequestStatuses::new(Scripted::new(vec![
        Ok(status("First answer")),
        Ok(status("Second answer")),
        Ok(status("Third answer")),
    ]));
    titles(&statuses, &db, OPEN_ONLY, at(0, 0), true).await;

    let at_the_limit = titles(&statuses, &db, OPEN_ONLY, at(5, 0), true).await;
    let after_that = titles(&statuses, &db, OPEN_ONLY, at(11, 0), true).await;

    assert_eq!(at_the_limit, ["Second answer"]);
    assert_eq!(after_that, ["Third answer"]);
}

#[tokio::test]
async fn not_fetching_never_refreshes_a_stale_status() {
    let (_dir, db) = seeded().await;
    let statuses = PullRequestStatuses::new(Scripted::new(vec![
        Ok(status("First answer")),
        Ok(status("Second answer")),
    ]));
    titles(&statuses, &db, OPEN_ONLY, at(0, 0), true).await;

    let stale = titles(&statuses, &db, OPEN_ONLY, at(30, 0), false).await;

    assert_eq!(stale, ["First answer"]);
}

#[tokio::test]
async fn the_same_pull_request_on_two_issues_shares_one_status() {
    let (_dir, db) = seeded().await;
    let statuses = PullRequestStatuses::new(Scripted::new(vec![
        Ok(status("First answer")),
        Ok(status("Second answer")),
    ]));
    let first = titles(&statuses, &db, OPEN_ONLY, at(0, 0), true).await;

    let mixed = statuses
        .of_issue(&db, MIXED.into(), at(1, 0), true)
        .await
        .unwrap();

    assert_eq!(first, ["First answer"]);
    assert_eq!(mixed.items[0].number, 481);
    assert_eq!(mixed.items[0].title, "First answer");
}

#[tokio::test]
async fn a_failed_lookup_shows_its_reason_without_a_status_and_keeps_the_links_title() {
    let (_dir, db) = seeded().await;
    let statuses = PullRequestStatuses::new(Scripted::new(vec![Err(
        "GitHub has no such pull request".into(),
    )]));

    let pulls = statuses
        .of_issue(&db, OPEN_ONLY.into(), at(0, 0), true)
        .await
        .unwrap();

    assert_eq!(pulls.items.len(), 1);
    let item = &pulls.items[0];
    assert_eq!(item.status, None);
    assert_eq!(
        item.problem.as_deref(),
        Some("GitHub has no such pull request")
    );
    assert_eq!(item.title, "Postcode checks");
}

#[tokio::test]
async fn a_failed_lookup_is_kept_like_a_success_and_asked_again_after_five_minutes() {
    let (_dir, db) = seeded().await;
    let statuses = PullRequestStatuses::new(Scripted::new(vec![
        Err("gh took too long to answer".into()),
        Ok(status("Recovered")),
    ]));
    statuses
        .of_issue(&db, OPEN_ONLY.into(), at(0, 0), true)
        .await
        .unwrap();

    let within = statuses
        .of_issue(&db, OPEN_ONLY.into(), at(1, 0), true)
        .await
        .unwrap();
    let after = statuses
        .of_issue(&db, OPEN_ONLY.into(), at(5, 0), true)
        .await
        .unwrap();

    assert_eq!(
        within.items[0].problem.as_deref(),
        Some("gh took too long to answer")
    );
    assert_eq!(within.items[0].status, None);
    assert_eq!(within.refresh_in_ms, Some(4 * 60 * 1000));
    assert_eq!(after.items[0].problem, None);
    assert_eq!(after.items[0].status, Some(status("Recovered")));
}

#[tokio::test]
async fn a_pull_request_the_demo_does_not_know_fails_with_a_reason() {
    let (_dir, db) = seeded().await;
    db.write(|tx, _| {
        Ok(tx
            .execute(
                "INSERT INTO cache.remote_link (issue_id, id, position, url, title)
             VALUES ('20003', '1', 0, 'https://github.com/example/shop/pull/9999', 'Unknown')",
                [],
            )
            .map(|_| ())?)
    })
    .await
    .unwrap();

    let pulls = demo_statuses()
        .of_issue(&db, NO_LINKS.into(), at(0, 0), true)
        .await
        .unwrap();

    assert_eq!(
        pulls.items[0].problem.as_deref(),
        Some("GitHub has no such pull request")
    );
    assert_eq!(pulls.refresh_in_ms, Some(5 * 60 * 1000));
}

#[tokio::test]
async fn the_pull_requests_serialize_with_camel_case_names() {
    let (_dir, db) = seeded().await;

    let pulls = demo_statuses()
        .of_issue(&db, OPEN_ONLY.into(), at(0, 0), true)
        .await
        .unwrap();

    assert_eq!(
        serde_json::to_value(&pulls).unwrap(),
        serde_json::json!({
            "items": [{
                "id": "1",
                "url": "https://github.com/example/shop/pull/481",
                "number": 481,
                "repo": "example/shop",
                "title": "Validate DACH postcodes",
                "openable": true,
                "status": {
                    "title": "Validate DACH postcodes",
                    "state": "open",
                    "checks": { "state": "pending", "passed": 9, "total": 12 },
                    "review": "waiting",
                },
                "problem": null,
            }],
            "refreshInMs": 300000,
        })
    );
}
