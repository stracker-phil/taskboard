//! The Jira site a connection talks to and the URLs derived from it.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{AppError, AppResult};
use crate::net::{GATEWAY_HOST, is_fake_jira};

const SITE_SUFFIX: &str = ".atlassian.net";
const SITE_URL_FIELD: &str = "siteUrl";

/// Classic (unscoped) tokens call the site; scoped tokens call the gateway with the site's cloud id.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum TokenKind {
    #[default]
    Classic,
    Scoped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    base_url: Url,
    pub email: String,
    pub token_kind: TokenKind,
    pub cloud_id: Option<String>,
}

impl Site {
    pub fn new(
        site_url: &str,
        email: impl Into<String>,
        token_kind: TokenKind,
        cloud_id: Option<String>,
    ) -> AppResult<Self> {
        Ok(Self {
            base_url: normalize_site_url(site_url)?,
            email: email.into(),
            token_kind,
            cloud_id,
        })
    }

    /// `https://<site>.atlassian.net/`, always with the trailing slash.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Where REST API v3 paths are joined on; ends with `/`.
    pub fn api_base(&self) -> AppResult<Url> {
        let base = match self.token_kind {
            TokenKind::Classic => self.base_url.clone(),
            TokenKind::Scoped => {
                let cloud_id = self.cloud_id.as_deref().ok_or_else(|| {
                    AppError::NotConnected(
                        "A scoped token needs the site's cloud id; test the connection again"
                            .into(),
                    )
                })?;
                join(&self.gateway(), &format!("ex/jira/{cloud_id}/"))
            }
        };
        Ok(join(&base, "rest/api/3/"))
    }

    pub fn browse_url(&self, key: &str) -> Url {
        join(&self.base_url, &format!("browse/{key}"))
    }

    /// Answers the site's cloud id without credentials.
    pub fn tenant_info_url(&self) -> Url {
        join(&self.base_url, "_edge/tenant_info")
    }

    /// The hosts this site's requests may go to.
    pub fn hosts(&self) -> Vec<String> {
        let site = self.base_url.host_str().unwrap_or_default().to_string();
        vec![site, GATEWAY_HOST.to_string()]
    }

    /// The fake Jira serves the gateway's paths itself.
    fn gateway(&self) -> Url {
        if is_fake_jira(&self.base_url) {
            self.base_url.clone()
        } else {
            Url::parse(&format!("https://{GATEWAY_HOST}/")).expect("a valid gateway URL")
        }
    }
}

fn join(base: &Url, path: &str) -> Url {
    base.join(path)
        .expect("a relative path joins onto a base URL")
}

/// Accepts `example`, `example.atlassian.net` or any URL on the site (a pasted board link) and returns
/// the site's origin. Debug builds and tests also accept the fake Jira's `http://127.0.0.1:<port>`.
pub fn normalize_site_url(input: &str) -> AppResult<Url> {
    let input = input.trim();
    if input.is_empty() {
        return Err(site_error(
            "Enter your Jira site, like example.atlassian.net",
        ));
    }
    let url =
        Url::parse(&with_scheme(input)).map_err(|_| site_error("This is not a web address"))?;
    if !is_fake_jira(&url) && !is_cloud_site(&url) {
        return Err(site_error(
            "Use your Jira Cloud address, like https://example.atlassian.net",
        ));
    }
    Ok(origin(&url))
}

/// `example` becomes `https://example.atlassian.net`, a bare host gets `https://`.
fn with_scheme(input: &str) -> String {
    if input.contains("://") {
        input.to_string()
    } else if input.contains('.') || input.contains(':') {
        format!("https://{input}")
    } else {
        format!("https://{input}{SITE_SUFFIX}")
    }
}

/// `https://<name>.atlassian.net` without a port or credentials.
fn is_cloud_site(url: &Url) -> bool {
    let site_name = url
        .host_str()
        .and_then(|host| host.strip_suffix(SITE_SUFFIX))
        .unwrap_or_default();
    url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && !site_name.is_empty()
        && !site_name.contains('.')
}

