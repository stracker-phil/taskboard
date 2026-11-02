//! The main window's tabs (ADR 0050): each tab is a place; the active one's is `Saved::location`, so everything
//! that follows the place (menus, the CLI, Unread's read-here) follows the active tab.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::history::History;
use super::{Location, Nav, NavStore, Saved};
use crate::db::Db;
use crate::error::AppResult;

/// Closed tabs ⌘⌥T can bring back.
const CLOSED_KEPT: usize = 10;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct SavedTab {
    pub id: u32,
    pub location: Option<Location>,
    /// The issue its Issue view shows, as the page reports it (`NavStore::set_issue`).
    #[serde(default)]
    pub issue: Option<String>,
    #[serde(default)]
    pub history: History,
}

/// The active tab; `normalize` makes sure there is one.
pub(super) fn active_tab_mut(saved: &mut Saved) -> Option<&mut SavedTab> {
    let active = saved.active_tab;
    saved.tabs.iter_mut().find(|t| t.id == active)
}

/// A tab as the page draws it.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: u32,
    /// None only while no level exists.
    pub location: Option<Location>,
}

/// A file from before tabs holds one place: it becomes the one tab. The active tab always holds the place.
pub(super) fn normalize(saved: &mut Saved) {
    if saved.tabs.is_empty() {
        let id = saved.next_tab.max(1);
        saved.tabs.push(SavedTab {
            id,
            location: saved.location,
            ..SavedTab::default()
        });
        saved.active_tab = id;
    }
    if !saved.tabs.iter().any(|t| t.id == saved.active_tab) {
        saved.active_tab = saved.tabs[0].id;
    }
    saved.next_tab = saved
        .next_tab
        .max(saved.tabs.iter().map(|t| t.id).max().unwrap_or(0) + 1);
    let (active, location) = (saved.active_tab, saved.location);
    if let Some(tab) = saved.tabs.iter_mut().find(|t| t.id == active) {
        tab.location = location;
    }
}

impl NavStore {
    pub fn tab_count(&self) -> usize {
        self.store.read(|s| s.tabs.len().max(1))
    }

    pub fn active_tab(&self) -> u32 {
        self.store.read(|s| {
            let mut s = s.clone();
            normalize(&mut s);
            s.active_tab
        })
    }

    /// A tab after the active one at the level `list_id` (its view `view_id`, else its last or default one), or at
    /// the active tab's place when none; `activate` switches to it.
    pub async fn new_tab(
        &self,
        db: &Db,
        list_id: Option<i64>,
        view_id: Option<i64>,
        activate: bool,
    ) -> AppResult<Nav> {
        let saved = self.store.read(Saved::clone);
        let at = match list_id {
            Some(list_id) => {
                db.read(move |conn| super::resolve(conn, &saved, list_id, view_id))
                    .await?
            }
            None => None,
        };
        let id = self
            .store
            .update_async(move |s| {
                normalize(s);
                let id = s.next_tab;
                s.next_tab += 1;
                let after = s
                    .tabs
                    .iter()
                    .position(|t| t.id == s.active_tab)
                    .map_or(s.tabs.len(), |i| i + 1);
                s.tabs.insert(
                    after,
                    SavedTab {
                        id,
                        location: at.or(s.location),
                        ..SavedTab::default()
                    },
                );
                id
            })
            .await?;
        if activate {
            self.activate_tab(db, id).await
        } else {
            self.current(db).await
        }
    }

    /// Shows the tab's place, as `current` resolves it (a tab that never had one, as on a fresh install, is at the
    /// starting place); the tab keeps it as `go` moves it.
    pub async fn activate_tab(&self, db: &Db, id: u32) -> AppResult<Nav> {
        let shown = self.current(db).await?;
        let Some(tab) = shown.tabs.iter().find(|t| t.id == id).copied() else {
            return Ok(shown);
        };
        self.store
            .update_async(move |s| {
                normalize(s);
                s.active_tab = id;
            })
            .await?;
        match tab.location {
            Some(at) => self.show(db, at.list_id, Some(at.view_id), false).await,
            None => self.current(db).await,
        }
    }

    /// The next (`step` 1) or previous (-1) tab, wrapping around.
    pub async fn step_tab(&self, db: &Db, step: i64) -> AppResult<Nav> {
        let next = self.store.read(move |s| {
            let mut s = s.clone();
            normalize(&mut s);
            let at = s.tabs.iter().position(|t| t.id == s.active_tab)? as i64;
            let len = s.tabs.len() as i64;
            Some(s.tabs[(at + step).rem_euclid(len) as usize].id)
        });
        match next {
            Some(id) => self.activate_tab(db, id).await,
            None => self.current(db).await,
        }
    }

    /// Closes the tab; the last one stays. Closing the active tab shows its right neighbor, else its left.
    pub async fn close_tab(&self, db: &Db, id: u32) -> AppResult<Nav> {
        let next = self
            .store
            .update_async(move |s| {
                normalize(s);
                if s.tabs.len() < 2 {
                    return None;
                }
                let at = s.tabs.iter().position(|t| t.id == id)?;
                let closed = s.tabs.remove(at);
                if let Some(location) = closed.location {
                    s.closed.push(location);
                    let over = s.closed.len().saturating_sub(CLOSED_KEPT);
                    s.closed.drain(..over);
                }
                (closed.id == s.active_tab).then(|| s.tabs[at.min(s.tabs.len() - 1)].id)
            })
            .await?;
        match next {
            Some(id) => self.activate_tab(db, id).await,
            None => self.current(db).await,
        }
    }

    /// Opens the last closed tab again, active.
    pub async fn reopen_tab(&self, db: &Db) -> AppResult<Nav> {
        let last = self.store.update_async(|s| s.closed.pop()).await?;
        match last {
            Some(at) => {
                self.new_tab(db, Some(at.list_id), Some(at.view_id), true)
                    .await
            }
            None => self.current(db).await,
        }
    }

    /// Moves the tab before `before`, or to the end.
    pub async fn move_tab(&self, db: &Db, id: u32, before: Option<u32>) -> AppResult<Nav> {
        self.store
            .update_async(move |s| {
                normalize(s);
                let Some(from) = s.tabs.iter().position(|t| t.id == id) else {
                    return;
                };
                let tab = s.tabs.remove(from);
                let to = before
                    .and_then(|b| s.tabs.iter().position(|t| t.id == b))
                    .unwrap_or(s.tabs.len());
                s.tabs.insert(to, tab);
            })
            .await?;
        self.current(db).await
    }
}
