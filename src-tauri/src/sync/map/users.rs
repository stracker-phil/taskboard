//! The people an issue names: assignee, reporter and commenters.

use serde_json::Value;

use super::text;
use crate::jira::dto::Issue;
use crate::net::AvatarUrl;

#[derive(Debug, Clone, PartialEq)]
pub struct UserRow {
    pub account_id: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

pub(super) fn users_of(issue: &Issue) -> Vec<UserRow> {
    let f = &issue.fields;
    let mut people: Vec<&Value> = vec![&f["assignee"], &f["reporter"]];
    people.extend(
        f["comment"]["comments"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| &c["author"]),
    );
    let mut users: Vec<UserRow> = Vec::new();
    for person in people {
        let (Some(account_id), Some(name)) =
            (text(&person["accountId"]), text(&person["displayName"]))
        else {
            continue;
        };
        if users.iter().all(|u| u.account_id != account_id) {
            users.push(UserRow {
                account_id,
                name,
                avatar_url: text(&person["avatarUrls"]["48x48"])
                    .filter(|url| AvatarUrl::parse(url).is_some()),
            });
        }
    }
    users
}
