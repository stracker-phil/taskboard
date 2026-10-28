//! The Jira connection: where and as whom. The token is never part of it; it lives in the Keychain.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::jira::{Site, TokenKind};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct Connection {
    /// Normalised (`https://example.atlassian.net/`); empty until set up.
    pub site_url: String,
    pub email: String,
    pub token_kind: TokenKind,
    #[ts(type = "string | null")]
    pub token_expires: Option<NaiveDate>,
    /// Scoped tokens only: found by Test Connection or typed in.
    pub cloud_id: Option<String>,
    /// From the last successful Test Connection; cleared when the site, email or token kind change.
    pub account_id: Option<String>,
    pub display_name: Option<String>,
    /// IANA name from `/myself`; JQL date literals use it.
    pub time_zone: Option<String>,
}

impl Connection {
    pub fn site(&self) -> AppResult<Site> {
        if self.site_url.is_empty() || self.email.is_empty() {
            return Err(AppError::NotConnected("Connect Jira in Settings".into()));
        }
        Site::new(
            &self.site_url,
            &self.email,
            self.token_kind,
            self.cloud_id.clone(),
        )
    }

    /// Who Jira said the user is no longer holds once the site, the email or the token kind change.
    pub fn same_account(&self, other: &Connection) -> bool {
        self.site_url == other.site_url
            && self.email == other.email
            && self.token_kind == other.token_kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn connection() -> Connection {
        Connection {
            site_url: "https://example.atlassian.net/".into(),
            email: "alex.morgan@example.com".into(),
            ..Connection::default()
        }
    }

    #[test]
    fn without_a_site_or_an_email_the_connection_is_not_connected() {
        for (site_url, email) in [
            ("", ""),
            ("https://example.atlassian.net/", ""),
            ("", "alex.morgan@example.com"),
        ] {
            let connection = Connection {
                site_url: site_url.into(),
                email: email.into(),
                ..Connection::default()
            };
            assert_eq!(
                connection.site().unwrap_err().kind(),
                crate::error::ErrorKind::NotConnected,
                "{site_url:?} {email:?}"
            );
        }
    }

    #[test]
    fn a_complete_connection_makes_a_site_with_its_token_kind_and_cloud_id() {
        let connection = Connection {
            token_kind: TokenKind::Scoped,
            cloud_id: Some("cloud-1".into()),
            ..connection()
        };

        let site = connection.site().unwrap();

        assert_eq!(site.base_url().as_str(), "https://example.atlassian.net/");
        assert_eq!(site.email, "alex.morgan@example.com");
        assert_eq!(site.token_kind, TokenKind::Scoped);
        assert_eq!(site.cloud_id.as_deref(), Some("cloud-1"));
    }

    #[test]
    fn the_account_changes_with_the_site_the_email_or_the_token_kind() {
        let base = connection();
        for (other, same) in [
            (connection(), true),
            (
                Connection {
                    display_name: Some("Alex Morgan".into()),
                    cloud_id: Some("cloud-1".into()),
                    token_expires: NaiveDate::from_ymd_opt(2027, 1, 1),
                    ..connection()
                },
                true,
            ),
            (
                Connection {
                    site_url: "https://other.atlassian.net/".into(),
                    ..connection()
                },
                false,
            ),
            (
                Connection {
                    email: "sam.lee@example.com".into(),
                    ..connection()
                },
                false,
            ),
            (
                Connection {
                    token_kind: TokenKind::Scoped,
                    ..connection()
                },
                false,
            ),
        ] {
            assert_eq!(base.same_account(&other), same, "{other:?}");
        }
    }

    #[test]
    fn the_connection_uses_camel_case_names_and_never_carries_a_token() {
        assert_eq!(
            serde_json::to_value(connection()).unwrap(),
            json!({
                "siteUrl": "https://example.atlassian.net/",
                "email": "alex.morgan@example.com",
                "tokenKind": "classic",
                "tokenExpires": null,
                "cloudId": null,
                "accountId": null,
                "displayName": null,
                "timeZone": null,
            })
        );
    }
}
