//! Web links: Jira's remote links on an issue (a pull request, a document).

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct RemoteLink {
    pub id: u64,
    #[serde(default)]
    pub relationship: Option<String>,
    #[serde(default)]
    pub application: Option<RemoteApplication>,
    pub object: RemoteObject,
}

/// The tool the link points into, as the app that created it named it.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RemoteApplication {
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct RemoteObject {
    /// Untrusted: anyone who can edit the issue sets it.
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub icon: Option<RemoteIcon>,
}

/// Read but never loaded: the webview fetches nothing remote (ADR 0027).
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RemoteIcon {
    #[serde(default)]
    pub url16x16: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}
