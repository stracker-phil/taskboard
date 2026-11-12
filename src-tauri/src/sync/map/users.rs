//! The people an issue names: assignee, reporter, commenters and the authors of its embedded changelog.

use serde_json::Value;

use super::text;
use crate::jira::dto::{Issue, User};
use crate::net::AvatarUrl;

#[derive(Debug, Clone, PartialEq)]
pub struct UserRow {
    pub account_id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    /// Jira's `accountType`; `app` is an automation.
    pub account_type: Option<String>,
}

impl UserRow {
    /// `None` without a display name: such a user object can't name anyone.
    pub fn of(user: &User) -> Option<Self> {
        (!user.display_name.is_empty()).then(|| Self {
            account_id: user.account_id.clone(),
            name: user.display_name.clone(),
            avatar_url: user.avatar_url(),
            account_type: user.account_type.clone(),
        })
    }
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
                account_type: text(&person["accountType"]),
            });
        }
    }
    let authors = issue
        .changelog
        .iter()
        .flat_map(|changelog| &changelog.histories)
        .filter_map(|entry| entry.author.as_ref().and_then(UserRow::of));
    for author in authors {
        if users.iter().all(|u| u.account_id != author.account_id) {
            users.push(author);
        }
    }
    users
}
