//! An attachment as a local file for Quick Look (ADR 0035): downloaded from the tracker once into
//! `<app cache>/attachments/<id>/<name>`, which is cleared when the app starts and quits.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, Weak};

use rusqlite::OptionalExtension;

use super::attachments::find_attachment;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::dto;
use crate::lock::MutexExt;
use crate::sync::source::Source;

/// Below the file system's 255 bytes per name.
const MAX_NAME_BYTES: usize = 200;

/// The attachment folder inside the app's cache folder.
pub fn cache_root(app_cache_dir: &Path) -> PathBuf {
    app_cache_dir.join("attachments")
}

/// Removes every downloaded attachment.
pub fn clear(root: &Path) {
    match fs::remove_dir_all(root) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("The attachment cache wasn't cleared: {e}"),
    }
}

/// The issue's attachment as a local file: the one already downloaded when it has the size the tracker
/// lists, else a fresh download.
pub async fn attachment_file(
    db: &Db,
    source: &dyn Source,
    root: &Path,
    issue_id: String,
    attachment_id: String,
) -> AppResult<PathBuf> {
    let attachment = issue_attachment(db, issue_id, attachment_id).await?;
    local_copy(source, root, &attachment).await
}

/// The attachment as the issue lists it; `NotFound` when it isn't one of the issue's.
pub(super) async fn issue_attachment(
    db: &Db,
    issue_id: String,
    attachment_id: String,
) -> AppResult<dto::Attachment> {
    let raw: Option<String> = db
        .read(move |conn| {
            Ok(conn
                .query_row(
                    "SELECT raw FROM cache.issue_body WHERE issue_id = ?1",
                    [&issue_id],
                    |r| r.get(0),
                )
                .optional()?)
        })
        .await?;
    raw.as_deref()
        .and_then(|raw| find_attachment(raw, &attachment_id))
        .ok_or_else(|| AppError::NotFound("The attachment isn't on this issue".into()))
}

/// The downloaded copy, reused when it has the listed size, else downloaded now. Asks for one attachment at
/// the same time download it once: the later ones wait and reuse the file.
pub(super) async fn local_copy(
    source: &dyn Source,
    root: &Path,
    attachment: &dto::Attachment,
) -> AppResult<PathBuf> {
    let path = cached_path(root, &attachment.id, &attachment.filename)?;
    let lock = download_lock(&path);
    let _downloading = lock.lock().await;
    if is_reusable(&path, attachment.size) {
        return Ok(path);
    }
    let bytes = source.attachment_content(&attachment.id).await?;
    save(&path, &bytes).await?;
    Ok(path)
}

fn download_lock(path: &Path) -> Arc<tokio::sync::Mutex<()>> {
    static DOWNLOADS: LazyLock<Mutex<HashMap<PathBuf, Weak<tokio::sync::Mutex<()>>>>> =
        LazyLock::new(Mutex::default);
    let mut downloads = DOWNLOADS.lock_unpoisoned();
    downloads.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = downloads.get(path).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(tokio::sync::Mutex::new(()));
    downloads.insert(path.to_owned(), Arc::downgrade(&lock));
    lock
}

/// `<root>/<id>/<safe name>`; the id must be a tracker id (digits), so it can't leave the folder.
pub fn cached_path(root: &Path, attachment_id: &str, filename: &str) -> AppResult<PathBuf> {
    if attachment_id.is_empty() || !attachment_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::InvalidInput(format!(
            "Not an attachment id: {attachment_id}"
        )));
    }
    Ok(root.join(attachment_id).join(safe_filename(filename)))
}

/// One path component that keeps the extension: no separators, control characters, `..` or leading dot.
pub fn safe_filename(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let mut safe = replaced;
    while safe.contains("..") {
        safe = safe.replace("..", ".");
    }
    let safe = safe.trim().trim_start_matches('.').trim();
    if safe.is_empty() {
        return "attachment".into();
    }
    shortened(safe)
}

