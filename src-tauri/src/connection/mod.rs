//! Connecting to Jira: the connection settings, the token in the Keychain, Test Connection, the tracked
//! projects and the sync scope (`docs/jira.md`).

mod account;
pub mod commands;
mod expiry;
mod projects;
mod scope;

use std::sync::Arc;

pub use account::{ConnectionInput, ConnectionTest, ConnectionView, save, test, view};
pub use expiry::{TokenExpiry, token_expiry};
pub use projects::{
    ProjectRow, fetch_versions, list_projects, refresh_projects, track_project, tracked_keys,
    tracked_projects,
};
pub use scope::{
    ScopeView, effective_scope, save_intervals, save_scope, scope_view, validate_scope,
};

use crate::error::{AppError, AppResult};
use crate::jira::{JiraClient, Limiter};
use crate::secrets::{Secret, SecretStore};
use crate::store::settings::Connection;

/// What every Jira request needs besides the settings: the Keychain and the shared rate limit.
#[derive(Clone)]
pub struct JiraAccess {
    secrets: Arc<dyn SecretStore>,
    limiter: Arc<Limiter>,
}

impl JiraAccess {
    pub fn new(secrets: Arc<dyn SecretStore>, limiter: Arc<Limiter>) -> Self {
        Self { secrets, limiter }
    }

    pub fn client(&self, connection: &Connection) -> AppResult<JiraClient> {
        Ok(JiraClient::new(
            connection.site()?,
            self.secrets.clone(),
            self.limiter.clone(),
        ))
    }

    /// An unreadable Keychain entry counts as none.
    pub fn has_token(&self, email: &str) -> bool {
        !email.is_empty() && self.secrets.get(email).is_ok_and(|token| token.is_some())
    }

    fn store_token(&self, email: &str, token: &str) -> AppResult<()> {
        self.secrets
            .set(email, &Secret::new(token))
            .map_err(keychain_error)
    }

    /// Keeps the token when only the email changes.
    fn move_token(&self, from: &str, to: &str) -> AppResult<()> {
        let Some(token) = self.secrets.get(from).map_err(keychain_error)? else {
            return Ok(());
        };
        self.secrets.set(to, &token).map_err(keychain_error)?;
        self.secrets.remove(from).map_err(keychain_error)
    }
}

fn keychain_error(e: String) -> AppError {
    AppError::Internal(format!("The Keychain failed: {e}"))
}
