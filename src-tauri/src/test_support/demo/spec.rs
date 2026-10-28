//! Compact descriptions of demo issues and comments, built into Jira JSON by `render`.
//!
//! Times are Vienna wall-clock strings (`"2026-10-28 11:42"`) relative to `DEMO_NOW`.

use super::catalog::{Kind, Priority, Status};
use super::people::{JR, Person};

#[derive(Clone, Debug)]
pub struct IssueSpec {
    pub key: String,
    pub kind: Kind,
    pub status: Status,
    pub summary: String,
    pub priority: Priority,
    pub assignee: Option<&'static Person>,
    pub reporter: &'static Person,
    pub parent: Option<String>,
    pub labels: Vec<&'static str>,
    pub due: Option<String>,
    pub fix: Option<&'static str>,
    pub sprints: Vec<u64>,
    pub points: Option<f64>,
    pub reviewer: Option<&'static Person>,
    pub epic_color: Option<&'static str>,
    pub created: String,
    pub updated: String,
    pub description: Option<String>,
    /// Keys of the issues this one blocks.
    pub blocks: Vec<&'static str>,
}

/// A new issue reported by Jonas, unassigned, Medium, created and updated at `created`.
pub fn issue(key: &str, kind: Kind, status: Status, summary: &str) -> IssueSpec {
    IssueSpec {
        key: key.to_owned(),
        kind,
        status,
        summary: summary.to_owned(),
        priority: Priority::Medium,
        assignee: None,
        reporter: JR,
        parent: None,
        labels: Vec::new(),
        due: None,
        fix: None,
        sprints: Vec::new(),
        points: None,
        reviewer: None,
        epic_color: None,
        created: String::new(),
        updated: String::new(),
        description: None,
        blocks: Vec::new(),
    }
}

impl IssueSpec {
    pub fn to(mut self, assignee: &'static Person) -> Self {
        self.assignee = Some(assignee);
        self
    }

    pub fn by(mut self, reporter: &'static Person) -> Self {
        self.reporter = reporter;
        self
    }

    pub fn prio(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn parent(mut self, key: &str) -> Self {
        self.parent = Some(key.to_owned());
        self
    }

    pub fn labels(mut self, labels: &[&'static str]) -> Self {
        self.labels = labels.to_vec();
        self
    }

    pub fn due(mut self, date: &str) -> Self {
        self.due = Some(date.to_owned());
        self
    }

    pub fn fix(mut self, version: &'static str) -> Self {
        self.fix = Some(version);
        self
    }

    /// Sprint ids, oldest first; a carried-over issue lists every sprint it was in.
    pub fn sprints(mut self, ids: &[u64]) -> Self {
        self.sprints = ids.to_vec();
        self
    }

    pub fn points(mut self, points: f64) -> Self {
        self.points = Some(points);
        self
    }

    pub fn blocks(mut self, key: &'static str) -> Self {
        self.blocks.push(key);
        self
    }

    pub fn reviewer(mut self, reviewer: &'static Person) -> Self {
        self.reviewer = Some(reviewer);
        self
    }

    pub fn color(mut self, ghx_label: &'static str) -> Self {
        self.epic_color = Some(ghx_label);
        self
    }

    /// Created and last updated; a done issue was resolved at `updated`.
    pub fn at(mut self, created: &str, updated: &str) -> Self {
        self.created = created.to_owned();
        self.updated = updated.to_owned();
        self
    }

    pub fn desc(mut self, text: &str) -> Self {
        self.description = Some(text.to_owned());
        self
    }
}

#[derive(Clone, Debug)]
pub struct CommentSpec {
    pub issue: &'static str,
    pub author: &'static Person,
    pub at: &'static str,
    pub text: &'static str,
}

pub fn comment(
    issue: &'static str,
    author: &'static Person,
    at: &'static str,
    text: &'static str,
) -> CommentSpec {
    CommentSpec {
        issue,
        author,
        at,
        text,
    }
}
