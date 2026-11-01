//! The app menu (Taskboard, File, Edit, Go, List, Issue, View, Window, Help). Every custom item's id is the command
//! it runs; `page:<id>` items run the main window's own commands (the same ids its keys run). The standard
//! items (About, Services, Hide, the Edit items, Minimize) are macOS's own. Plain-letter keys (J, K, P, M) stay
//! off the menu items, so they never take a typed letter from a text field.

mod build;
mod items;
pub mod palette;
pub mod place;

use tauri::{AppHandle, Manager};

use crate::bus;

pub use build::build;
pub use items::*;

/// Runs the command a clicked item names; ids the bus doesn't know are ignored.
pub fn clicked(app: &AppHandle, id: &str) {
    if app.state::<bus::Commands>().knows(id) {
        bus::spawn(app, id);
    }
}
