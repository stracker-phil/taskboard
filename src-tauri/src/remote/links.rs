//! Follows a `taskboard://` link with what the command line does for the same request.

use tauri::{AppHandle, Manager};

use super::places;
use crate::bus;
use crate::error::AppResult;
use crate::launch::{self, LaunchEvent};
use crate::links::{Link, parse};

/// Opens what the link asks for; a link it can't follow shows why.
pub async fn open(app: &AppHandle, link: &str) {
    if let Err(message) = follow(app, link).await {
        log::warn!("Couldn't follow a link: {message}");
        launch::queue_event(app, LaunchEvent::Error { message });
    }
}

async fn follow(app: &AppHandle, link: &str) -> Result<(), String> {
    let run = |result: AppResult<()>| result.map_err(|e| e.to_string());
    match parse(link)? {
        Link::Issue { key } => run(places::open_issue(app, key).await),
        Link::Go { level, view } => run(places::go(app, level, view).await.map(|_| ())),
        Link::Search { text } => {
            launch::queue_event(app, LaunchEvent::Search { text });
            Ok(())
        }
        Link::Command { id } => {
            crate::window::show_main(app);
            let work = app.state::<bus::Commands>().start(app.clone(), &id);
            run(async { work?.await }.await)
        }
    }
}