fn origin(url: &Url) -> Url {
    Url::parse(&format!("{}/", url.origin().ascii_serialization())).expect("an origin is a URL")
}

fn site_error(message: &str) -> AppError {
    AppError::InvalidField {
        field: SITE_URL_FIELD.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLOUD_ID: &str = "11111111-2222-3333-4444-555555555555";

    fn classic() -> Site {
        Site::new(
            "example",
            "alex.morgan@example.com",
            TokenKind::Classic,
            None,
        )
        .unwrap()
    }

    fn site(url: &str, kind: TokenKind, cloud_id: Option<&str>) -> Site {
        Site::new(
            url,
            "alex.morgan@example.com",
            kind,
            cloud_id.map(Into::into),
        )
        .unwrap()
    }

    #[test]
    fn site_addresses_normalize_to_the_origin() {
        for input in [
            "example",
            "example.atlassian.net",
            "https://Example.atlassian.net/jira/software/projects/NW/boards/1?x=1",
            "  https://example.atlassian.net  ",
        ] {
            assert_eq!(
                normalize_site_url(input).unwrap().as_str(),
                "https://example.atlassian.net/",
                "{input}"
            );
        }
    }

    #[test]
    fn addresses_that_are_not_a_jira_cloud_site_are_refused_on_the_site_url_field() {
        for input in [
            "",
            "http://example.atlassian.net",
            "https://example.com",
            "https://a.b.atlassian.net",
            "https://example.atlassian.net:8443",
            "https://user@example.atlassian.net",
            "https://atlassian.net",
        ] {
            let err = normalize_site_url(input).unwrap_err();
            assert!(
                matches!(&err, AppError::InvalidField { field, .. } if field == "siteUrl"),
                "{input}: {err:?}"
            );
        }
    }

    #[test]
    fn the_fake_jira_on_loopback_is_accepted_in_tests() {
        assert_eq!(
            normalize_site_url("http://127.0.0.1:4567")
                .unwrap()
                .as_str(),
            "http://127.0.0.1:4567/"
        );
    }

    #[test]
    fn a_classic_token_calls_the_site() {
        assert_eq!(
            classic().api_base().unwrap().as_str(),
            "https://example.atlassian.net/rest/api/3/"
        );
    }

    #[test]
    fn a_scoped_token_calls_the_gateway_with_the_cloud_id() {
        let site = site("example", TokenKind::Scoped, Some(CLOUD_ID));
        assert_eq!(
            site.api_base().unwrap().as_str(),
            format!("https://api.atlassian.com/ex/jira/{CLOUD_ID}/rest/api/3/")
        );
    }

    #[test]
    fn a_scoped_token_without_a_cloud_id_is_not_connected() {
        let site = site("example", TokenKind::Scoped, None);
        assert!(matches!(site.api_base(), Err(AppError::NotConnected(_))));
    }

    #[test]
    fn a_scoped_loopback_site_serves_the_gateway_paths_itself() {
        let site = site("http://127.0.0.1:4567", TokenKind::Scoped, Some(CLOUD_ID));
        assert_eq!(
            site.api_base().unwrap().as_str(),
            format!("http://127.0.0.1:4567/ex/jira/{CLOUD_ID}/rest/api/3/")
        );
    }

    #[test]
    fn browse_and_tenant_info_urls_hang_off_the_site() {
        let site = classic();
        assert_eq!(
            site.browse_url("NW-12").as_str(),
            "https://example.atlassian.net/browse/NW-12"
        );
        assert_eq!(
            site.tenant_info_url().as_str(),
            "https://example.atlassian.net/_edge/tenant_info"
        );
    }

    #[test]
    fn the_allowed_hosts_are_the_site_and_the_gateway() {
        assert_eq!(
            classic().hosts(),
            vec![
                "example.atlassian.net".to_string(),
                "api.atlassian.com".to_string()
            ]
        );
    }

    #[test]
    fn token_kinds_serialize_in_camel_case() {
        assert_eq!(
            serde_json::to_value([TokenKind::Classic, TokenKind::Scoped]).unwrap(),
            serde_json::json!(["classic", "scoped"])
        );
    }
}
