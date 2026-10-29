//! The app's only HTTP client: JSON over HTTPS to the listed hosts and pictures from Atlassian's avatar host,
//! no redirects, answers size-capped.
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::sync::Arc;
use std::time::Duration;

use reqwest::Method;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use url::Url;

use super::avatar::{Image, MAX_AVATAR_BYTES, check_image};
use crate::net::{Answer, AvatarUrl, NetError, is_fake_jira};

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
        let body = read_capped(&mut response, MAX_ANSWER_BYTES, too_large).await?;
        Ok(Answer {
            status,
            retry_after,
            rate_limit_reason,
            body,
        })
    }

    /// GETs a profile picture (ADR 0032) without `Authorization` or cookie; a redirect or an answer that fails
    /// `check_image` fails.
    pub(super) async fn get_image(&self, url: &AvatarUrl) -> Result<Image, NetError> {
        let request = self.image_request(url)?;
        let mut response = self
            .client
            .execute(request)
            .await
            .map_err(transport_error)?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(NetError::Unexpected(format!(
                "The avatar host answered HTTP {status}"
            )));
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        if response
            .content_length()
            .is_some_and(|length| length > MAX_AVATAR_BYTES as u64)
        {
            return Err(super::avatar::too_large());
        }
        let body = read_capped(&mut response, MAX_AVATAR_BYTES, super::avatar::too_large).await?;
        check_image(content_type.as_deref(), body)
    }

    /// The request `get_image` sends, checked against the avatar host rule again right before the wire.
    pub(super) fn image_request(&self, url: &AvatarUrl) -> Result<reqwest::Request, NetError> {
        if AvatarUrl::parse(url.as_str()).is_none() {
            return Err(NetError::NotAllowed(
                url.as_url().host_str().unwrap_or("").into(),
            ));
        }
        self.client
            .get(url.as_url().clone())
            .header(ACCEPT, "image/png, image/jpeg, image/gif, image/webp")
            .build()
            .map_err(|e| NetError::Request(e.without_url().to_string()))
    }
}

/// The body, failing with `too_large()` once it passes `cap` bytes.
async fn read_capped(
    response: &mut reqwest::Response,
    cap: usize,
    too_large: fn() -> NetError,
) -> Result<Vec<u8>, NetError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if body.len() + chunk.len() > cap {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
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

    const AVATAR: &str = "https://avatar-management--avatars.us-west-2.prod.public.atl-paas.net/712020:aaaa/0f1e2d3c/48";

    #[test]
    fn an_avatar_request_carries_no_credentials_or_cookies() {
        let http = http(&["example.atlassian.net", "api.atlassian.com"]);
        let request = http
            .image_request(&AvatarUrl::parse(AVATAR).unwrap())
            .unwrap();

        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.url().as_str(), AVATAR);
        assert!(request.headers().get(AUTHORIZATION).is_none());
        assert!(request.headers().get(reqwest::header::COOKIE).is_none());
        assert!(request.body().is_none());
        assert_eq!(
            request.headers().get(ACCEPT).unwrap(),
            "image/png, image/jpeg, image/gif, image/webp"
        );
    }

    #[test]
    fn the_avatar_host_is_not_a_jira_host() {
        let http = http(&["example.atlassian.net", "api.atlassian.com"]);
        assert!(!http.allows(&url(AVATAR)));
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