/// At most `MAX_NAME_BYTES`, cut from the end of the name before its extension.
fn shortened(name: &str) -> String {
    if name.len() <= MAX_NAME_BYTES {
        return name.into();
    }
    let ext = name.rsplit_once('.').map_or("", |(_, ext)| ext);
    let ext = if ext.len() <= 16 { ext } else { "" };
    let keep = MAX_NAME_BYTES - ext.len() - 1;
    let stem = &name[..name.floor_char_boundary(keep)];
    if ext.is_empty() {
        stem.into()
    } else {
        format!("{stem}.{ext}")
    }
}

/// The file is there with the size the tracker lists (or any size when it lists none).
pub fn is_reusable(path: &Path, size: Option<u64>) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file() && size.is_none_or(|size| m.len() == size))
}

/// Written next to its place first under a name of its own, so a half-written file is never shown.
async fn save(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let folder = path
        .parent()
        .ok_or_else(|| AppError::Internal("An attachment path without a folder".into()))?;
    tokio::fs::create_dir_all(folder).await.map_err(io_error)?;
    let partial = folder.join(format!(".download-{}", uuid::Uuid::new_v4()));
    let written = match tokio::fs::write(&partial, bytes).await {
        Ok(()) => tokio::fs::rename(&partial, path).await,
        Err(e) => Err(e),
    };
    if written.is_err() {
        let _ = tokio::fs::remove_file(&partial).await;
    }
    written.map_err(io_error)
}

