//! What a web link's address is: a pull request (GitHub, GitLab, Bitbucket) or a plain link, its host, and
//! whether the browser may open it. Pure, so the rules are tested on their own.

use serde::Serialize;
use ts_rs::TS;
use url::Url;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum WebLinkKind {
    /// A GitHub or Bitbucket pull request, or a GitLab merge request.
    PullRequest,
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classified {
    pub kind: WebLinkKind,
    /// The host without `www.`; none for an address that isn't http(s).
    pub host: Option<String>,
    /// `repo#481` for a pull request.
    pub reference: Option<String>,
    /// https without credentials: the only links `open_url` opens (ADR 0027).
    pub openable: bool,
}

pub fn classify(address: &str) -> Classified {
    let web = Url::parse(address)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"));
    let host = web.as_ref().and_then(Url::host_str).map(|host| {
        host.strip_prefix("www.")
            .unwrap_or(host)
            .to_ascii_lowercase()
    });
    let reference = web
        .as_ref()
        .zip(host.as_deref())
        .and_then(|(url, host)| pull_request(host, &segments(url)));
    Classified {
        kind: if reference.is_some() {
            WebLinkKind::PullRequest
        } else {
            WebLinkKind::Link
        },
        host,
        reference,
        openable: crate::commands::is_web_url(address),
    }
}

fn segments(url: &Url) -> Vec<&str> {
    url.path_segments()
        .map(|parts| parts.filter(|part| !part.is_empty()).collect())
        .unwrap_or_default()
}

/// `repo#n` when the path is a pull or merge request: `github.com/<owner>/<repo>/pull/<n>`,
/// `bitbucket.org/<workspace>/<repo>/pull-requests/<n>`, `<any GitLab>/<group…>/<repo>/-/merge_requests/<n>`.
fn pull_request(host: &str, path: &[&str]) -> Option<String> {
    let (repo, number) = match (host, path) {
        ("github.com", [_, repo, "pull", number, ..])
        | ("bitbucket.org", [_, repo, "pull-requests", number, ..]) => (*repo, *number),
        _ => {
            let at = path.iter().position(|part| *part == "-")?;
            match &path[at..] {
                ["-", "merge_requests", number, ..] if at >= 2 => (path[at - 1], *number),
                _ => return None,
            }
        }
    };
    let is_number = !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit());
    is_number.then(|| format!("{repo}#{number}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pull_request(host: &str, reference: &str, openable: bool) -> Classified {
        Classified {
            kind: WebLinkKind::PullRequest,
            host: Some(host.into()),
            reference: Some(reference.into()),
            openable,
        }
    }

    fn link(host: Option<&str>, openable: bool) -> Classified {
        Classified {
            kind: WebLinkKind::Link,
            host: host.map(Into::into),
            reference: None,
            openable,
        }
    }

    #[test]
    fn pull_requests_and_merge_requests_are_recognized_with_their_repo_and_number() {
        for (address, host, reference) in [
            (
                "https://github.com/example/shop/pull/481",
                "github.com",
                "shop#481",
            ),
            (
                "https://github.com/example/shop/pull/481/files",
                "github.com",
                "shop#481",
            ),
            (
                "https://www.github.com/example/shop/pull/481",
                "github.com",
                "shop#481",
            ),
            (
                "https://GitHub.com/example/shop/pull/7",
                "github.com",
                "shop#7",
            ),
            (
                "https://gitlab.com/example/store-api/-/merge_requests/58",
                "gitlab.com",
                "store-api#58",
            ),
            (
                "https://gitlab.example.com/shop/store-api/-/merge_requests/58",
                "gitlab.example.com",
                "store-api#58",
            ),
            (
                "https://gitlab.example.com/shop/backend/store-api/-/merge_requests/58/diffs",
                "gitlab.example.com",
                "store-api#58",
            ),
            (
                "https://bitbucket.org/example/shop-media/pull-requests/12",
                "bitbucket.org",
                "shop-media#12",
            ),
        ] {
            assert_eq!(
                classify(address),
                pull_request(host, reference, true),
                "{address}"
            );
        }
    }

    #[test]
    fn addresses_that_are_not_pull_requests_are_plain_links() {
        for (address, host) in [
            ("https://github.com/example/shop/issues/481", "github.com"),
            ("https://github.com/example/shop", "github.com"),
            ("https://github.com/example/shop/pull/", "github.com"),
            ("https://github.com/example/shop/pull/abc", "github.com"),
            ("https://github.com/example/shop/pull/12abc", "github.com"),
            ("https://github.com/pull/481", "github.com"),
            (
                "https://bitbucket.org/example/shop-media/pull-requests/new",
                "bitbucket.org",
            ),
            (
                "https://gitlab.example.com/shop/store-api/-/merge_requests/new",
                "gitlab.example.com",
            ),
            (
                "https://gitlab.example.com/shop/store-api/-/issues/58",
                "gitlab.example.com",
            ),
            (
                "https://gitlab.example.com/-/merge_requests/58",
                "gitlab.example.com",
            ),
            (
                "https://gitlab.example.com/shop/store-api/merge_requests/58",
                "gitlab.example.com",
            ),
            ("https://example.com/example/shop/pull/481", "example.com"),
        ] {
            assert_eq!(classify(address), link(Some(host), true), "{address}");
        }
    }

    #[test]
    fn a_plain_https_link_shows_its_host_without_www_and_opens() {
        for (address, host) in [
            (
                "https://docs.example.com/checkout/postcode-rules",
                "docs.example.com",
            ),
            ("https://www.example.com/spec", "example.com"),
            ("https://WWW.Example.COM/spec", "example.com"),
        ] {
            assert_eq!(classify(address), link(Some(host), true), "{address}");
        }
    }

    #[test]
    fn an_http_link_shows_its_host_but_does_not_open() {
        assert_eq!(
            classify("http://wiki.example.com/postcodes"),
            link(Some("wiki.example.com"), false)
        );
        assert_eq!(
            classify("http://github.com/example/shop/pull/481"),
            pull_request("github.com", "shop#481", false)
        );
    }

    #[test]
    fn an_https_link_with_credentials_is_not_openable() {
        for address in [
            "https://alex:secret@example.com/spec",
            "https://alex@example.com/spec",
        ] {
            assert_eq!(
                classify(address),
                link(Some("example.com"), false),
                "{address}"
            );
        }
    }

    #[test]
    fn an_address_that_is_not_http_has_no_host_and_does_not_open() {
        for address in [
            "javascript:alert(1)",
            "mailto:alex.morgan@example.com",
            "file:///etc/hosts",
            "ftp://example.com/spec",
            "data:text/html,hi",
        ] {
            assert_eq!(classify(address), link(None, false), "{address}");
        }
    }

    #[test]
    fn text_that_is_no_address_is_a_plain_link_that_does_not_open() {
        for address in ["", "   ", "not a url", "example.com/spec", "//example.com"] {
            assert_eq!(classify(address), link(None, false), "{address:?}");
        }
    }
}
