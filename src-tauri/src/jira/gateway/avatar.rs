//! Profile pictures from Atlassian's public avatar host (ADR 0032): the one request that isn't a Jira API call.
//! It carries no credentials, follows no redirect and only ever returns a small raster image.

use super::http::Http;
use crate::net::{AvatarUrl, NetError};

/// Jira's 48 px avatars are a few KB; anything near this cap isn't one.
pub const MAX_AVATAR_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
}

impl ImageKind {
    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
        }
    }

    pub fn from_mime(mime: &str) -> Option<Self> {
        [Self::Png, Self::Jpeg, Self::Gif, Self::Webp]
            .into_iter()
            .find(|kind| kind.mime() == mime)
    }

    /// By a file name's extension (`.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, any case).
    pub fn from_extension(filename: &str) -> Option<Self> {
        let (_, ext) = filename.rsplit_once('.')?;
        match ext.to_ascii_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "gif" => Some(Self::Gif),
            "webp" => Some(Self::Webp),
            _ => None,
        }
    }

    /// The format the bytes start with.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(Self::Jpeg)
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some(Self::Gif)
        } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
            Some(Self::Webp)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub kind: ImageKind,
    pub bytes: Vec<u8>,
}

/// An answer is a picture only when the declared type and the bytes name the same raster format and it fits
/// `MAX_AVATAR_BYTES`.
pub fn check_image(content_type: Option<&str>, bytes: Vec<u8>) -> Result<Image, NetError> {
    if bytes.len() > MAX_AVATAR_BYTES {
        return Err(too_large());
    }
    let declared = content_type
        .and_then(|value| value.split(';').next())
        .map(|mime| mime.trim().to_ascii_lowercase())
        .and_then(|mime| ImageKind::from_mime(&mime))
        .ok_or_else(|| {
            NetError::Unexpected("The avatar host sent something other than a picture".into())
        })?;
    if ImageKind::sniff(&bytes) != Some(declared) {
        return Err(NetError::Unexpected(
            "The avatar host's picture isn't the format it claims".into(),
        ));
    }
    Ok(Image {
        kind: declared,
        bytes,
    })
}

pub(super) fn too_large() -> NetError {
    NetError::Unexpected(format!(
        "The avatar is larger than {} KB",
        MAX_AVATAR_BYTES / 1024
    ))
}

/// Fetches profile pictures; shared by every connection, as the host is public and the same for every site.
pub struct AvatarClient {
    http: Http,
}

impl Default for AvatarClient {
    fn default() -> Self {
        Self {
            http: Http::new(Vec::<String>::new()),
        }
    }
}

impl AvatarClient {
    pub async fn fetch(&self, url: &AvatarUrl) -> Result<Image, NetError> {
        self.http.get_image(url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10, b'J', b'F'];
    const GIF: &[u8] = b"GIF89a\x01\0\x01\0";
    const WEBP: &[u8] = b"RIFF\x24\0\0\0WEBPVP8 ";
    const SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";

    #[test]
    fn a_png_jpeg_gif_or_webp_that_matches_its_type_is_a_picture() {
        for (content_type, bytes, kind) in [
            ("image/png", PNG, ImageKind::Png),
            ("image/jpeg", JPEG, ImageKind::Jpeg),
            ("image/gif", GIF, ImageKind::Gif),
            ("image/webp", WEBP, ImageKind::Webp),
            ("IMAGE/PNG; charset=binary", PNG, ImageKind::Png),
        ] {
            let image = check_image(Some(content_type), bytes.to_vec()).unwrap();
            assert_eq!(
                image,
                Image {
                    kind,
                    bytes: bytes.to_vec()
                },
                "{content_type}"
            );
        }
    }

    #[test]
    fn a_file_name_extension_names_a_raster_format_in_any_case() {
        for (filename, expected) in [
            ("a.png", Some(ImageKind::Png)),
            ("a.PNG", Some(ImageKind::Png)),
            ("a.b.jpg", Some(ImageKind::Jpeg)),
            ("a.JPEG", Some(ImageKind::Jpeg)),
            ("a.gif", Some(ImageKind::Gif)),
            ("a.webp", Some(ImageKind::Webp)),
            ("a.svg", None),
            ("a.pdf", None),
            ("png", None),
            ("a.", None),
        ] {
            assert_eq!(ImageKind::from_extension(filename), expected, "{filename}");
        }
    }

    #[test]
    fn sniffing_names_the_format_the_bytes_start_with() {
        for (bytes, expected) in [
            (PNG, Some(ImageKind::Png)),
            (JPEG, Some(ImageKind::Jpeg)),
            (GIF, Some(ImageKind::Gif)),
            (WEBP, Some(ImageKind::Webp)),
            (SVG, None),
            (b"".as_slice(), None),
        ] {
            assert_eq!(ImageKind::sniff(bytes), expected);
        }
    }

    #[test]
    fn svg_html_and_unknown_types_are_refused() {
        for (content_type, bytes) in [
            (Some("image/svg+xml"), SVG),
            (Some("text/html"), b"<html></html>".as_slice()),
            (Some("application/octet-stream"), PNG),
            (Some(""), PNG),
            (None, PNG),
        ] {
            assert!(
                check_image(content_type, bytes.to_vec()).is_err(),
                "{content_type:?}"
            );
        }
    }

    #[test]
    fn bytes_that_are_not_the_declared_format_are_refused() {
        for (content_type, bytes) in [
            ("image/png", SVG),
            ("image/png", JPEG),
            ("image/jpeg", GIF),
            ("image/webp", b"RIFF\x24\0\0\0WAVEfmt ".as_slice()),
            ("image/gif", b"".as_slice()),
        ] {
            assert!(
                check_image(Some(content_type), bytes.to_vec()).is_err(),
                "{content_type}"
            );
        }
    }

    #[test]
    fn a_picture_up_to_the_cap_passes_and_one_byte_more_fails() {
        let mut bytes = PNG.to_vec();
        bytes.resize(MAX_AVATAR_BYTES, 0);
        assert!(check_image(Some("image/png"), bytes.clone()).is_ok());

        bytes.push(0);
        assert!(check_image(Some("image/png"), bytes).is_err());
    }
}
