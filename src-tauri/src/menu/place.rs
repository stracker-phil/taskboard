//! What the menus show of the current place: the shown view's layout, checked in List, and the level's views
//! in Go.

use tauri::{AppHandle, Manager};

use crate::lists::{Kind, node};
use crate::state::AppState;
use crate::views::{Layout, list_views};

/// The List menu's layout items and the layout each one shows.
pub const LAYOUTS: [(&str, Layout); 3] = [
    ("page:view.layout.list", Layout::List),
    ("page:view.layout.board", Layout::Board),
    ("page:view.layout.table", Layout::Table),
];

/// ⌘1–⌘9 open the first nine views; the page's `view.select.<n>` commands run them.
pub const VIEW_KEYS: usize = 9;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Place {
    /// The shown view's layout; none on a level without view tabs.
    pub layout: Option<Layout>,
    /// The level's views (id, name) in tab order.
    pub views: Vec<(i64, String)>,
    pub view_id: Option<i64>,
}

impl Place {
    /// A layout item is checked when it is the shown view's layout; other items have no check.
    pub fn checked(&self, command: &str) -> Option<bool> {
        LAYOUTS
            .iter()
            .find(|(id, _)| *id == command)
            .map(|(_, layout)| self.layout == Some(*layout))
    }
}

/// The current place; empty when there is none or it can't be read.
pub async fn read(app: &AppHandle) -> Place {
    let state = app.state::<AppState>();
    let Ok(nav) = state.nav.current(&state.db).await else {
        return Place::default();
    };
    let Some(at) = nav.location else {
        return Place::default();
    };
    let read = state
        .db
        .read(move |conn| {
            // Bookmarks is one fixed page without view tabs.
            if node(conn, at.list_id)?.kind == Kind::Pinned {
                return Ok(Place::default());
            }
            let views = list_views(conn, at.list_id)?;
            Ok(Place {
                layout: views.iter().find(|v| v.id == at.view_id).map(|v| v.layout),
                views: views.into_iter().map(|v| (v.id, v.name)).collect(),
                view_id: Some(at.view_id),
            })
        })
        .await;
    read.unwrap_or_else(|e: crate::error::AppError| {
        log::warn!("Couldn't read the place for the menus: {e}");
        Place::default()
    })
}

/// Builds the menus again for the current place.
pub fn refresh(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let place = read(&app).await;
        let handle = app.clone();
        let rebuilt = app.run_on_main_thread(move || {
            if let Err(e) =
                super::build(&handle, &place).and_then(|menu| handle.set_menu(menu).map(|_| ()))
            {
                log::warn!("Couldn't rebuild the menus: {e}");
            }
        });
        if let Err(e) = rebuilt {
            log::warn!("Couldn't rebuild the menus: {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn showing(layout: Option<Layout>) -> Place {
        Place {
            layout,
            ..Place::default()
        }
    }

    #[test]
    fn only_the_shown_layouts_item_is_checked() {
        for (shown, expected) in [
            (Layout::List, [Some(true), Some(false), Some(false)]),
            (Layout::Board, [Some(false), Some(true), Some(false)]),
            (Layout::Table, [Some(false), Some(false), Some(true)]),
        ] {
            let place = showing(Some(shown));
            let found = LAYOUTS.map(|(command, _)| place.checked(command));
            assert_eq!(found, expected, "{shown:?}");
        }
    }

    #[test]
    fn without_a_shown_layout_no_layout_item_is_checked() {
        let place = showing(None);

        for (command, _) in LAYOUTS {
            assert_eq!(place.checked(command), Some(false), "{command}");
        }
    }

    #[test]
    fn items_that_are_not_layouts_have_no_check() {
        let place = showing(Some(Layout::Board));

        for command in ["page:view.save", "nav:nextList", "page:view.select.1", ""] {
            assert_eq!(place.checked(command), None, "{command}");
        }
    }

    #[test]
    fn the_layout_items_are_the_three_page_layout_commands() {
        let found: Vec<&str> = LAYOUTS.iter().map(|(command, _)| *command).collect();
        assert_eq!(
            found,
            [
                "page:view.layout.list",
                "page:view.layout.board",
                "page:view.layout.table"
            ]
        );
        assert_eq!(VIEW_KEYS, 9);
    }
}
