//! An attachment the owner keeps (ADR 0075): the downloaded copy (`preview`) copied to the Downloads folder under a
//! free name, or to the place the save panel picked. Taskboard neither tracks nor deletes these files.

use std::io;
use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::{AppError, AppResult};

/// Past this many copies of one name, saving fails.
const MAX_NUMBER: u32 = 9999;

/// Copies `from` into `folder` under its own name, or the first free "name 2.ext", "name 3.ext" (as Finder
/// numbers copies). Claiming the name and writing are one step, so two saves never share a file.
pub async fn save_new(folder: &Path, from: &Path) -> AppResult<PathBuf> {
    let name = from
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::Internal("A downloaded attachment without a name".into()))?;
    for number in 1..=MAX_NUMBER {
        let target = folder.join(numbered(name, number));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .await
        {
            Ok(file) => {
                copy_into(file, from, &target).await?;
                return Ok(target);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(io_error(e)),
        }
    }
    Err(AppError::Io(
        "Every numbered name for the attachment is taken".into(),
    ))
}

/// Copies `from` to `target`, replacing a file there (the save panel has asked already).
pub async fn save_to(target: &Path, from: &Path) -> AppResult<()> {
    fs::copy(from, target).await.map(drop).map_err(io_error)
}

async fn copy_into(mut file: fs::File, from: &Path, target: &Path) -> AppResult<()> {
    let copied = match fs::File::open(from).await {
        Ok(mut source) => tokio::io::copy(&mut source, &mut file).await.map(drop),
        Err(e) => Err(e),
    };
    if copied.is_err() {
        drop(file);
        let _ = fs::remove_file(target).await;
    }
    copied.map_err(io_error)
}

/// "report.pdf" as copy `number`: itself first, then "report 2.pdf"; a leading-dot or extensionless name gets the
/// number at its end.
fn numbered(name: &str, number: u32) -> String {
    if number == 1 {
        return name.into();
    }
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{stem} {number}.{ext}"),
        _ => format!("{name} {number}"),
    }
}

fn io_error(e: io::Error) -> AppError {
    AppError::Io(format!("The attachment couldn't be saved: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cached_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[tokio::test]
    async fn save_new_copies_the_file_under_its_own_name_and_keeps_the_source() {
        let cache = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let from = cached_file(cache.path(), "release-notes.pdf", b"made-up pdf bytes");

        let saved = save_new(folder.path(), &from).await.unwrap();

        assert_eq!(saved, folder.path().join("release-notes.pdf"));
        assert_eq!(std::fs::read(&saved).unwrap(), b"made-up pdf bytes");
        assert_eq!(std::fs::read(&from).unwrap(), b"made-up pdf bytes");
    }

    #[tokio::test]
    async fn save_new_numbers_copies_when_the_name_is_taken_and_leaves_existing_files() {
        let cache = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let from = cached_file(cache.path(), "logs.zip", b"new");
        cached_file(folder.path(), "logs.zip", b"first");

        let second = save_new(folder.path(), &from).await.unwrap();
        let third = save_new(folder.path(), &from).await.unwrap();

        assert_eq!(second, folder.path().join("logs 2.zip"));
        assert_eq!(third, folder.path().join("logs 3.zip"));
        assert_eq!(
            std::fs::read(folder.path().join("logs.zip")).unwrap(),
            b"first"
        );
        assert_eq!(std::fs::read(&second).unwrap(), b"new");
        assert_eq!(std::fs::read(&third).unwrap(), b"new");
    }

    #[tokio::test]
    async fn save_new_numbers_names_by_where_the_extension_starts() {
        for (name, expected) in [
            ("notes", "notes 2"),
            (".env", ".env 2"),
            ("archive.tar.gz", "archive.tar 2.gz"),
        ] {
            let cache = tempfile::tempdir().unwrap();
            let folder = tempfile::tempdir().unwrap();
            let from = cached_file(cache.path(), name, b"copy");
            cached_file(folder.path(), name, b"original");

            let saved = save_new(folder.path(), &from).await.unwrap();

            assert_eq!(saved, folder.path().join(expected), "{name}");
        }
    }

    #[tokio::test]
    async fn save_new_with_a_missing_source_fails_and_leaves_no_file_behind() {
        let cache = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let from = cache.path().join("release-notes.pdf");

        let err = save_new(folder.path(), &from).await.unwrap_err();

        assert!(matches!(err, AppError::Io(_)), "{err:?}");
        assert_eq!(file_names(folder.path()), Vec::<String>::new());
    }

    #[tokio::test]
    async fn save_new_into_a_missing_folder_fails_with_an_io_error() {
        let cache = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let from = cached_file(cache.path(), "logs.zip", b"bytes");

        let err = save_new(&root.path().join("no-such-folder"), &from)
            .await
            .unwrap_err();

        assert!(matches!(err, AppError::Io(_)), "{err:?}");
    }

    #[tokio::test]
    async fn save_to_copies_to_the_exact_target_and_replaces_an_existing_file() {
        let cache = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let from = cached_file(cache.path(), "release-notes.pdf", b"fresh");
        let target = folder.path().join("chosen name.pdf");

        save_to(&target, &from).await.unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"fresh");

        std::fs::write(&target, b"stale and longer than the new bytes").unwrap();
        save_to(&target, &from).await.unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"fresh");
        assert_eq!(
            file_names(folder.path()),
            vec!["chosen name.pdf".to_string()]
        );
    }
}
