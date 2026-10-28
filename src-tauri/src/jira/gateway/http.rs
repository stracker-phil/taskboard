//! The app's only HTTP client: JSON over HTTPS to the listed hosts, no redirects, answers size-capped.
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::sync::Arc;
use std::time::Duration;

use reqwest::Method;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use url::Url;

use crate::net::{Answer, NetError, is_fake_jira};

const TIMEOUT: Duration = Duration::from_secs(30);
/// Far above the largest page Taskboard asks for (100 issues with rendered fields).
pub const MAX_ANSWER_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone)]
pub(super) struct Http {
    client: reqwest::Client,
    hosts: Arc<[String]>,
}

impl Http {
    pub(super) fn new<S: Into<String>>(hosts: impl IntoIterator<Item = S>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .gzip(true)
            // A redirect could lead anywhere; the host check only sees the first URL.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("Taskboard/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|e| {
                log::warn!("The HTTP client fell back to its defaults: {e}");
                reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .unwrap_or_default()
            });
        Self {
            client,
            hosts: hosts.into_iter().map(Into::into).collect(),
        }
    }

    /// `https` to a listed host, or the fake Jira when its loopback address is listed.
    pub(super) fn allows(&self, url: &Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        self.hosts.iter().any(|h| h == host) && (url.scheme() == "https" || is_fake_jira(url))
    }

    /// Sends one request; `authorization` is the full header value and is never logged. A redirect, a
    /// successful answer that isn't JSON and an answer over `MAX_ANSWER_BYTES` fail.
    pub(super) async fn send(
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
        let mut response = request.send().await.map_err(transport_error)?;
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        let status = response.status().as_u16();
        if (300..400).contains(&status) {
            return Err(NetError::Unexpected(format!(
                "Jira answered HTTP {status} (a redirect), which Taskboard doesn't follow"
            )));
        }
        let is_json = header(CONTENT_TYPE.as_str())
            .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"));
        if (200..300).contains(&status) && !is_json {
            return Err(NetError::Unexpected(
                "Jira answered with something other than JSON".into(),
            ));
        }
        let retry_after = header("retry-after");
        let rate_limit_reason = header("ratelimit-reason");
        if response
            .content_length()
            .is_some_and(|length| length > MAX_ANSWER_BYTES as u64)
        {
            return Err(too_large());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if body.len() + chunk.len() > MAX_ANSWER_BYTES {
                return Err(too_large());
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Answer {
            status,
            retry_after,
            rate_limit_reason,
            body,
        })
    }
}

fn too_large() -> NetError {
    NetError::Unexpected(format!(
        "Jira's answer is larger than {} MB",
        MAX_ANSWER_BYTES / (1024 * 1024)
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    fn http(hosts: &[&str]) -> Http {
        Http::new(hosts.iter().copied())
    }

    #[test]
    fn https_to_a_listed_host_is_allowed() {
        let http = http(&["example.atlassian.net", "api.atlassian.com"]);
        for target in [
            "https://example.atlassian.net/rest/api/3/myself",
            "https://EXAMPLE.atlassian.net/rest/api/3/myself",
            "https://api.atlassian.com/ex/jira/abc/rest/api/3/myself",
        ] {
            assert!(http.allows(&url(target)), "{target}");
        }
    }

    #[test]
    fn unlisted_and_lookalike_hosts_are_refused() {
        let http = http(&["example.atlassian.net", "api.atlassian.com"]);
        for target in [
            "https://other.example.com/",
            "https://example.atlassian.net.example.com/",
            "https://api.atlassian.com.example.com/",
            "https://evilexample.atlassian.net/",
            "https://example.atlassian.net./",
            "https://xapi.atlassian.com/",
            "https://example.atlassian.net@evil.example.com/",
            "https://evil.example.com/example.atlassian.net",
            "https://localhost/",
        ] {
            assert!(!http.allows(&url(target)), "{target}");
        }
    }

    #[test]
    fn plain_http_and_other_schemes_are_refused_even_for_a_listed_host() {
        let http = http(&["example.atlassian.net", "api.atlassian.com"]);
        for target in [
            "http://example.atlassian.net/",
            "http://api.atlassian.com/",
            "ftp://example.atlassian.net/",
            "ws://example.atlassian.net/",
            "file:///etc/passwd",
            "data:text/plain,hi",
        ] {
            assert!(!http.allows(&url(target)), "{target}");
        }
    }

    #[test]
    fn plain_http_to_a_listed_loopback_address_is_allowed_in_tests() {
        let http = http(&["127.0.0.1"]);
        assert!(http.allows(&url("http://127.0.0.1:4567/rest/api/3/myself")));
    }

    #[test]
    fn loopback_is_refused_when_it_is_not_listed() {
        let http = http(&["example.atlassian.net", "api.atlassian.com"]);
        for target in [
            "http://127.0.0.1:4567/",
            "http://127.0.0.1/",
            "https://127.0.0.1/",
        ] {
            assert!(!http.allows(&url(target)), "{target}");
        }
    }

    #[test]
    fn localhost_is_refused_next_to_a_listed_loopback_address() {
        let http = http(&["127.0.0.1"]);
        for target in [
            "http://localhost:4567/",
            "http://127.0.0.2:4567/",
            "http://[::1]:4567/",
            "http://0.0.0.0:4567/",
        ] {
            assert!(!http.allows(&url(target)), "{target}");
        }
    }

    #[tokio::test]
    async fn sending_to_a_disallowed_host_fails_without_connecting() {
        let http = http(&["example.atlassian.net"]);
        for (target, host) in [
            (
                "https://evil.example.com/rest/api/3/myself",
                "evil.example.com",
            ),
            (
                "http://example.atlassian.net/rest/api/3/myself",
                "example.atlassian.net",
            ),
            ("http://127.0.0.1:9/", "127.0.0.1"),
        ] {
            let err = http
                .send(Method::GET, &url(target), Some("Basic c2VjcmV0"), None)
                .await
                .unwrap_err();
            assert_eq!(err, NetError::NotAllowed(host.into()), "{target}");
        }
    }

    #[tokio::test]
    async fn a_post_to_a_disallowed_host_fails_with_not_allowed() {
        let http = http(&["example.atlassian.net"]);
        let body = serde_json::json!({ "jql": "project = NW" });
        let err = http
            .send(
                Method::POST,
                &url("https://evil.example.com/rest/api/3/search/jql"),
                None,
                Some(&body),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, NetError::NotAllowed(_)), "{err:?}");
    }
}
