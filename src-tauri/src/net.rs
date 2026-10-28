//! The only way out to the network (ADR 0013, ADR 0020): JSON requests over HTTPS to the hosts the Jira
//! connection needs. No telemetry, no update check; the webview never talks to the network.

use std::sync::Arc;
use std::time::Duration;

use reqwest::Method;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderValue};
use url::Url;

/// The host scoped API tokens call; the other host is the configured Jira site.
pub const GATEWAY_HOST: &str = "api.atlassian.com";
const LOOPBACK: &str = "127.0.0.1";
const TIMEOUT: Duration = Duration::from_secs(30);
const REDACTED: &str = "[redacted]";

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum NetError {
    #[error("{0} is not a host Taskboard talks to")]
    NotAllowed(String),
    /// No connection: DNS, refused, unreachable or timed out.
    #[error("{0}")]
    Offline(String),
    #[error("{0}")]
    Request(String),
}

/// Any HTTP answer, successful or not; the caller decides what a status means.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub status: u16,
    pub retry_after: Option<String>,
    pub rate_limit_reason: Option<String>,
    pub body: Vec<u8>,
}

#[derive(Clone)]
pub struct Http {
    client: reqwest::Client,
    hosts: Arc<[String]>,
}

impl Http {
    pub fn new<S: Into<String>>(hosts: impl IntoIterator<Item = S>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .gzip(true)
            .user_agent(concat!("Taskboard/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|e| {
                log::warn!("The HTTP client fell back to its defaults: {e}");
                reqwest::Client::new()
            });
        Self {
            client,
            hosts: hosts.into_iter().map(Into::into).collect(),
        }
    }

    /// `https` to a listed host, or the fake Jira when its loopback address is listed.
    pub fn allows(&self, url: &Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        self.hosts.iter().any(|h| h == host) && (url.scheme() == "https" || is_fake_jira(url))
    }

    /// Sends one request; `authorization` is the full header value and is never logged.
    pub async fn send(
        &self,
        method: Method,
        url: &Url,
        authorization: Option<&str>,
        json: Option<&serde_json::Value>,
    ) -> Result<Answer, NetError> {
        if !self.allows(url) {
            return Err(NetError::NotAllowed(url.host_str().unwrap_or("").into()));
        }
        let mut request = self
            .client
            .request(method, url.clone())
            .header(ACCEPT, "application/json");
        if let Some(value) = authorization {
            let mut header =
                HeaderValue::from_str(value).map_err(|e| NetError::Request(e.to_string()))?;
            header.set_sensitive(true);
            request = request.header(AUTHORIZATION, header);
        }
        if let Some(body) = json {
            request = request.json(body);
        }
        let response = request.send().await.map_err(transport_error)?;
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        let status = response.status().as_u16();
        let retry_after = header("retry-after");
        let rate_limit_reason = header("ratelimit-reason");
        let body = response.bytes().await.map_err(transport_error)?.to_vec();
        Ok(Answer {
            status,
            retry_after,
            rate_limit_reason,
            body,
        })
    }
}

/// Plain `http` to loopback, where the fake Jira runs; only tests and debug builds may use it.
pub(crate) fn is_fake_jira(url: &Url) -> bool {
    cfg!(any(test, debug_assertions)) && url.scheme() == "http" && url.host_str() == Some(LOOPBACK)
}

/// Without the URL, which may name a private site.
fn transport_error(e: reqwest::Error) -> NetError {
    let offline = e.is_connect() || e.is_timeout();
    let message = e.without_url().to_string();
    if offline {
        NetError::Offline(message)
    } else {
        NetError::Request(message)
    }
}

/// A log line without credentials: the value of an `Authorization` header, any `Basic`/`Bearer` token
/// and the account's email.
pub fn redact(line: &str, email: &str) -> String {
    let without_headers: String = line
        .split_inclusive('\n')
        .map(redact_authorization_header)
        .collect();
    let without_tokens = redact_after_word(&without_headers, "basic ");
    let without_tokens = redact_after_word(&without_tokens, "bearer ");
    replace_ignoring_case(&without_tokens, email, "[email]")
}

fn redact_authorization_header(line: &str) -> String {
    let Some(at) = line.to_ascii_lowercase().find("authorization") else {
        return line.to_string();
    };
    let end = at + "authorization".len();
    let newline = if line.ends_with('\n') { "\n" } else { "" };
    format!("{}: {REDACTED}{newline}", &line[..end])
}

