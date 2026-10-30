//! Creating and ending pins. A target has at most one active pin; pinning it again replaces its duration and
//! reason.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::duration::{PinDuration, expires_at};
use crate::db::{Changes, Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::lists::Viewer;
use crate::sync::map::format_utc;

/// A pin (alias `p`) that hasn't been unpinned and hasn't run out at the time bound to `now_param`.
pub fn active(now_param: &str) -> String {
    format!("p.unpinned_at IS NULL AND (p.expires_at IS NULL OR p.expires_at > {now_param})")
}

/// The longest reason kept; the rest is cut.
const REASON_MAX: usize = 200;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum PinKind {
    Issue,
    Comment,
    FilterList,
    View,
}

impl PinKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Issue => "issue",
            Self::Comment => "comment",
            Self::FilterList => "filter_list",
            Self::View => "view",
        }
    }
}

fn normalize_reason(text: Option<String>) -> Option<String> {
    let text = text?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then(|| text.chars().take(REASON_MAX).collect())
}

fn active_pin(conn: &Connection, kind: PinKind, target: &str, now: &str) -> AppResult<Option<i64>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT p.id FROM pin p WHERE p.target_kind = ?2 AND p.target_id = ?3 AND {}",
                active("?1")
            ),
            params![now, kind.as_str(), target],
            |r| r.get(0),
        )
        .optional()?)
}

fn target_exists(conn: &Connection, kind: PinKind, target: &str) -> AppResult<bool> {
    let sql = match kind {
        PinKind::Issue => "SELECT EXISTS (SELECT 1 FROM cache.issue WHERE id = ?1)",
        PinKind::Comment => "SELECT EXISTS (SELECT 1 FROM cache.comment WHERE id = ?1)",
        PinKind::FilterList => {
            "SELECT EXISTS (SELECT 1 FROM filter_list WHERE CAST(id AS TEXT) = ?1)"
        }
        PinKind::View => "SELECT EXISTS (SELECT 1 FROM view WHERE CAST(id AS TEXT) = ?1)",
    };
    Ok(conn.query_row(sql, [target], |r| r.get(0))?)
}

fn pin(
    conn: &Connection,
    changes: &mut Changes,
    viewer: &Viewer,
    kind: PinKind,
    target: &str,
    duration: PinDuration,
    reason: Option<String>,
) -> AppResult<()> {
    if !target_exists(conn, kind, target)? {
        return Err(AppError::NotFound(format!(
            "Nothing to bookmark: {} {target}",
            kind.as_str()
        )));
    }
    let now = format_utc(viewer.now);
    let expires = expires_at(duration, viewer.now, viewer.tz()).map(format_utc);
    match active_pin(conn, kind, target, &now)? {
        Some(id) => conn.execute(
            "UPDATE pin SET reason = ?2, expires_at = ?3 WHERE id = ?1",
            params![id, normalize_reason(reason), expires],
        )?,
        None => conn.execute(
            "INSERT INTO pin (target_kind, target_id, reason, expires_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![kind.as_str(), target, normalize_reason(reason), expires, now],
        )?,
    };
    changes.local_changed(LocalPart::Pins);
    Ok(())
}

fn unpin(conn: &Connection, changes: &mut Changes, id: i64, now: &str) -> AppResult<()> {
    let ended = conn.execute(
        &format!(
            "UPDATE pin AS p SET unpinned_at = ?1 WHERE p.id = ?2 AND {}",
            active("?1")
        ),
        params![now, id],
    )?;
    if ended > 0 {
        changes.local_changed(LocalPart::Pins);
    }
    Ok(())
}

pub async fn pin_create(
    db: &Db,
    viewer: Viewer,
    kind: PinKind,
    target: String,
    duration: PinDuration,
    reason: Option<String>,
) -> AppResult<()> {
    db.write(move |tx, changes| pin(tx, changes, &viewer, kind, &target, duration, reason))
        .await
}

/// Ends the pin now; it moves to Recently unpinned.
pub async fn pin_remove(db: &Db, viewer: Viewer, id: i64) -> AppResult<()> {
    db.write(move |tx, changes| unpin(tx, changes, id, &format_utc(viewer.now)))
        .await
}

/// Unpins the issue if it is pinned, otherwise pins it for `duration` (the owner's default).
pub async fn pin_toggle_issue(
    db: &Db,
    viewer: Viewer,
    id: String,
    duration: PinDuration,
) -> AppResult<()> {
    db.write(move |tx, changes| {
        let now = format_utc(viewer.now);
        match active_pin(tx, PinKind::Issue, &id, &now)? {
            Some(pin_id) => unpin(tx, changes, pin_id, &now),
            None => pin(tx, changes, &viewer, PinKind::Issue, &id, duration, None),
        }
    })
    .await
}
