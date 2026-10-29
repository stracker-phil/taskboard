//! Media nodes. A node naming an image attached to the issue becomes a placeholder,
//! `<img data-attachment-id="…" alt="…" width="…" height="…">`, which the page fills with a `data:` URL that Rust
//! downloads (ADR 0039); it never carries a `src`. Anything else is a text chip, and nothing remote is referenced.
//!
//! A media node's `id` is a Media Services file id, not the attachment's id, so the link is the file name
//! (`alt`, else `name` or `__fileName`), matched against the issue's attachments.

use serde_json::Value;

use super::{escape, web_address};

/// One of the issue's attachments, as a media node may name it.
#[derive(Clone, Debug, PartialEq)]
pub struct File {
    pub id: String,
    pub filename: String,
    pub mime_type: Option<String>,
    /// UTC, RFC 3339, as `Media::written_at`: they compare as text.
    pub created_at: Option<String>,
}

/// What a document's media nodes may show: the issue's attachments, and when the document was written (a
/// comment's time; none for a description or field).
#[derive(Clone, Copy, Debug, Default)]
pub struct Media<'a> {
    pub files: &'a [File],
    pub written_at: Option<&'a str>,
}

impl Media<'_> {
    /// No attachments: every media node is a chip.
    pub const NONE: Media<'static> = Media {
        files: &[],
        written_at: None,
    };

    /// The attachment called `name`. Of several, the newest created at or before the document was written,
    /// else the newest.
    pub fn find(&self, name: &str) -> Option<&File> {
        let mut named: Vec<&File> = self.files.iter().filter(|f| f.filename == name).collect();
        named.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        let before = |f: &File| match (self.written_at, f.created_at.as_deref()) {
            (Some(written), Some(created)) => created <= written,
            _ => false,
        };
        named
            .iter()
            .rev()
            .copied()
            .find(|f| before(f))
            .or(named.last().copied())
    }
}

const IMAGE_TYPES: &[&str] = &["image/png", "image/jpeg", "image/gif", "image/webp"];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

/// PNG, JPEG, GIF or WebP by its MIME type, else by its extension; never SVG.
pub fn is_inline_image(file: &File) -> bool {
    match file.mime_type.as_deref().map(str::to_ascii_lowercase) {
        Some(mime) if mime.starts_with("image/") => IMAGE_TYPES.contains(&mime.as_str()),
        _ => file
            .filename
            .rsplit_once('.')
            .is_some_and(|(_, ext)| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str())),
    }
}

/// A `media` or `mediaInline` node; `single` is its `mediaSingle` parent's attributes, when it has one.
pub(super) fn render(node: &Value, single: Option<&Value>, media: &Media, out: &mut String) {
    let attrs = &node["attrs"];
    let name = ["alt", "name", "__fileName"]
        .iter()
        .find_map(|key| attrs[*key].as_str().filter(|n| !n.trim().is_empty()));
    if attrs["type"] == "external" {
        return external(attrs["url"].as_str(), name, out);
    }
    let image = name
        .and_then(|name| media.find(name))
        .filter(|file| is_inline_image(file) && is_digits(&file.id));
    match (image, name) {
        (Some(file), Some(name)) => placeholder(file, name, size(attrs, single), out),
        _ => chip(name, out),
    }
}

fn placeholder(file: &File, name: &str, size: Option<(u64, u64)>, out: &mut String) {
    out.push_str(&format!("<img data-attachment-id=\"{}\" alt=\"", file.id));
    escape(name, out);
    out.push('"');
    if let Some((width, height)) = size {
        out.push_str(&format!(" width=\"{width}\" height=\"{height}\""));
    }
    out.push('>');
}

/// The node's pixel size, scaled to its `mediaSingle`'s pixel width when it sets one.
fn size(attrs: &Value, single: Option<&Value>) -> Option<(u64, u64)> {
    let width = attrs["width"].as_u64().filter(|&w| w > 0)?;
    let height = attrs["height"].as_u64().filter(|&h| h > 0)?;
    let shown = single
        .filter(|single| single["widthType"] == "pixel")
        .and_then(|single| single["width"].as_f64())
        .map(|w| w.round() as u64)
        .filter(|&w| w > 0);
    Some(match shown {
        Some(shown) => (shown, (height * shown).div_ceil(width)),
        None => (width, height),
    })
}

fn chip(name: Option<&str>, out: &mut String) {
    out.push_str("<span class=\"jira-chip\">Attachment");
    if let Some(name) = name {
        out.push_str(" · ");
        escape(name, out);
    }
    out.push_str("</span> ");
}

/// An image on another server: a chip that links to it, never loaded.
fn external(url: Option<&str>, name: Option<&str>, out: &mut String) {
    let href = web_address(url);
    let tag = match href {
        Some(href) => {
            out.push_str("<a class=\"jira-chip\" href=\"");
            escape(href, out);
            out.push_str("\">");
            "a"
        }
        None => {
            out.push_str("<span class=\"jira-chip\">");
            "span"
        }
    };
    out.push_str("Image");
    if let Some(label) = name.or(href).filter(|l| !l.trim().is_empty()) {
        out.push_str(" · ");
        escape(label, out);
    }
    out.push_str(&format!("</{tag}> "));
}

fn is_digits(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}
