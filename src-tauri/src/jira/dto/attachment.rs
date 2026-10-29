//! A file attached to an issue, as the `attachment` field of a search lists it.

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    #[serde(default)]
    pub filename: String,
    /// Bytes.
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub author: Option<AttachmentAuthor>,
    /// The download address on the site; untrusted until checked against it.
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentAuthor {
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_full_jira_attachment_deserializes() {
        let attachment: Attachment = serde_json::from_value(json!({
            "self": "https://example.atlassian.net/rest/api/3/attachment/80001",
            "id": "80001",
            "filename": "release-notes.pdf",
            "author": { "accountId": "712020:abc", "displayName": "Marta Klein", "active": true },
            "created": "2026-10-12T09:30:00.000+0200",
            "size": 1_240_000,
            "mimeType": "application/pdf",
            "content": "https://example.atlassian.net/rest/api/3/attachment/content/80001",
        }))
        .unwrap();

        assert_eq!(
            attachment,
            Attachment {
                id: "80001".into(),
                filename: "release-notes.pdf".into(),
                size: Some(1_240_000),
                mime_type: Some("application/pdf".into()),
                created: Some("2026-10-12T09:30:00.000+0200".into()),
                author: Some(AttachmentAuthor {
                    account_id: Some("712020:abc".into()),
                    display_name: Some("Marta Klein".into()),
                }),
                content: Some(
                    "https://example.atlassian.net/rest/api/3/attachment/content/80001".into()
                ),
            }
        );
    }

    #[test]
    fn a_missing_author_and_size_are_none() {
        let attachment: Attachment =
            serde_json::from_value(json!({ "id": "80002", "filename": "notes.txt" })).unwrap();

        assert_eq!(attachment.author, None);
        assert_eq!(attachment.size, None);
    }
}
