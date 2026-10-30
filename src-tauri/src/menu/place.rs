//! What the menus show of the current place: the shown view's layout, checked in List, and the level's views
//! in Go.

use std::sync::Mutex;

use tauri::menu::{Menu, MenuItemKind};
use tauri::{AppHandle, Manager, Wry};

use crate::bus::CommandStatus;
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

    /// Whether `id` is the Go menu's item of the shown view (`page:view.select.<n>`).
    fn shows_view_item(&self, id: &str) -> Option<bool> {
        let n: usize = id.strip_prefix("page:view.select.")?.parse().ok()?;
        let (view_id, _) = self.views.get(n.checked_sub(1)?)?;
        Some(self.view_id == Some(*view_id))
    }
}

/// The views the menu bar was last built with: while they stay, a move only updates checks and enabled states,
/// as rebuilding the bar makes macOS redraw every menu title.
static BUILT_VIEWS: Mutex<Option<Vec<(i64, String)>>> = Mutex::new(None);

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

fn show(app: &AppHandle, place: &Place) -> tauri::Result<()> {
    let mut built = BUILT_VIEWS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(menu) = app.menu()
        && built.as_ref() == Some(&place.views)
    {
        return update(app, &menu, place);
    }
    app.set_menu(super::build(app, place)?)?;
    *built = Some(place.views.clone());
    Ok(())
}

/// Sets every item's enabled state and check in place.
fn update(app: &AppHandle, menu: &Menu<Wry>, place: &Place) -> tauri::Result<()> {
    let registry = app.state::<crate::bus::Commands>();
    let commands = registry.available(app);
    let mut pending = menu.items()?;
    while let Some(item) = pending.pop() {
        match item {
            MenuItemKind::Submenu(submenu) => pending.extend(submenu.items()?),
            MenuItemKind::MenuItem(item) if commands.knows(item.id().as_ref()) => {
                item.set_enabled(commands.is_enabled(item.id().as_ref()))?;
            }
            MenuItemKind::Check(item) => {
                let id = item.id().as_ref();
                if let Some(checked) = place.checked(id).or_else(|| place.shows_view_item(id)) {
                    item.set_checked(checked)?;
                }
                if commands.knows(id) {
                    item.set_enabled(commands.is_enabled(id))?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Shows the current place in the menus: a new set of views rebuilds them, a move within it updates them.
pub fn refresh(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let place = read(&app).await;
        let handle = app.clone();
        let rebuilt = app.run_on_main_thread(move || {
            if let Err(e) = show(&handle, &place) {
                log::warn!("Couldn't update the menus: {e}");
            }
        });
        if let Err(e) = rebuilt {
            log::warn!("Couldn't update the menus: {e}");
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

    fn viewing(view_id: Option<i64>) -> Place {
        Place {
            views: vec![
                (10, "Sprint".to_string()),
                (20, "Backlog".to_string()),
                (30, "Done".to_string()),
            ],
            view_id,
            ..Place::default()
        }
    }

    #[test]
    fn only_the_shown_views_item_is_the_shown_one() {
        let place = viewing(Some(20));

        for (command, expected) in [
            ("page:view.select.1", Some(false)),
            ("page:view.select.2", Some(true)),
            ("page:view.select.3", Some(false)),
        ] {
            assert_eq!(place.shows_view_item(command), expected, "{command}");
        }
    }

    #[test]
    fn without_a_shown_view_no_view_item_is_the_shown_one() {
        let place = viewing(None);

        for command in ["page:view.select.1", "page:view.select.3"] {
            assert_eq!(place.shows_view_item(command), Some(false), "{command}");
        }
    }

    #[test]
    fn ids_that_are_no_view_item_of_the_level_have_no_answer() {
        let place = viewing(Some(10));

        for command in [
            "page:view.select.0",
            "page:view.select.4",
            "page:view.select.9",
            "page:view.select.",
            "page:view.select.one",
            "page:view.select.-1",
            "page:view.select.1.2",
            "page:view.layout.list",
            "nav:nextList",
            "",
        ] {
            assert_eq!(place.shows_view_item(command), None, "{command}");
        }
    }

    #[test]
    fn a_level_without_views_has_no_view_item() {
        let place = showing(Some(Layout::List));

        for command in ["page:view.select.1", "page:view.select.0"] {
            assert_eq!(place.shows_view_item(command), None, "{command}");
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
