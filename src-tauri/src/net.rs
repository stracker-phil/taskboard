//! What the network layer shares (ADR 0013, ADR 0020): the hosts, its answer and error types, and log
//! redaction. The only HTTP client lives in `jira::gateway` (ADR 0025); the webview never talks to the
//! network.

use url::Url;

/// The host scoped API tokens call; the other host is the configured Jira site.
pub const GATEWAY_HOST: &str = "api.atlassian.com";
const LOOPBACK: &str = "127.0.0.1";
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
    /// A redirect, an answer over the size cap, or a success that isn't JSON.
    #[error("{0}")]
    Unexpected(String),
}

/// Any HTTP answer, successful or not; the caller decides what a status means.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub status: u16,
    pub retry_after: Option<String>,
    pub rate_limit_reason: Option<String>,
    pub body: Vec<u8>,
}

/// Plain `http` to loopback, where the fake Jira runs; only tests and debug builds may use it.
pub(crate) fn is_fake_jira(url: &Url) -> bool {
    cfg!(any(test, debug_assertions)) && url.scheme() == "http" && url.host_str() == Some(LOOPBACK)
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
