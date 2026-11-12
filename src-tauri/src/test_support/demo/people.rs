//! The demo site's people, as Jira user objects.

use serde_json::{Value, json};

use super::SITE_URL;

pub const ME_EMAIL: &str = "alex.morgan@example.com";
const TIME_ZONE: &str = "Europe/Berlin";

#[derive(Debug, PartialEq, Eq)]
pub struct Person {
    pub account_id: &'static str,
    pub name: &'static str,
    pub email: &'static str,
    /// Jira's `accountType`: `atlassian` for a person, `app` for an automation.
    pub account_type: &'static str,
}

pub const AM: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01",
    name: "Alex Morgan",
    email: ME_EMAIL,
    account_type: "atlassian",
};
pub const JR: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e02",
    name: "Jonas Reiter",
    email: "jonas.reiter@example.com",
    account_type: "atlassian",
};
pub const MK: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03",
    name: "Marta Klein",
    email: "marta.klein@example.com",
    account_type: "atlassian",
};
pub const AL: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e04",
    name: "Anna Lutz",
    email: "anna.lutz@example.com",
    account_type: "atlassian",
};
pub const SB: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e05",
    name: "Sam Becker",
    email: "sam.becker@example.com",
    account_type: "atlassian",
};
pub const LN: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e06",
    name: "Lea Novak",
    email: "lea.novak@example.com",
    account_type: "atlassian",
};
pub const TH: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e07",
    name: "Tom Hale",
    email: "tom.hale@example.com",
    account_type: "atlassian",
};

/// An automation (a made-up test management app): its changes and comments come from an app account.
pub const TR: &Person = &Person {
    account_id: "557058:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e90",
    name: "Test Runner",
    email: "test-runner@example.com",
    account_type: "app",
};

/// The people; the automation isn't one of them.
pub const EVERYONE: [&Person; 7] = [AM, JR, MK, AL, SB, LN, TH];

/// The user object Jira embeds in issues and comments.
pub fn user(person: &Person) -> Value {
    let avatar = |size: u32| {
        format!(
            "https://avatar.example.com/{}/{size}.png",
            person.account_id.replace(':', "-")
        )
    };
    json!({
        "self": format!("{SITE_URL}/rest/api/3/user?accountId={}", person.account_id),
        "accountId": person.account_id,
        "accountType": person.account_type,
        "emailAddress": person.email,
        "avatarUrls": {
            "48x48": avatar(48),
            "24x24": avatar(24),
            "16x16": avatar(16),
            "32x32": avatar(32),
        },
        "displayName": person.name,
        "active": true,
        "timeZone": TIME_ZONE,
    })
}

/// `GET /myself` for the signed-in demo user.
pub fn myself() -> Value {
    let mut me = user(AM);
    me["locale"] = json!("en_US");
    me
}

/// Everyone and the automation.
pub fn users() -> Vec<Value> {
    EVERYONE.into_iter().chain([TR]).map(user).collect()
}
