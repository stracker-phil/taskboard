//! The site's metadata: the user, the cloud id, fields, projects, statuses, versions.

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Myself {
    pub account_id: String,
    pub display_name: String,
    /// IANA name; JQL date literals are read in this zone.
    pub time_zone: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TenantInfo {
    pub cloud_id: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub account_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default = "active")]
    pub active: bool,
}

fn active() -> bool {
    true
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub schema: Option<FieldSchema>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldSchema {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub items: Option<String>,
    /// The custom field's type key (`com.pyxis.greenhopper.jira:gh-sprint`).
    #[serde(default)]
    pub custom: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub key: String,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub id: String,
    pub name: String,
    pub status_category: StatusCategory,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusCategory {
    /// `new`, `indeterminate` or `done`.
    pub key: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub released: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub release_date: Option<String>,
}
