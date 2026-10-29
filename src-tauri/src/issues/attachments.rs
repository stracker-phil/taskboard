//! An issue's attachments, read from the `attachment` field the sync stores with the issue. Quick Look shows
//! a downloaded copy (`preview`); the browser opens the address on the Jira site, where the owner is signed in.

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;
use url::Url;

use crate::jira::dto;
use crate::sync::map::utc;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    /// Plain text: the page never renders it as HTML.
    pub filename: String,
    /// The extension in caps ("PDF"), else "FILE".
    pub badge: String,
    /// "1.2 MB"; none when Jira sent no size.
    pub size: Option<String>,
    pub mime_type: Option<String>,
    /// UTC, RFC 3339.
    pub created_at: Option<String>,
    pub author_name: Option<String>,
    /// The download address, only when it is on the connected Jira site (see [`keep_site_urls`]).
    pub url: Option<String>,
}

/// The attachments in the issue's raw JSON (as the cache stores it), in Jira's order; entries Jira sent
/// without an id are skipped.
pub fn attachments_of(raw: &str) -> Vec<Attachment> {
    let raw: Value = serde_json::from_str(raw).unwrap_or_default();
    raw["fields"]["attachment"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| serde_json::from_value::<dto::Attachment>(item.clone()).ok())
        .map(|a| Attachment {
            badge: badge(&a.filename),
            size: a.size.map(human_size),
            created_at: a
                .created
                .as_ref()
                .and_then(|c| utc(&Value::from(c.as_str()))),
            author_name: a.author.and_then(|p| p.display_name),
            url: a.content,
            mime_type: a.mime_type,
            filename: a.filename,
            id: a.id,
        })
        .collect()
}

/// The attachment with this id, as Jira listed it.
pub fn find_attachment(raw: &str, id: &str) -> Option<dto::Attachment> {
    let raw: Value = serde_json::from_str(raw).ok()?;
    raw["fields"]["attachment"]
        .as_array()?
        .iter()
        .filter_map(|item| serde_json::from_value::<dto::Attachment>(item.clone()).ok())
        .find(|a| a.id == id)
}

/// Only links to the connected site reach the page.
pub fn keep_site_urls(attachments: &mut [Attachment], site_url: &str) {
    for attachment in attachments {
        if !attachment
            .url
            .as_deref()
            .is_some_and(|url| is_on_site(url, site_url))
        {
            attachment.url = None;
        }
    }
}

pub fn is_on_site(url: &str, site_url: &str) -> bool {
    let (Ok(url), Ok(site)) = (Url::parse(url), Url::parse(site_url)) else {
        return false;
    };
    url.scheme() == "https"
        && site.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some()
        && url.host_str() == site.host_str()
        && url.port_or_known_default() == site.port_or_known_default()
}

