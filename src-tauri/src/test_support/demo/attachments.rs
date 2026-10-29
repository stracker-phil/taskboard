//! Files attached to a few demo issues, shaped like the `attachment` field of `POST /search/jql`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::SITE_URL;
use super::attachment_files::demo_file;
use super::clock::Clock;
use super::people::{AM, JR, MK, Person, SB, user};
use super::render::issue_id;

const FIRST_ATTACHMENT_ID: u64 = 80_001;

struct AttachmentSpec {
    issue: &'static str,
    filename: &'static str,
    mime_type: &'static str,
    size: u64,
    author: &'static Person,
    at: &'static str,
}

const ATTACHMENTS: &[AttachmentSpec] = &[
    AttachmentSpec {
        issue: "NW-1020",
        filename: "release-notes.pdf",
        mime_type: "application/pdf",
        size: 1_240_000,
        author: MK,
        at: "2026-10-12 09:30",
    },
    AttachmentSpec {
        issue: "NW-1020",
        filename: "checkout-flow.png",
        mime_type: "image/png",
        size: 384_500,
        author: JR,
        at: "2026-10-12 09:31",
    },
    AttachmentSpec {
        issue: "NW-1042",
        filename: "screenshot-login.png",
        mime_type: "image/png",
        size: 212_000,
        author: AM,
        at: "2026-10-20 14:05",
    },
    AttachmentSpec {
        issue: "LUM-309",
        filename: "logs.zip",
        mime_type: "application/zip",
        size: 5_600_000,
        author: SB,
        at: "2026-10-24 11:10",
    },
];

/// Adds the demo attachments to their issues' fields; returns each one's MIME type and made-up bytes by id.
pub fn attach(issues: &mut [Value], clock: &Clock) -> BTreeMap<String, (String, Vec<u8>)> {
    let mut files = BTreeMap::new();
    for (offset, spec) in (0_u64..).zip(ATTACHMENTS) {
        let id = FIRST_ATTACHMENT_ID + offset;
        let issue_id = issue_id(spec.issue);
        let issue = issues
            .iter_mut()
            .find(|issue| issue["id"] == issue_id.as_str())
            .unwrap_or_else(|| panic!("demo attachment on {}", spec.issue));
        let item = json!({
            "self": format!("{SITE_URL}/rest/api/3/attachment/{id}"),
            "id": id.to_string(),
            "filename": spec.filename,
            "author": user(spec.author),
            "created": clock.stamp(spec.at),
            "size": spec.size,
            "mimeType": spec.mime_type,
            "content": format!("{SITE_URL}/rest/api/3/attachment/content/{id}"),
        });
        match issue["fields"]["attachment"].as_array_mut() {
            Some(list) => list.push(item),
            None => issue["fields"]["attachment"] = json!([item]),
        }
        files.insert(
            id.to_string(),
            (spec.mime_type.to_owned(), demo_file(spec.mime_type)),
        );
    }
    files
}
