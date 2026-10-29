//! What the running app sends back for the list subcommands; `--json` prints these as they are.

use serde::{Deserialize, Serialize};

use crate::lists::Kind;
use crate::views::Layout;

/// A tracked Space (`taskboard spaces`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpaceRow {
    /// Its sidebar level, for `go`.
    pub id: i64,
    pub key: String,
    pub name: String,
    pub client: Option<String>,
    pub open: i64,
    pub current: bool,
}

/// A sidebar level other than a Space (`taskboard filters`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LevelRow {
    pub id: i64,
    pub kind: Kind,
    pub name: String,
    /// The key of the Space it belongs to; none under Home.
    pub space: Option<String>,
    /// Open issues; none when its filter can't be used.
    pub open: Option<i64>,
    pub current: bool,
}

/// A view of a level (`taskboard views`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ViewRow {
    pub id: i64,
    pub name: String,
    pub layout: Layout,
    pub is_default: bool,
    pub current: bool,
}

/// The level whose views `taskboard views` lists, and its views.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ViewsReport {
    pub level: String,
    pub views: Vec<ViewRow>,
}

/// An issue found by `taskboard search`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IssueRow {
    pub key: String,
    pub status: String,
    pub summary: String,
    /// Deleted in Jira or moved out of the sync scope; kept in the cache.
    pub gone: bool,
}

/// Where the main window is (`status`, `go`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaceReport {
    pub list_id: i64,
    pub level: String,
    pub view_id: i64,
    pub view: String,
}
