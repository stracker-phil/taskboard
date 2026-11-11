//! A GitHub pull request named by its address. Only `github.com/<owner>/<repo>/pull/<n>` qualifies, and owner
//! and repo must look like GitHub names: they become arguments of `gh`, and the address is untrusted (anyone who
//! can edit the issue sets it).

use url::Url;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GithubPr {
    pub owner: String,
    pub repo: String,
    pub number: u32,
}

impl GithubPr {
    pub fn parse(address: &str) -> Option<Self> {
        let url = Url::parse(address).ok()?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return None;
        }
        let host = url.host_str()?.to_ascii_lowercase();
        if host != "github.com" && host != "www.github.com" {
            return None;
        }
        let path: Vec<&str> = url
            .path_segments()?
            .filter(|part| !part.is_empty())
            .collect();
        let [owner, repo, "pull", number, ..] = path.as_slice() else {
            return None;
        };
        let number = number
            .bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| number.parse::<u32>().ok())
            .flatten()
            .filter(|n| *n > 0)?;
        (is_name(owner) && is_name(repo)).then(|| Self {
            owner: owner.to_ascii_lowercase(),
            repo: repo.to_ascii_lowercase(),
            number,
        })
    }

    /// `owner/repo`, as `gh --repo` takes it.
    pub fn repo_path(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

/// Letters, digits, `-`, `_` and `.`, not starting with `-` or `.`: never read as a flag or a path.
fn is_name(part: &str) -> bool {
    !part.is_empty()
        && part.len() <= 100
        && !part.starts_with(['-', '.'])
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(owner: &str, repo: &str, number: u32) -> GithubPr {
        GithubPr {
            owner: owner.into(),
            repo: repo.into(),
            number,
        }
    }

    #[test]
    fn a_github_pull_request_address_gives_owner_repo_and_number() {
        for (address, expected) in [
            (
                "https://github.com/example/shop/pull/481",
                pr("example", "shop", 481),
            ),
            (
                "https://www.github.com/example/shop/pull/481",
                pr("example", "shop", 481),
            ),
            (
                "https://GitHub.COM/example/shop/pull/481",
                pr("example", "shop", 481),
            ),
            (
                "https://github.com/example/shop/pull/481/files",
                pr("example", "shop", 481),
            ),
            (
                "https://github.com/example/shop/pull/481/",
                pr("example", "shop", 481),
            ),
            (
                "https://github.com/Example/Shop/pull/481",
                pr("example", "shop", 481),
            ),
            (
                "https://github.com/my-org_1/shop.v2/pull/7",
                pr("my-org_1", "shop.v2", 7),
            ),
            (
                "https://github.com/example/shop/pull/4294967295",
                pr("example", "shop", u32::MAX),
            ),
        ] {
            assert_eq!(GithubPr::parse(address), Some(expected), "{address}");
        }
    }

    #[test]
    fn addresses_that_are_not_github_pull_requests_are_rejected() {
        for address in [
            "http://github.com/example/shop/pull/481",
            "https://user@github.com/example/shop/pull/481",
            "https://user:secret@github.com/example/shop/pull/481",
            "https://gitlab.com/example/shop/pull/481",
            "https://bitbucket.org/example/shop/pull/481",
            "https://example.com/example/shop/pull/481",
            "https://github.com.example.com/example/shop/pull/481",
            "https://notgithub.com/example/shop/pull/481",
            "https://github.com/example/shop/issues/481",
            "https://github.com/example/shop/pull/",
            "https://github.com/example/shop/pull",
            "https://github.com/example/shop",
            "https://github.com/example",
            "https://github.com/example/shop/pull/abc",
            "https://github.com/example/shop/pull/12a",
            "https://github.com/example/shop/pull/-5",
            "https://github.com/example/shop/pull/0",
            "https://github.com/example/shop/pull/4294967296",
            "https://github.com/example/shop/pull/99999999999999999999",
            "not an address",
            "",
        ] {
            assert_eq!(GithubPr::parse(address), None, "{address}");
        }
    }

    #[test]
    fn owners_and_repos_that_could_pass_as_flags_or_paths_are_rejected() {
        for address in [
            "https://github.com/-example/shop/pull/1",
            "https://github.com/example/-shop/pull/1",
            "https://github.com/.example/shop/pull/1",
            "https://github.com/example/.shop/pull/1",
            "https://github.com/example/../pull/1",
            "https://github.com/%2Dexample/shop/pull/1",
            "https://github.com/example/%2Dshop/pull/1",
            "https://github.com/exa mple/shop/pull/1",
            "https://github.com/example/sh op/pull/1",
            "https://github.com/exam$ple/shop/pull/1",
            "https://github.com/example/shop;rm/pull/1",
        ] {
            assert_eq!(GithubPr::parse(address), None, "{address}");
        }
    }

    #[test]
    fn owners_and_repos_longer_than_a_github_name_are_rejected() {
        let long = "a".repeat(101);
        let longest = "a".repeat(100);

        assert_eq!(
            GithubPr::parse(&format!("https://github.com/{long}/shop/pull/1")),
            None
        );
        assert_eq!(
            GithubPr::parse(&format!("https://github.com/example/{long}/pull/1")),
            None
        );
        assert_eq!(
            GithubPr::parse(&format!("https://github.com/{longest}/shop/pull/1")),
            Some(pr(&longest, "shop", 1))
        );
    }

    #[test]
    fn the_repo_path_is_owner_slash_repo() {
        assert_eq!(pr("example", "shop", 481).repo_path(), "example/shop");
    }
}