fn io_error(e: io::Error) -> AppError {
    AppError::Io(format!("The attachment couldn't be saved: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separators_colons_and_control_characters_become_underscores() {
        for (name, expected) in [
            ("report.pdf", "report.pdf"),
            ("Q3 report (final).pdf", "Q3 report (final).pdf"),
            ("a/b\\c:d.txt", "a_b_c_d.txt"),
            ("x\u{0}y.txt", "x_y.txt"),
            ("a\tb\nc.txt", "a_b_c.txt"),
            ("../../etc/passwd", "_._etc_passwd"),
            ("C:\\Users\\demo\\notes.txt", "C__Users_demo_notes.txt"),
        ] {
            assert_eq!(safe_filename(name), expected, "{name:?}");
        }
    }

    #[test]
    fn repeated_dots_collapse_and_leading_dots_and_whitespace_go() {
        for (name, expected) in [
            ("a...b.txt", "a.b.txt"),
            ("a..b", "a.b"),
            ("...hidden", "hidden"),
            (".env", "env"),
            ("  notes.txt  ", "notes.txt"),
            ("  .env  ", "env"),
            (" ..x", "x"),
            ("trailing.", "trailing."),
        ] {
            assert_eq!(safe_filename(name), expected, "{name:?}");
        }
    }

    #[test]
    fn a_name_without_anything_left_becomes_attachment() {
        for name in ["", "   ", ".", "...", " .. ", ". "] {
            assert_eq!(safe_filename(name), "attachment", "{name:?}");
        }
    }

    #[test]
    fn a_name_is_never_a_path_that_leaves_its_folder() {
        for name in ["../../x", "/etc/passwd", "..\\..\\x", "a/../../b", "~/../x"] {
            let safe = safe_filename(name);

            assert!(!safe.contains(['/', '\\']), "{name:?} -> {safe:?}");
            assert!(!safe.contains(".."), "{name:?} -> {safe:?}");
            assert!(!safe.starts_with('.'), "{name:?} -> {safe:?}");
        }
    }

    #[test]
    fn a_long_name_is_cut_from_the_stem_and_keeps_its_extension() {
        let long = format!("{}.pdf", "a".repeat(300));

        let safe = safe_filename(&long);

        assert_eq!(safe, format!("{}.pdf", "a".repeat(196)));
        assert_eq!(safe.len(), MAX_NAME_BYTES);
    }

    #[test]
    fn a_name_of_exactly_the_limit_is_kept_whole() {
        let name = format!("{}.txt", "b".repeat(196));

        assert_eq!(name.len(), MAX_NAME_BYTES);
        assert_eq!(safe_filename(&name), name);
    }

    #[test]
    fn a_long_name_is_not_cut_in_the_middle_of_a_multi_byte_character() {
        let long = format!("a{}.pdf", "é".repeat(150));

        let safe = safe_filename(&long);

        assert_eq!(safe, format!("a{}.pdf", "é".repeat(97)));
        assert!(safe.len() <= MAX_NAME_BYTES);
    }

    #[test]
    fn a_long_name_with_an_oversized_extension_is_cut_without_one() {
        let long = format!("{}.{}", "a".repeat(300), "b".repeat(20));

        let safe = safe_filename(&long);

        assert_eq!(safe, "a".repeat(MAX_NAME_BYTES - 1));
    }

    #[test]
    fn a_long_name_without_an_extension_is_cut_to_fit() {
        let safe = safe_filename(&"c".repeat(400));

        assert_eq!(safe, "c".repeat(MAX_NAME_BYTES - 1));
    }

    #[test]
    fn the_cache_folder_sits_in_the_app_cache_under_attachments() {
        assert_eq!(
            cache_root(Path::new("/cache/app")),
            Path::new("/cache/app/attachments")
        );
    }

    #[test]
    fn a_cached_path_is_the_id_folder_with_the_safe_name() {
        let root = Path::new("/cache/attachments");

        assert_eq!(
            cached_path(root, "80001", "release-notes.pdf").unwrap(),
            root.join("80001").join("release-notes.pdf")
        );
        assert_eq!(
            cached_path(root, "80001", "../x.pdf").unwrap(),
            root.join("80001").join("_x.pdf")
        );
        assert_eq!(
            cached_path(root, "7", "").unwrap(),
            root.join("7").join("attachment")
        );
    }

    #[test]
    fn an_id_that_is_not_all_digits_is_refused() {
        let root = Path::new("/cache/attachments");

        for id in ["../1", "", "12a", "1/2", "-1", " 1", "1\0", "NW-1", ".."] {
            assert!(
                matches!(
                    cached_path(root, id, "a.pdf"),
                    Err(AppError::InvalidInput(_))
                ),
                "{id:?}"
            );
        }
    }

    #[test]
    fn a_file_of_the_listed_size_is_reusable() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.bin");
        fs::write(&file, b"12345").unwrap();

        assert!(is_reusable(&file, Some(5)));
        assert!(!is_reusable(&file, Some(4)));
        assert!(!is_reusable(&file, Some(6)));
        assert!(!is_reusable(&file, Some(0)));
    }

    #[test]
    fn a_file_of_any_size_is_reusable_when_the_tracker_lists_none() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.bin");
        fs::write(&file, b"12345").unwrap();
        let empty = dir.path().join("empty.bin");
        fs::write(&empty, b"").unwrap();

        assert!(is_reusable(&file, None));
        assert!(is_reusable(&empty, None));
        assert!(is_reusable(&empty, Some(0)));
    }

    #[test]
    fn a_missing_file_or_a_folder_is_not_reusable() {
        let dir = tempfile::tempdir().unwrap();

        assert!(!is_reusable(&dir.path().join("missing.bin"), None));
        assert!(!is_reusable(&dir.path().join("missing.bin"), Some(1)));
        assert!(!is_reusable(dir.path(), None));
        assert!(!is_reusable(dir.path(), Some(0)));
    }

    #[test]
    fn clearing_removes_every_downloaded_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = cache_root(dir.path());
        fs::create_dir_all(root.join("80001")).unwrap();
        fs::write(root.join("80001").join("a.pdf"), b"%PDF-").unwrap();

        clear(&root);

        assert!(!root.exists());
        assert!(dir.path().exists());
    }

    #[test]
    fn clearing_a_folder_that_does_not_exist_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        let root = cache_root(dir.path());

        clear(&root);
        clear(&root);

        assert!(!root.exists());
    }
}