fn badge(filename: &str) -> String {
    filename
        .rsplit_once('.')
        .map(|(stem, ext)| (stem, ext.trim()))
        .filter(|(stem, ext)| {
            !stem.is_empty()
                && !ext.is_empty()
                && ext.len() <= 5
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .map_or_else(|| "FILE".into(), |(_, ext)| ext.to_ascii_uppercase())
}

/// Decimal units, as Finder shows them: "820 B", "48 KB", "1.2 MB".
fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    while value >= 999.5 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if value < 9.95 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SITE: &str = "https://example.atlassian.net/";

    fn raw_with(attachments: Value) -> String {
        json!({ "fields": { "attachment": attachments } }).to_string()
    }

    fn item(filename: &str, size: Option<u64>) -> Value {
        let mut item = json!({ "id": "80001", "filename": filename });
        if let Some(size) = size {
            item["size"] = size.into();
        }
        item
    }

    fn with_url(url: Option<&str>) -> Attachment {
        Attachment {
            id: "80001".into(),
            filename: "a.pdf".into(),
            badge: "PDF".into(),
            size: None,
            mime_type: None,
            created_at: None,
            author_name: None,
            url: url.map(Into::into),
        }
    }

    #[test]
    fn a_full_attachment_is_mapped_to_what_the_page_shows() {
        let raw = raw_with(json!([{
            "id": "80001",
            "filename": "release-notes.pdf",
            "size": 1_240_000,
            "mimeType": "application/pdf",
            "created": "2026-10-12T09:30:00.000+0200",
            "author": { "accountId": "712020:abc", "displayName": "Marta Klein" },
            "content": "https://example.atlassian.net/rest/api/3/attachment/content/80001",
        }]));

        assert_eq!(
            attachments_of(&raw),
            [Attachment {
                id: "80001".into(),
                filename: "release-notes.pdf".into(),
                badge: "PDF".into(),
                size: Some("1.2 MB".into()),
                mime_type: Some("application/pdf".into()),
                created_at: Some("2026-10-12T07:30:00.000Z".into()),
                author_name: Some("Marta Klein".into()),
                url: Some(
                    "https://example.atlassian.net/rest/api/3/attachment/content/80001".into()
                ),
            }]
        );
    }

    #[test]
    fn the_attachment_serializes_with_camel_case_keys() {
        let value = serde_json::to_value(with_url(None)).unwrap();

        for key in ["mimeType", "createdAt", "authorName"] {
            assert!(value.get(key).is_some(), "{key}");
        }
    }

    #[test]
    fn the_badge_is_the_extension_in_caps_or_file() {
        for (filename, expected) in [
            ("release-notes.pdf", "PDF"),
            ("archive.tar.gz", "GZ"),
            ("Photo.JPeg", "JPEG"),
            ("README", "FILE"),
            (".env", "FILE"),
            ("trailing.", "FILE"),
            ("odd.ext-name", "FILE"),
            ("long.extension", "FILE"),
            ("", "FILE"),
        ] {
            let found = attachments_of(&raw_with(json!([item(filename, None)])));

            assert_eq!(found[0].badge, expected, "{filename:?}");
        }
    }

    #[test]
    fn the_size_is_shown_in_decimal_units() {
        for (bytes, expected) in [
            (0, "0 B"),
            (820, "820 B"),
            (999, "999 B"),
            (1_000, "1.0 KB"),
            (48_000, "48 KB"),
            (1_240_000, "1.2 MB"),
            (5_600_000, "5.6 MB"),
            (999_999, "1.0 MB"),
            (2_500_000_000, "2.5 GB"),
        ] {
            let found = attachments_of(&raw_with(json!([item("a.bin", Some(bytes))])));

            assert_eq!(found[0].size.as_deref(), Some(expected), "{bytes}");
        }
    }

    #[test]
    fn a_missing_author_and_size_stay_empty() {
        let found = attachments_of(&raw_with(json!([item("notes.txt", None)])));

        assert_eq!(found[0].size, None);
        assert_eq!(found[0].author_name, None);
        assert_eq!(found[0].created_at, None);
        assert_eq!(found[0].url, None);
    }

    #[test]
    fn entries_without_an_id_are_skipped_and_the_order_is_kept() {
        let raw = raw_with(json!([
            { "id": "2", "filename": "b.txt" },
            { "filename": "no-id.txt" },
            { "id": "1", "filename": "a.txt" },
        ]));

        let ids: Vec<String> = attachments_of(&raw).into_iter().map(|a| a.id).collect();

        assert_eq!(ids, ["2", "1"]);
    }

    #[test]
    fn an_issue_without_the_attachment_field_has_none() {
        assert_eq!(attachments_of(&json!({ "fields": {} }).to_string()), []);
        assert_eq!(attachments_of("{}"), []);
        assert_eq!(attachments_of(&raw_with(json!([]))), []);
    }

    #[test]
    fn unparsable_raw_json_has_no_attachments() {
        assert_eq!(attachments_of("not json"), []);
        assert_eq!(attachments_of(""), []);
    }

    #[test]
    fn an_attachment_is_found_by_its_id_as_jira_listed_it() {
        let raw = raw_with(json!([
            item("a.txt", None),
            {
                "id": "80002",
                "filename": "release-notes.pdf",
                "size": 1_240_000,
                "mimeType": "application/pdf",
            },
        ]));

        let found = find_attachment(&raw, "80002").unwrap();

        assert_eq!(found.id, "80002");
        assert_eq!(found.filename, "release-notes.pdf");
        assert_eq!(found.size, Some(1_240_000));
        assert_eq!(found.mime_type.as_deref(), Some("application/pdf"));
        assert_eq!(find_attachment(&raw, "80001").unwrap().filename, "a.txt");
    }

    #[test]
    fn no_attachment_is_found_for_an_unknown_id_or_unusable_raw_json() {
        let raw = raw_with(json!([item("a.txt", None)]));

        assert_eq!(find_attachment(&raw, "99999"), None);
        assert_eq!(find_attachment(&raw, ""), None);
        assert_eq!(find_attachment(&raw_with(json!([])), "80001"), None);
        assert_eq!(find_attachment("{}", "80001"), None);
        assert_eq!(find_attachment("not json", "80001"), None);
        assert_eq!(find_attachment("", "80001"), None);
    }

    #[test]
    fn only_https_urls_on_exactly_the_site_host_are_on_the_site() {
        for (url, expected) in [
            (
                "https://example.atlassian.net/rest/api/3/attachment/content/1",
                true,
            ),
            ("https://example.atlassian.net:443/secure/a.png", true),
            ("http://example.atlassian.net/secure/a.png", false),
            ("https://api.atlassian.com/ex/jira/abc/attachment/1", false),
            ("https://other.atlassian.net/secure/a.png", false),
            (
                "https://example.atlassian.net.evil.example.com/a.png",
                false,
            ),
            ("https://user:secret@example.atlassian.net/a.png", false),
            ("https://user@example.atlassian.net/a.png", false),
            ("https://example.atlassian.net:8443/a.png", false),
            ("javascript:alert(1)", false),
            ("/rest/api/3/attachment/content/1", false),
            ("", false),
        ] {
            assert_eq!(is_on_site(url, SITE), expected, "{url}");
        }
    }

    #[test]
    fn nothing_is_on_a_site_that_is_not_https() {
        assert!(!is_on_site(
            "http://127.0.0.1:47470/rest/api/3/attachment/content/1",
            "http://127.0.0.1:47470/"
        ));
        assert!(!is_on_site("https://example.atlassian.net/a", "not a url"));
    }

    #[test]
    fn urls_off_the_site_are_cleared_and_those_on_it_are_kept() {
        let on_site = "https://example.atlassian.net/rest/api/3/attachment/content/1";
        let mut attachments = vec![
            with_url(Some(on_site)),
            with_url(Some("https://evil.example.com/a.pdf")),
            with_url(Some("http://example.atlassian.net/a.pdf")),
            with_url(None),
        ];

        keep_site_urls(&mut attachments, SITE);

        let urls: Vec<Option<&str>> = attachments.iter().map(|a| a.url.as_deref()).collect();
        assert_eq!(urls, [Some(on_site), None, None, None]);
    }
}
