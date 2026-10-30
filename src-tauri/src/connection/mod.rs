//! Connecting to Jira: the connection settings, the token in the Keychain, Test Connection, the tracked
//! projects and the sync scope (`docs/jira.md`).

mod account;
pub mod commands;
mod expiry;
mod projects;
mod scope;

use std::sync::{Arc, Mutex};

pub use account::{ConnectionInput, ConnectionTest, ConnectionView, save, test, view};
pub use expiry::{TokenExpiry, token_expiry};
pub use projects::{
    ProjectRow, fetch_versions, list_projects, refresh_projects, store_spaces, track_project,
    tracked_keys, tracked_projects,
};
pub use scope::{
    ScopeView, effective_scope, save_intervals, save_scope, scope_view, validate_scope,
};

use crate::error::{AppError, AppResult};
use crate::jira::{JiraClient, Limiter};
use crate::lock::MutexExt;
use crate::secrets::{Remembered, Secret, SecretStore};
use crate::store::settings::Connection;

/// What every Jira request needs besides the settings: the Keychain, read once and kept in memory, the shared
/// rate limit and the client of the connected site, whose connections are reused.
#[derive(Clone)]
pub struct JiraAccess {
    secrets: Arc<Remembered>,
    limiter: Arc<Limiter>,
    client: Arc<Mutex<Option<Arc<JiraClient>>>>,
}

impl JiraAccess {
    pub fn new(secrets: Arc<dyn SecretStore>, limiter: Arc<Limiter>) -> Self {
        Self {
            secrets: Arc::new(Remembered::new(secrets)),
            limiter,
            client: Arc::default(),
        }
    }

    /// The same client while the connection's site stays the same.
    pub fn client(&self, connection: &Connection) -> AppResult<Arc<JiraClient>> {
        let site = connection.site()?;
        let mut cached = self.client.lock_unpoisoned();
        if let Some(client) = cached.as_ref().filter(|c| *c.site() == site) {
            return Ok(Arc::clone(client));
        }
        let client = Arc::new(JiraClient::new(
            site,
            self.secrets.clone(),
            self.limiter.clone(),
        ));
        *cached = Some(Arc::clone(&client));
        Ok(client)
    }

    /// Saving the connection reads the Keychain afresh and starts a new client.
    fn forget(&self) {
        self.secrets.forget();
        *self.client.lock_unpoisoned() = None;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeSecrets;

    fn access() -> JiraAccess {
        JiraAccess::new(
            Arc::new(FakeSecrets::default()),
            Arc::new(Limiter::new(1000, 1000)),
        )
    }

    fn connection(site_url: &str, email: &str) -> Connection {
        Connection {
            site_url: site_url.into(),
            email: email.into(),
            ..Connection::default()
        }
    }

    #[test]
    fn an_unchanged_connection_gets_the_same_client() {
        let access = access();
        let connection = connection(
            "https://northwind.atlassian.net/",
            "alex.morgan@example.com",
        );

        let first = access.client(&connection).unwrap();
        let second = access.client(&connection).unwrap();

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn a_clone_of_the_access_shares_the_client() {
        let access = access();
        let connection = connection(
            "https://northwind.atlassian.net/",
            "alex.morgan@example.com",
        );

        let first = access.client(&connection).unwrap();
        let second = access.clone().client(&connection).unwrap();

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn a_changed_site_or_email_gets_a_new_client() {
        let access = access();
        let original = connection(
            "https://northwind.atlassian.net/",
            "alex.morgan@example.com",
        );
        let first = access.client(&original).unwrap();

        for changed in [
            connection("https://contoso.atlassian.net/", "alex.morgan@example.com"),
            connection("https://northwind.atlassian.net/", "sam.lee@example.com"),
        ] {
            let next = access.client(&changed).unwrap();
            assert!(!Arc::ptr_eq(&first, &next), "{changed:?}");
        }
    }

    #[test]
    fn a_connection_that_is_not_set_up_has_no_client() {
        assert!(matches!(
            access().client(&Connection::default()),
            Err(AppError::NotConnected(_))
        ));
    }
}
