//! `JiraClient`: authenticated, rate-limited, retried JSON requests to one Jira site, with Jira's status
//! codes mapped to `AppError`.

use std::sync::{Arc, Mutex};

use base64::Engine;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

use super::limiter::Limiter;
use super::retry::{Retry, send_with_retries};
use super::site::Site;
use crate::error::{AppError, AppResult};
use crate::net::{Answer, Http, NetError, redact};
use crate::secrets::SecretStore;

/// Whether a `400` means Jira rejected the request's JQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BadRequest {
    Plain,
    Jql,
}

pub struct JiraClient {
    http: Http,
    site: Site,
    secrets: Arc<dyn SecretStore>,
    limiter: Arc<Limiter>,
    rng: Mutex<fastrand::Rng>,
}

impl JiraClient {
    /// `limiter` is shared by every client, so a reconnect can't double the request rate.
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

    /// `path` is relative to the REST API base (`myself`, `search/jql`).
    pub(crate) async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> AppResult<T> {
        let mut url = self.api_url(path)?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        self.call(Method::GET, &url, None, BadRequest::Plain, true)
            .await
    }

    /// A read that Jira takes as a POST (searches, counts, bulk fetches); retried like a GET.
    pub(crate) async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
        json: &Value,
        bad_request: BadRequest,
    ) -> AppResult<T> {
        let mut url = self.api_url(path)?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        self.call(Method::POST, &url, Some(json), bad_request, true)
            .await
    }

    /// A GET outside the REST API, without credentials (`_edge/tenant_info`).
    pub(crate) async fn get_public<T: DeserializeOwned>(&self, url: &Url) -> AppResult<T> {
        self.call(Method::GET, url, None, BadRequest::Plain, false)
            .await
    }

    fn api_url(&self, path: &str) -> AppResult<Url> {
        self.site
            .api_base()?
            .join(path)
            .map_err(|e| AppError::Internal(format!("Bad Jira path {path}: {e}")))
    }

    async fn call<T: DeserializeOwned>(
        &self,
        method: Method,
        url: &Url,
        json: Option<&Value>,
        bad_request: BadRequest,
        authenticated: bool,
    ) -> AppResult<T> {
        let authorization = if authenticated {
            Some(self.authorization()?)
        } else {
            None
        };
        // Reads are safe to replay; a write must pass `Retry::Never`.
        let answer = send_with_retries(Retry::Allowed, &self.rng, || async {
            self.limiter.acquire().await;
            self.http
                .send(method.clone(), url, authorization.as_deref(), json)
                .await
        })
        .await
        .map_err(|e| self.net_error(e))?;
        self.log(&format!("{method} {} → {}", url.path(), answer.status));
        if !(200..300).contains(&answer.status) {
            return Err(status_error(&answer, bad_request, url.path()));
        }
        serde_json::from_slice(&answer.body).map_err(|e| {
            AppError::Internal(format!(
                "Jira sent an unexpected answer to {}: {e}",
                url.path()
            ))
        })
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
            NetError::NotAllowed(_) | NetError::Request(_) => AppError::Internal(message),
        }
    }

    fn log(&self, line: &str) {
        log::debug!("{}", redact(line, &self.site.email));
    }
}

/// What a non-2xx answer means for the caller.
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
