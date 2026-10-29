//! When a cached profile picture is good enough and when to ask the avatar host again.

use chrono::{DateTime, Duration, Utc};

/// A picture is asked for again after a week, as people rarely change theirs.
pub const FRESH_FOR: Duration = Duration::days(7);
/// After a failed fetch (offline, refused) the address isn't asked again for a day.
pub const RETRY_AFTER: Duration = Duration::days(1);

/// What the cache holds for one person.
#[derive(Debug, Clone, PartialEq)]
pub struct Cached {
    pub url: String,
    pub picture: Option<Picture>,
    pub fetched_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// Whether to fetch `url` at `now`: never asked, a new address, a picture older than `FRESH_FOR`, or
/// `RETRY_AFTER` past a failure for the same address.
pub fn due(cached: Option<&Cached>, url: &str, now: DateTime<Utc>) -> bool {
    let Some(cached) = cached else {
        return true;
    };
    if cached.url != url {
        return true;
    }
    // A success clears `failed_at`, so a failure means the last attempt for `url` got nothing.
    if let Some(failed_at) = cached.failed_at {
        return now - failed_at >= RETRY_AFTER;
    }
    let fresh =
        cached.picture.is_some() && cached.fetched_at.is_some_and(|at| now - at < FRESH_FOR);
    !fresh
}
