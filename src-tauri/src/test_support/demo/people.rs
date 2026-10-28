//! The demo site's people, as Jira user objects.

use serde_json::{Value, json};

use super::SITE_URL;

pub const ME_EMAIL: &str = "alex.morgan@example.com";
const TIME_ZONE: &str = "Europe/Vienna";

#[derive(Debug, PartialEq, Eq)]
pub struct Person {
    pub account_id: &'static str,
    pub name: &'static str,
    pub email: &'static str,
}

pub const AM: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01",
    name: "Alex Morgan",
    email: ME_EMAIL,
};
pub const JR: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e02",
    name: "Jonas Reiter",
    email: "jonas.reiter@example.com",
};
pub const MK: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03",
    name: "Marta Klein",
    email: "marta.klein@example.com",
};
pub const AL: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e04",
    name: "Anna Lutz",
    email: "anna.lutz@example.com",
};
pub const SB: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e05",
    name: "Sam Becker",
    email: "sam.becker@example.com",
};
pub const LN: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e06",
    name: "Lea Novak",
    email: "lea.novak@example.com",
};
pub const TH: &Person = &Person {
    account_id: "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e07",
    name: "Tom Hale",
    email: "tom.hale@example.com",
};

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
        "accountType": "atlassian",
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

pub fn users() -> Vec<Value> {
    EVERYONE.into_iter().map(user).collect()
}
