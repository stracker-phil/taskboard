//! An image attached to an issue as a `data:` URL, for the placeholders rich text writes (ADR 0039). It comes
//! through the attachment download Quick Look uses (ADR 0035), reusing its file, and only when the bytes are
//! the raster format the attachment declares; recent answers stay in memory so scrolling doesn't read again.

use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use base64::Engine;

use super::preview::{issue_attachment, local_copy};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::dto;
use crate::jira::gateway::ImageKind;
use crate::lock::MutexExt;
use crate::sync::source::Source;

/// Larger images stay a chip: Quick Look still shows them.
pub const MAX_INLINE_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const MEMORY_ENTRIES: usize = 20;
const MEMORY_BYTES: usize = 50 * 1024 * 1024;

/// The attachment as a `data:` URL; `None` (the page keeps the chip) when it isn't a PNG, JPEG, GIF or WebP
/// whose bytes match its declared type, or is larger than `MAX_INLINE_IMAGE_BYTES`. `NotFound` when the
/// attachment isn't the issue's.
pub async fn attachment_image(
    db: &Db,
    source: &dyn Source,
    root: &Path,
    memory: &ImageMemory,
    issue_id: String,
    attachment_id: String,
) -> AppResult<Option<String>> {
    let attachment = issue_attachment(db, issue_id, attachment_id).await?;
    let Some(kind) = declared_kind(&attachment) else {
        return Ok(None);
    };
    if attachment
        .size
        .is_some_and(|size| size > MAX_INLINE_IMAGE_BYTES)
    {
        return Ok(None);
    }
    if let Some(url) = memory.get(&attachment.id) {
        return Ok(Some(url));
    }
    let path = local_copy(source, root, &attachment).await?;
    let bytes = fs::read(&path).map_err(|e| AppError::Io(e.to_string()))?;
    let url = data_url(kind, &bytes);
    if let Some(url) = &url {
        memory.put(&attachment.id, url.clone());
    }
    Ok(url)
}

/// The raster format the attachment claims: its MIME type, else (without an image MIME type) its extension.
/// SVG and other types are none.
pub fn declared_kind(attachment: &dto::Attachment) -> Option<ImageKind> {
    let mime = attachment
        .mime_type
        .as_deref()
        .and_then(|value| value.split(';').next())
        .map(|mime| mime.trim().to_ascii_lowercase());
    match mime {
        Some(mime) if mime.starts_with("image/") => ImageKind::from_mime(&mime),
        _ => ImageKind::from_extension(&attachment.filename),
    }
}

/// `data:<type>;base64,…` when the bytes are `declared` and within `MAX_INLINE_IMAGE_BYTES`.
pub fn data_url(declared: ImageKind, bytes: &[u8]) -> Option<String> {
    let fits = bytes.len() as u64 <= MAX_INLINE_IMAGE_BYTES;
    (fits && ImageKind::sniff(bytes) == Some(declared)).then(|| {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        format!("data:{};base64,{encoded}", declared.mime())
    })
}

/// The most recently shown images' `data:` URLs by attachment id: at most 20, and 50 MB together.
#[derive(Default)]
pub struct ImageMemory {
    recent: Mutex<VecDeque<(String, String)>>,
}

impl ImageMemory {
    pub fn get(&self, attachment_id: &str) -> Option<String> {
        let mut recent = self.recent.lock_unpoisoned();
        let at = recent.iter().position(|(id, _)| id == attachment_id)?;
        let entry = recent.remove(at)?;
        let url = entry.1.clone();
        recent.push_back(entry);
        Some(url)
    }

