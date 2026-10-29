//! The one door to Jira (ADR 0025): every request is an allowlisted `Endpoint`; profile pictures come only
//! from Atlassian's avatar host through `AvatarClient` (ADR 0032).

mod avatar;
mod endpoint;
mod guard;
mod http;
mod params;
#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex};

use base64::Engine;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub use avatar::{AvatarClient, Image, ImageKind, MAX_AVATAR_BYTES, check_image};
pub use endpoint::{Access, BadRequest, Endpoint, PAGE_SIZE, Planned};
pub use guard::check;
pub use http::MAX_ANSWER_BYTES;
pub use params::{
    BULK_FETCH_MAX, Expand, FieldIds, IssueRef, IssueRefs, Jql, PageToken, ProjectKey,
};

use super::limiter::Limiter;
use super::retry::{Retry, send_with_retries};
use super::site::Site;
use crate::error::{AppError, AppResult};
use crate::net::{Answer, NetError, redact};
use crate::secrets::SecretStore;
use http::Http;

pub struct Gateway {
    http: Http,
    site: Site,
    secrets: Arc<dyn SecretStore>,
    limiter: Arc<Limiter>,
    rng: Mutex<fastrand::Rng>,
}

impl Gateway {
    pub fn new(site: Site, secrets: Arc<dyn SecretStore>, limiter: Arc<Limiter>) -> Self {
        Self {
            http: Http::new(site.hosts()),
            site,
            secrets,
            limiter,
            rng: Mutex::new(fastrand::Rng::new()),
        }
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub async fn send<T: DeserializeOwned>(&self, endpoint: Endpoint) -> AppResult<T> {
        let planned = endpoint.plan(&self.site)?;
        if let Err(refused) = check(&planned, &self.site) {
            self.log(&refused.to_string());
            return Err(refused);
        }
        let authorization = if planned.authenticated {
            Some(self.authorization()?)
        } else {
            None
        };
        let answer = self.fetch(&planned, authorization.as_deref()).await?;
        let path = planned.url.path();
        self.log(&format!("{} {path} → {}", planned.method, answer.status));
        if !(200..300).contains(&answer.status) {
            return Err(status_error(&answer, planned.bad_request, path));
        }
        serde_json::from_slice(&answer.body).map_err(|e| {
            AppError::Internal(format!("Jira sent an unexpected answer to {path}: {e}"))
        })
    }

    /// Rate-limited; a read is retried on `429`/`503`, a write never.
    async fn fetch(&self, planned: &Planned, authorization: Option<&str>) -> AppResult<Answer> {
        let retry = match planned.access {
            Access::Read => Retry::Allowed,
            Access::Write => Retry::Never,
        };
        send_with_retries(retry, &self.rng, || async {
            self.limiter.acquire().await;
            self.http
                .send(
                    planned.method.clone(),
                    &planned.url,
                    authorization,
                    planned.body.as_ref(),
                )
                .await
        })
        .await
        .map_err(|e| self.net_error(e))
    }

    /// Built per request so a token changed in the Keychain is picked up.
    fn authorization(&self) -> AppResult<String> {
        let token = self
            .secrets
            .get(&self.site.email)
            .map_err(|e| AppError::Internal(format!("The Keychain failed: {e}")))?
            .ok_or_else(|| AppError::NotConnected("Add your API token in Settings".into()))?;
        let pair = zeroize::Zeroizing::new(format!("{}:{}", self.site.email, token.expose()));
        let encoded = base64::engine::general_purpose::STANDARD.encode(pair.as_bytes());
        Ok(format!("Basic {encoded}"))
    }

    fn net_error(&self, e: NetError) -> AppError {
        let message = redact(&e.to_string(), &self.site.email);
        match e {
            NetError::Offline(_) => AppError::Offline(format!("Jira can't be reached: {message}")),
            NetError::NotAllowed(_) | NetError::Request(_) | NetError::Unexpected(_) => {
                AppError::Internal(message)
            }
        }
    }

    fn log(&self, line: &str) {
        log::debug!("{}", redact(line, &self.site.email));
    }
}

/// A `400` is `InvalidJql` only on the endpoints that take JQL.
fn status_error(answer: &Answer, bad_request: BadRequest, path: &str) -> AppError {
    match answer.status {
        401 => AppError::Auth("Jira didn't accept the email and API token".into()),
        403 => AppError::Auth("Jira refused access; check the token and its scopes".into()),
        400 if bad_request == BadRequest::Jql => AppError::InvalidJql(error_messages(&answer.body)),
        404 => AppError::NotFound(format!("Jira has no {path}")),
        status => AppError::Internal(format!("Jira answered HTTP {status} to {path}")),
    }
}

/// Jira's `errorMessages` and the values of `errors`.
fn error_messages(body: &[u8]) -> Vec<String> {
    let parsed: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let listed = parsed["errorMessages"].as_array().into_iter().flatten();
    let by_field = parsed["errors"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values());
    let messages: Vec<String> = listed
        .chain(by_field)
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    if messages.is_empty() {
        vec!["Jira rejected the query".into()]
    } else {
        messages
    }
}
