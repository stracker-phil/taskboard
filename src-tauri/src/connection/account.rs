//! Saving the connection, and Test Connection, which fills in what Jira knows: the cloud id for scoped
//! tokens, the account and its time zone.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::JiraAccess;
use super::expiry::{TokenExpiry, token_expiry};
use crate::error::{AppError, AppResult};
use crate::jira::TokenKind;
use crate::jira::site::normalize_site_url;
use crate::store::settings::{Connection, SettingsStore};

/// What the Connection page edits.
#[derive(Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub site_url: String,
    pub email: String,
    pub token_kind: TokenKind,
    #[ts(type = "string | null")]
    pub token_expires: Option<NaiveDate>,
    /// Typed in when the site doesn't tell it; `None` keeps the one found.
    pub cloud_id: Option<String>,
}

/// The connection as the page shows it: never the token, only whether there is one.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub connection: Connection,
    pub has_token: bool,
    pub expiry: TokenExpiry,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    pub display_name: String,
    pub account_id: String,
    pub time_zone: String,
    pub cloud_id: Option<String>,
}

pub fn view(settings: &SettingsStore, access: &JiraAccess, today: NaiveDate) -> ConnectionView {
    let connection = settings.get().connection;
    ConnectionView {
        has_token: access.has_token(&connection.email),
        expiry: token_expiry(connection.token_expires, today),
        connection,
    }
}

/// Validates and saves; `token` (when given) goes to the Keychain under the email.
pub async fn save(
    settings: &SettingsStore,
    access: &JiraAccess,
    input: ConnectionInput,
    token: Option<String>,
    today: NaiveDate,
) -> AppResult<ConnectionView> {
    let site_url = normalize_site_url(&input.site_url)?.to_string();
    let email = input.email.trim().to_string();
    if !email.contains('@') {
        return Err(AppError::InvalidField {
            field: "email".into(),
            message: "Enter the email of your Atlassian account".into(),
        });
    }
    let previous = settings.get().connection;
    let token = token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    match &token {
        Some(token) => access.store_token(&email, token)?,
        None if previous.email != email && !previous.email.is_empty() => {
            access.move_token(&previous.email, &email)?
        }
        None => {}
    }
    let next = next_connection(&previous, input, site_url, email);
    settings.edit(move |s| s.connection = next).await?;
    Ok(view(settings, access, today))
}

/// A new site forgets its cloud id; a typed cloud id replaces it (empty clears it); another account forgets
/// what Jira said about the old one.
fn next_connection(
    previous: &Connection,
    input: ConnectionInput,
    site_url: String,
    email: String,
) -> Connection {
    let mut next = Connection {
        site_url,
        email,
        token_kind: input.token_kind,
        token_expires: input.token_expires,
        ..previous.clone()
    };
    if next.site_url != previous.site_url {
        next.cloud_id = None;
    }
    if let Some(cloud_id) = input.cloud_id.map(|id| id.trim().to_string()) {
        next.cloud_id = (!cloud_id.is_empty()).then_some(cloud_id);
    }
    if !next.same_account(previous) {
        next.account_id = None;
        next.display_name = None;
        next.time_zone = None;
    }
    next
}

/// Asks Jira who the saved credentials belong to and stores the answer. A scoped token first needs the
/// site's cloud id; a site that doesn't tell it is `NotFound`, and the page then asks for it.
pub async fn test(settings: &SettingsStore, access: &JiraAccess) -> AppResult<ConnectionTest> {
    let mut connection = settings.get().connection;
    if connection.token_kind == TokenKind::Scoped && connection.cloud_id.is_none() {
        let tenant = access.client(&connection)?.tenant_info().await;
        connection.cloud_id = Some(tenant.map_err(cloud_id_error)?.cloud_id);
    }
    let myself = access.client(&connection)?.myself().await?;
    let tested = ConnectionTest {
        display_name: myself.display_name,
        account_id: myself.account_id,
        time_zone: myself.time_zone,
        cloud_id: connection.cloud_id.clone(),
    };
    let found = tested.clone();
    settings
        .edit(move |s| {
            // Saved only if the connection wasn't changed while Jira answered.
            if s.connection.same_account(&connection) {
                s.connection.cloud_id = found.cloud_id;
                s.connection.account_id = Some(found.account_id);
                s.connection.display_name = Some(found.display_name);
                s.connection.time_zone = Some(found.time_zone);
            }
        })
        .await?;
    Ok(tested)
}

fn cloud_id_error(e: AppError) -> AppError {
    match e {
        AppError::Offline(_) => e,
        _ => AppError::NotFound(
            "The site didn't tell its cloud id; enter it from admin.atlassian.com".into(),
        ),
    }
}