    pub fn put(&self, attachment_id: &str, url: String) {
        let mut recent = self.recent.lock_unpoisoned();
        recent.retain(|(id, _)| id != attachment_id);
        recent.push_back((attachment_id.to_owned(), url));
        while recent.len() > MEMORY_ENTRIES
            || recent.iter().map(|(_, url)| url.len()).sum::<usize>() > MEMORY_BYTES
        {
            recent.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
    const SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";

    fn attachment(filename: &str, mime_type: Option<&str>) -> dto::Attachment {
        dto::Attachment {
            id: "80001".into(),
            filename: filename.into(),
            size: Some(1024),
            mime_type: mime_type.map(Into::into),
            created: None,
            author: None,
            content: None,
        }
    }

    #[test]
    fn a_matching_image_becomes_a_base64_data_url() {
        assert_eq!(
            data_url(ImageKind::Png, PNG),
            Some("data:image/png;base64,iVBORw0KGgo=".to_owned())
        );
    }

    #[test]
    fn bytes_of_another_format_than_declared_give_no_data_url() {
        assert_eq!(data_url(ImageKind::Jpeg, PNG), None);
        assert_eq!(data_url(ImageKind::Png, b"GIF89a\x01\0\x01\0"), None);
    }

    #[test]
    fn svg_bytes_never_become_a_data_url() {
        for kind in [
            ImageKind::Png,
            ImageKind::Jpeg,
            ImageKind::Gif,
            ImageKind::Webp,
        ] {
            assert_eq!(data_url(kind, SVG), None, "{kind:?}");
        }
    }

    #[test]
    fn an_image_over_the_cap_gives_no_data_url_and_one_at_the_cap_does() {
        let mut bytes = PNG.to_vec();
        bytes.resize(MAX_INLINE_IMAGE_BYTES as usize, 0);
        assert!(data_url(ImageKind::Png, &bytes).is_some());

        bytes.push(0);
        assert_eq!(data_url(ImageKind::Png, &bytes), None);
    }

    #[test]
    fn the_declared_kind_comes_from_the_mime_type() {
        for (mime, expected) in [
            ("image/png", Some(ImageKind::Png)),
            ("image/jpeg", Some(ImageKind::Jpeg)),
            ("image/gif", Some(ImageKind::Gif)),
            ("image/webp", Some(ImageKind::Webp)),
            ("IMAGE/PNG", Some(ImageKind::Png)),
            ("image/png; charset=binary", Some(ImageKind::Png)),
            ("image/svg+xml", None),
            ("image/bmp", None),
            ("application/pdf", None),
        ] {
            assert_eq!(
                declared_kind(&attachment("file", Some(mime))),
                expected,
                "{mime}"
            );
        }
    }

    #[test]
    fn the_declared_kind_falls_back_to_the_extension_without_an_image_mime_type() {
        for (filename, mime, expected) in [
            ("shot.png", None, Some(ImageKind::Png)),
            (
                "shot.JPG",
                Some("application/octet-stream"),
                Some(ImageKind::Jpeg),
            ),
            ("shot.gif", Some(""), Some(ImageKind::Gif)),
            ("shot.svg", None, None),
            ("notes.pdf", Some("application/octet-stream"), None),
            ("fake.png", Some("image/svg+xml"), None),
        ] {
            assert_eq!(
                declared_kind(&attachment(filename, mime)),
                expected,
                "{filename} {mime:?}"
            );
        }
    }

    #[test]
    fn memory_returns_what_was_put_and_nothing_for_an_unknown_id() {
        let memory = ImageMemory::default();
        memory.put("1", "data:one".into());
        assert_eq!(memory.get("1"), Some("data:one".to_owned()));
        assert_eq!(memory.get("2"), None);
    }

    #[test]
    fn putting_an_id_again_replaces_its_url() {
        let memory = ImageMemory::default();
        memory.put("1", "data:old".into());
        memory.put("1", "data:new".into());
        assert_eq!(memory.get("1"), Some("data:new".to_owned()));
    }

    #[test]
    fn memory_forgets_the_oldest_entry_after_twenty() {
        let memory = ImageMemory::default();
        for id in 1..=21 {
            memory.put(&id.to_string(), format!("data:{id}"));
        }
        assert_eq!(memory.get("1"), None);
        assert_eq!(memory.get("2"), Some("data:2".to_owned()));
        assert_eq!(memory.get("21"), Some("data:21".to_owned()));
    }

    #[test]
    fn a_recently_read_entry_survives_eviction() {
        let memory = ImageMemory::default();
        for id in 1..=20 {
            memory.put(&id.to_string(), format!("data:{id}"));
        }
        assert!(memory.get("1").is_some());
        memory.put("21", "data:21".into());
        assert_eq!(memory.get("1"), Some("data:1".to_owned()));
        assert_eq!(memory.get("2"), None);
    }

    #[test]
    fn memory_forgets_the_oldest_entries_beyond_fifty_megabytes() {
        let memory = ImageMemory::default();
        let big = "x".repeat(20 * 1024 * 1024);
        memory.put("1", big.clone());
        memory.put("2", big.clone());
        assert!(memory.get("1").is_some());
        memory.put("3", big);
        assert_eq!(memory.get("2"), None);
        assert!(memory.get("1").is_some());
        assert!(memory.get("3").is_some());
    }
}
