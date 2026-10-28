//! Generated issues over the demo projects, for performance tests.

use chrono::TimeDelta;

use super::catalog::Kind::{self, Bug, Epic, Story, Task};
use super::catalog::{PROJECTS, Priority, Status, VERSIONS};
use super::clock::{Clock, demo_now, local};
use super::people::{EVERYONE, Person};
use super::spec::{IssueSpec, issue};
use super::{JiraData, site};

const SEED: u64 = 0x7a5c_b0a2;
/// One issue in this many is an epic.
const EPIC_EVERY: usize = 25;
const VERBS: [&str; 8] = [
    "Fix", "Add", "Remove", "Refactor", "Document", "Speed up", "Test", "Redesign",
];
const OBJECTS: [&str; 10] = [
    "the login form",
    "order export",
    "search results",
    "the settings page",
    "invoice emails",
    "image uploads",
    "the booking calendar",
    "push notifications",
    "the admin dashboard",
    "rate limiting",
];
const LABELS: [&str; 4] = ["frontend", "backend", "client", "support"];
const POINTS: [f64; 5] = [1.0, 2.0, 3.0, 5.0, 8.0];
const NW_SPRINTS: [u64; 3] = [21, 22, 23];

/// The demo site with `n` deterministic pseudo-random issues instead of the hand-written ones.
pub fn synthetic(n: usize) -> JiraData {
    let mut rng = fastrand::Rng::with_seed(SEED);
    let tracked: Vec<_> = PROJECTS.iter().filter(|p| p.key != "ARC").collect();
    let mut next_number = vec![1_u32; tracked.len()];
    let mut epics: Vec<Vec<String>> = vec![Vec::new(); tracked.len()];
    let now = demo_now();

    let specs: Vec<IssueSpec> = (0..n)
        .map(|i| {
            let slot = i % tracked.len();
            let project = tracked[slot];
            let key = format!("{}-{}", project.key, next_number[slot]);
            next_number[slot] += 1;
            let kind = if (i / tracked.len()) % EPIC_EVERY == 0 {
                Epic
            } else {
                [Story, Task, Bug][rng.usize(..3)]
            };
            let created = now - TimeDelta::minutes(rng.i64(60..400 * 24 * 60));
            let updated = created + TimeDelta::minutes(rng.i64(0..=(now - created).num_minutes()));
            let summary = format!(
                "{} {}",
                VERBS[rng.usize(..VERBS.len())],
                OBJECTS[rng.usize(..OBJECTS.len())]
            );
            let mut spec = issue(&key, kind, status(&mut rng), &summary)
                .by(person(&mut rng))
                .at(&local(created), &local(updated));
            spec.priority = Priority::ALL[rng.usize(..Priority::ALL.len())];
            spec.assignee = rng.bool().then(|| person(&mut rng));
            spec.labels = LABELS
                .iter()
                .copied()
                .filter(|_| rng.u8(..4) == 0)
                .collect();
            spec = fill_optional(spec, &mut rng, project.key, kind);
            if kind == Epic {
                spec = spec.color(epic_color(&mut rng));
                epics[slot].push(key);
            } else if let Some(epic) = pick(&mut rng, &epics[slot]) {
                spec = spec.parent(epic);
            }
            spec
        })
        .collect();
    site(&specs, &[], &Clock::new(now))
}

fn fill_optional(
    mut spec: IssueSpec,
    rng: &mut fastrand::Rng,
    project: &str,
    kind: Kind,
) -> IssueSpec {
    let versions: Vec<_> = VERSIONS.iter().filter(|v| v.project == project).collect();
    if !versions.is_empty() && rng.bool() {
        spec = spec.fix(versions[rng.usize(..versions.len())].name);
    }
    if kind == Epic {
        return spec;
    }
    if project == "NW" && rng.u8(..5) < 2 {
        spec = spec.sprints(&[NW_SPRINTS[rng.usize(..NW_SPRINTS.len())]]);
    }
    if rng.bool() {
        spec = spec.points(POINTS[rng.usize(..POINTS.len())]);
    }
    if rng.u8(..5) == 0 {
        spec = spec.reviewer(person(rng));
    }
    if rng.u8(..5) == 0 {
        let day = 1 + rng.u8(..28);
        spec = spec.due(&format!("2026-11-{day:02}"));
    }
    spec
}

/// Done for two in five issues, so the default scope keeps most of them.
fn status(rng: &mut fastrand::Rng) -> Status {
    match rng.u8(..10) {
        0..=3 => Status::Done,
        4..=5 => Status::ToDo,
        6..=7 => Status::InProgress,
        8 => Status::InReview,
        _ => Status::Blocked,
    }
}

fn person(rng: &mut fastrand::Rng) -> &'static Person {
    EVERYONE[rng.usize(..EVERYONE.len())]
}

fn pick<'a>(rng: &mut fastrand::Rng, keys: &'a [String]) -> Option<&'a str> {
    if keys.is_empty() || rng.bool() {
        return None;
    }
    Some(&keys[rng.usize(..keys.len())])
}

fn epic_color(rng: &mut fastrand::Rng) -> &'static str {
    const COLORS: [&str; 14] = [
        "ghx-label-1",
        "ghx-label-2",
        "ghx-label-3",
        "ghx-label-4",
        "ghx-label-5",
        "ghx-label-6",
        "ghx-label-7",
        "ghx-label-8",
        "ghx-label-9",
        "ghx-label-10",
        "ghx-label-11",
        "ghx-label-12",
        "ghx-label-13",
        "ghx-label-14",
    ];
    COLORS[rng.usize(..COLORS.len())]
}