/// Replaces a token-like word after each (case-insensitive) `prefix`; "basic auth" stays readable.
fn redact_after_word(text: &str, prefix: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut rest = 0;
    while let Some(found) = lower[rest..].find(prefix) {
        let start = rest + found + prefix.len();
        let end = text[start..]
            .find(|c: char| c.is_whitespace() || c == '"' || c == ',')
            .map_or(text.len(), |n| start + n);
        out.push_str(&text[rest..start]);
        out.push_str(if looks_like_token(&text[start..end]) {
            REDACTED
        } else {
            &text[start..end]
        });
        rest = end;
    }
    out.push_str(&text[rest..]);
    out
}

fn looks_like_token(word: &str) -> bool {
    word.len() >= 8
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+/=-_.".contains(c))
}

fn replace_ignoring_case(text: &str, needle: &str, with: &str) -> String {
    if needle.is_empty() {
        return text.to_string();
    }
    let lower = text.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut rest = 0;
    while let Some(found) = lower[rest..].find(&needle) {
        out.push_str(&text[rest..rest + found]);
        out.push_str(with);
        rest += found + needle.len();
    }
    out.push_str(&text[rest..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMAIL: &str = "alex.morgan@example.com";

    fn http(hosts: &[&str]) -> Http {
        Http::new(hosts.iter().copied())
    }

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    #[test]
    fn https_is_allowed_to_a_listed_host_only() {
        let http = http(&["example.atlassian.net", GATEWAY_HOST]);
        for (target, allowed) in [
            ("https://example.atlassian.net/rest/api/3/myself", true),
            ("https://api.atlassian.com/ex/jira/abc/rest/api/3/", true),
            ("https://evil.example/", false),
            ("https://example.atlassian.net.evil.example/", false),
            ("https://other.atlassian.net/", false),
        ] {
            assert_eq!(http.allows(&url(target)), allowed, "{target}");
        }
    }

    #[test]
    fn plain_http_and_other_schemes_are_refused_for_listed_hosts() {
        let http = http(&["example.atlassian.net"]);
        for target in [
            "http://example.atlassian.net/",
            "ftp://example.atlassian.net/",
        ] {
            assert!(!http.allows(&url(target)), "{target}");
        }
    }

    #[test]
    fn plain_http_to_a_listed_loopback_address_is_allowed_in_tests() {
        let http = http(&["example.atlassian.net", "127.0.0.1"]);
        assert!(http.allows(&url("http://127.0.0.1:4567/rest/api/3/myself")));
    }

    #[test]
    fn loopback_is_refused_when_it_is_not_listed() {
        let http = http(&["example.atlassian.net"]);
        assert!(!http.allows(&url("http://127.0.0.1:4567/")));
    }

    #[test]
    fn localhost_is_refused_even_next_to_a_listed_loopback_address() {
        let http = http(&["example.atlassian.net", "127.0.0.1"]);
        assert!(!http.allows(&url("http://localhost:4567/")));
    }

    #[tokio::test]
    async fn sending_to_a_disallowed_host_fails_without_connecting() {
        let http = http(&["example.atlassian.net"]);
        let result = http
            .send(Method::GET, &url("https://evil.example/steal"), None, None)
            .await;
        assert_eq!(result, Err(NetError::NotAllowed("evil.example".into())));
    }

    #[test]
    fn an_authorization_header_shows_neither_token_nor_email() {
        let line = format!("Authorization: Basic dGVzdDp0b2tlbg== for {EMAIL}");
        let redacted = redact(&line, EMAIL);
        assert_eq!(redacted, "Authorization: [redacted]");
    }

    #[test]
    fn the_email_is_hidden_in_any_case() {
        assert_eq!(
            redact(
                "user ALEX.Morgan@Example.com sent Authorization: Basic dGVzdDp0b2tlbg==",
                EMAIL
            ),
            "user [email] sent Authorization: [redacted]"
        );
    }

    #[test]
    fn a_basic_token_outside_a_header_is_removed_and_the_email_hidden() {
        assert_eq!(
            redact(&format!("sent Basic dGVzdDp0b2tlbg== for {EMAIL}"), EMAIL),
            "sent Basic [redacted] for [email]"
        );
    }

    #[test]
    fn a_bearer_token_is_removed() {
        assert_eq!(
            redact("token=Bearer abcdefgh12345.xyz end", EMAIL),
            "token=Bearer [redacted] end"
        );
    }

    #[test]
    fn plain_prose_about_basic_auth_stays_readable() {
        assert_eq!(redact("basic auth failed", EMAIL), "basic auth failed");
    }

    #[test]
    fn an_empty_email_changes_nothing_else() {
        assert_eq!(
            redact("sent Basic dGVzdDp0b2tlbg== to example", ""),
            "sent Basic [redacted] to example"
        );
        assert_eq!(redact("nothing secret here", ""), "nothing secret here");
    }
}
