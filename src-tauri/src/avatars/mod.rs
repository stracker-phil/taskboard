//! People's profile pictures (ADR 0032): Rust fetches them from Atlassian's avatar host, caches them in
//! `cache.avatar` and hands the page a `data:` URL, so the webview never loads a remote address.

pub mod commands;
mod freshness;
mod store;

use std::future::Future;

use base64::Engine;
use chrono::{DateTime, Utc};

pub use freshness::{FRESH_FOR, RETRY_AFTER};

use crate::db::Db;
use crate::error::AppResult;
use crate::jira::gateway::Image;
use crate::net::{AvatarUrl, NetError};
use freshness::{Picture, due};

/// The person's picture as a `data:` URL, or `None` (the page shows initials) when the sync saw no address on
/// the avatar host or no picture ever arrived. Fetches with `fetch` when `freshness::due`; a failed fetch is
/// remembered and the last picture stays.
pub async fn user_avatar<F, Fut>(
    db: &Db,
    account_id: String,
    now: DateTime<Utc>,
    fetch: F,
) -> AppResult<Option<String>>
where
    F: FnOnce(AvatarUrl) -> Fut,
    Fut: Future<Output = Result<Image, NetError>>,
{
    let id = account_id.clone();
    let (wanted, cached) = db
        .read(move |conn| Ok((store::wanted_url(conn, &id)?, store::cached(conn, &id)?)))
        .await?;
    let Some(url) = wanted.as_deref().and_then(AvatarUrl::parse) else {
        return Ok(None);
    };
    let last = cached.as_ref().and_then(|c| c.picture.clone());
    if !due(cached.as_ref(), url.as_str(), now) {
        return Ok(last.as_ref().map(data_url));
    }
    let address = url.as_str().to_owned();
    match fetch(url).await {
        Ok(image) => {
            let picture = Picture {
                mime: image.kind.mime().to_owned(),
                bytes: image.bytes,
            };
            let shown = data_url(&picture);
            db.write(move |tx, _| store::save_picture(tx, &account_id, &address, &picture, now))
                .await?;
            Ok(Some(shown))
        }
        Err(e) => {
            log::info!("A profile picture wasn't fetched: {e}");
            db.write(move |tx, _| store::save_failure(tx, &account_id, &address, now))
                .await?;
            Ok(last.as_ref().map(data_url))
        }
    }
}

fn data_url(picture: &Picture) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(&picture.bytes);
    format!("data:{};base64,{encoded}", picture.mime)
}
