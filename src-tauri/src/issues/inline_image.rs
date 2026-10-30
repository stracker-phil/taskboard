//! An image attached to an issue, served to rich text's placeholders through the `inline-image:` scheme
//! (ADR 0042). It comes through the attachment download Quick Look uses (ADR 0035), reusing its file, and only
//! when the bytes are the raster format the attachment declares.

use std::path::Path;

use tauri::http::{Response, StatusCode, header};

use super::preview::{issue_attachment, local_copy};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::dto;
use crate::jira::gateway::ImageKind;
use crate::sync::source::Source;

/// The webview asks `inline-image://localhost/<issue id>/<attachment id>`.
pub const INLINE_IMAGE_SCHEME: &str = "inline-image";
/// Larger images stay a chip: Quick Look still shows them.
pub const MAX_INLINE_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct InlineImage {
    pub kind: ImageKind,
    pub bytes: Vec<u8>,
}

/// The issue and attachment ids of a scheme request's path; both must be tracker ids (digits).
pub fn image_path(path: &str) -> Option<(String, String)> {
    let (issue_id, attachment_id) = path.strip_prefix('/')?.split_once('/')?;
    let is_id = |id: &str| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit());
    (is_id(issue_id) && is_id(attachment_id)).then(|| (issue_id.into(), attachment_id.into()))
}

/// The attachment's bytes; `None` (the page keeps the chip) when it isn't a PNG, JPEG, GIF or WebP whose bytes
/// match its declared type, or is larger than `MAX_INLINE_IMAGE_BYTES`. `NotFound` when the attachment isn't
/// the issue's.
pub async fn attachment_image(
    db: &Db,
    source: &dyn Source,
    root: &Path,
    issue_id: String,
    attachment_id: String,
) -> AppResult<Option<InlineImage>> {
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
    let path = local_copy(source, root, &attachment).await?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;
    Ok(is_inline_image(kind, &bytes).then_some(InlineImage { kind, bytes }))
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

/// The bytes are `declared` and within `MAX_INLINE_IMAGE_BYTES`.
pub fn is_inline_image(declared: ImageKind, bytes: &[u8]) -> bool {
    bytes.len() as u64 <= MAX_INLINE_IMAGE_BYTES && ImageKind::sniff(bytes) == Some(declared)
}

/// The scheme's answer: the image with its type, else 404 (no image, keep the chip) or 500. Never stored
/// by the webview: the bytes stay in the app's cache folder only.
pub fn image_response(answer: AppResult<Option<InlineImage>>) -> Response<Vec<u8>> {
    let (status, image) = match answer {
        Ok(Some(image)) => (StatusCode::OK, Some(image)),
        Ok(None) | Err(AppError::NotFound(_) | AppError::InvalidInput(_)) => {
            (StatusCode::NOT_FOUND, None)
        }
        Err(e) => {
            log::warn!("An inline image couldn't be loaded: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, None)
        }
    };
    let builder = Response::builder()
        .status(status)
        .header(header::CACHE_CONTROL, "no-store");
    let response = match image {
        Some(image) => builder
            .header(header::CONTENT_TYPE, image.kind.mime())
            .body(image.bytes),
        None => builder.body(Vec::new()),
    };
    response.unwrap_or_else(|_| Response::new(Vec::new()))
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
    fn bytes_of_the_declared_format_are_an_inline_image() {
        assert!(is_inline_image(ImageKind::Png, PNG));
    }

    #[test]
    fn bytes_of_another_format_than_declared_are_no_inline_image() {
        assert!(!is_inline_image(ImageKind::Jpeg, PNG));
        assert!(!is_inline_image(ImageKind::Png, b"GIF89a\x01\0\x01\0"));
    }

    #[test]
    fn svg_bytes_are_never_an_inline_image() {
        for kind in [
            ImageKind::Png,
            ImageKind::Jpeg,
            ImageKind::Gif,
            ImageKind::Webp,
        ] {
            assert!(!is_inline_image(kind, SVG), "{kind:?}");
        }
    }

    #[test]
    fn an_image_over_the_cap_is_no_inline_image_and_one_at_the_cap_is() {
        let mut bytes = PNG.to_vec();
        bytes.resize(MAX_INLINE_IMAGE_BYTES as usize, 0);
        assert!(is_inline_image(ImageKind::Png, &bytes));

        bytes.push(0);
        assert!(!is_inline_image(ImageKind::Png, &bytes));
    }

    #[test]
    fn a_scheme_path_is_two_numeric_ids() {
        for (path, expected) in [
            ("/101042/80001", Some(("101042", "80001"))),
            ("/1/2", Some(("1", "2"))),
            ("", None),
            ("/", None),
            ("/101042", None),
            ("/101042/", None),
            ("//80001", None),
            ("/101042/80001/", None),
            ("/101042/80001/3", None),
            ("101042/80001", None),
            ("/abc/80001", None),
            ("/101042/8000x", None),
            ("/../80001", None),
            ("/101042/-1", None),
        ] {
            assert_eq!(
                image_path(path),
                expected.map(|(a, b): (&str, &str)| (a.to_string(), b.to_string())),
                "{path:?}"
            );
        }
    }

    fn png_image() -> InlineImage {
        InlineImage {
            kind: ImageKind::Png,
            bytes: PNG.to_vec(),
        }
    }

    #[test]
    fn an_image_is_answered_with_its_bytes_type_and_no_caching() {
        let response = image_response(Ok(Some(png_image())));

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.body(), PNG);
    }

    #[test]
    fn no_image_or_a_bad_request_is_a_404_with_an_empty_body() {
        for (answer, label) in [
            (Ok(None), "none"),
            (Err(AppError::NotFound("attachment".into())), "not found"),
            (
                Err(AppError::InvalidInput("id".into())),
                "invalid input",
            ),
        ] {
            let response = image_response(answer);

            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{label}");
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store", "{label}");
            assert!(response.body().is_empty(), "{label}");
        }
    }

    #[test]
    fn any_other_failure_is_a_500_with_an_empty_body() {
        for error in [
            AppError::Io("disk full".into()),
            AppError::Db("locked".into()),
        ] {
            let response = image_response(Err(error));

            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert!(response.body().is_empty());
        }
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

}
