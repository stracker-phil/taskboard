//! Where the main window is: a sidebar level and one of its views (`nav.json`), per tab (`tabs.rs`). Each level
//! remembers its last view; the tabs and their places come back at launch.

pub mod commands;
mod history;
mod tabs;

pub use history::Direction;
use history::Step;
pub use tabs::Tab;
use tabs::{SavedTab, active_tab_mut, normalize};

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::Db;
use crate::error::AppResult;
use crate::lists::{Kind, Node, all_nodes, node};
use crate::store::json_store::JsonStore;
use crate::views::{View, list_views};

/// A sidebar level (a `filter_list` id: Everything, a Home list, a Space or a filter list) and its view.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub list_id: i64,
    pub view_id: i64,
}

/// What the page needs to draw the sidebar's place.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Nav {
    /// None only while no level exists (a broken local file).
    pub location: Option<Location>,
    /// Space nodes the owner expanded; the active Space shows expanded regardless.
    pub expanded: Vec<i64>,
    /// The open tabs in strip order, at least one; the active one is at `location`.
    pub tabs: Vec<Tab>,
    pub active_tab: u32,
    /// The issue the active tab's Issue view shows, as the page last reported it; Back and Forward set it.
    pub issue: Option<String>,
    /// The active tab has somewhere to go back or forward to.
    pub can_back: bool,
    pub can_forward: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
struct Saved {
    /// The active tab's place.
    location: Option<Location>,
    /// Level id (as text, a JSON key) → its last view.
    last_views: BTreeMap<String, i64>,
    expanded: Vec<i64>,
    /// Empty in a file from before tabs (`tabs::normalize`).
    tabs: Vec<SavedTab>,
    active_tab: u32,
    next_tab: u32,
    /// The places of the last closed tabs, newest last.
    closed: Vec<Location>,
}

/// Told when the place changes, whoever moved it (the page, a menu, the CLI).
pub type OnMove = Box<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct NavStore {
    store: JsonStore<Saved>,
    /// Issues read while the owner stays at this place: Unread keeps showing them until the place changes.
    read_here: Arc<Mutex<BTreeSet<String>>>,
    on_move: Arc<Mutex<Option<OnMove>>>,
}

const NAV_FILE: &str = "nav.json";

impl NavStore {
    pub fn open(dir: &Path) -> Self {
        Self {
            store: JsonStore::open(dir.join(NAV_FILE)),
            read_here: Arc::default(),
            on_move: Arc::default(),
        }
    }

    pub fn on_move(&self, notify: OnMove) {
        *self.on_move.lock().unwrap_or_else(|e| e.into_inner()) = Some(notify);
    }

    fn moved(&self) {
        if let Some(notify) = &*self.on_move.lock().unwrap_or_else(|e| e.into_inner()) {
            notify();
        }
    }

    /// The saved place if it still exists, else Everything; every tab's likewise.
    pub async fn current(&self, db: &Db) -> AppResult<Nav> {
        let mut saved = self.store.read(Saved::clone);
        normalize(&mut saved);
        let expanded = saved.expanded.clone();
        let active_tab = saved.active_tab;
        let (issue, can_back, can_forward) =
            active_tab_mut(&mut saved).map_or((None, false, false), |tab| {
                (
                    tab.issue.clone(),
                    tab.history.can(Direction::Back),
                    tab.history.can(Direction::Forward),
                )
            });
        let (location, tabs) = db
            .read(move |conn| {
                let place = |at: Option<Location>| -> AppResult<Option<Location>> {
                    if let Some(at) = at
                        && let Some(found) = resolve(conn, &saved, at.list_id, Some(at.view_id))?
                    {
                        return Ok(Some(found));
                    }
                    match everything(conn)? {
                        Some(id) => resolve(conn, &saved, id, None),
                        None => Ok(None),
                    }
                };
                let location = place(saved.location)?;
                let tabs = saved
                    .tabs
                    .iter()
                    .map(|tab| {
                        let location = if tab.id == saved.active_tab {
                            location
                        } else {
                            place(tab.location)?
                        };
                        Ok(Tab {
                            id: tab.id,
                            location,
                        })
                    })
                    .collect::<AppResult<Vec<_>>>()?;
                Ok((location, tabs))
            })
            .await?;
        Ok(Nav {
            location,
            expanded,
            tabs,
            active_tab,
            issue,
            can_back,
            can_forward,
        })
    }

    /// Opens `list_id` at `view_id`, or at the view it showed last, or its default view. A new place is a step
    /// Back returns from.
    pub async fn go(&self, db: &Db, list_id: i64, view_id: Option<i64>) -> AppResult<Nav> {
        self.show(db, list_id, view_id, true).await
    }

    /// `go`; `step` notes the place left in the active tab's history and closes its issue.
    async fn show(
        &self,
        db: &Db,
        list_id: i64,
        view_id: Option<i64>,
        step: bool,
    ) -> AppResult<Nav> {
        let saved = self.store.read(Saved::clone);
        let before = saved.location;
        let left = if step { self.here(db).await? } else { None };
        let found = db
            .read(move |conn| {
                node(conn, list_id)?;
                resolve(conn, &saved, list_id, view_id)
            })
            .await?;
        if let Some(at) = found {
            if before != Some(at) {
                self.read_here().clear();
            }
            self.store
                .update_async(move |s| {
                    normalize(s);
                    if step
                        && left != Some(at)
                        && let Some(tab) = active_tab_mut(s)
                    {
                        if let Some(from) = left {
                            let issue = tab.issue.take();
                            tab.history.left(Step {
                                location: from,
                                issue,
                            });
                        }
                        tab.issue = None;
                    }
                    s.location = Some(at);
                    s.last_views.insert(at.list_id.to_string(), at.view_id);
                    normalize(s);
                })
                .await?;
            if before != Some(at) {
                self.moved();
            }
        }
        self.current(db).await
    }

    /// The page shows `issue` in the active tab's Issue view, or none: another issue is a step Back returns from.
    pub async fn set_issue(&self, db: &Db, issue: Option<String>) -> AppResult<Nav> {
        let location = self.here(db).await?;
        self.store
            .update_async(move |s| {
                normalize(s);
                let Some(tab) = active_tab_mut(s) else {
                    return;
                };
                if tab.issue == issue {
                    return;
                }
                if let Some(location) = location {
                    let from = Step {
                        location,
                        issue: tab.issue.take(),
                    };
                    tab.history.left(from);
                }
                tab.issue = issue;
            })
            .await?;
        self.current(db).await
    }

    /// The place shown: the saved one, or where a fresh store starts before it saved one.
    async fn here(&self, db: &Db) -> AppResult<Option<Location>> {
        match self.store.read(|s| s.location) {
            Some(at) => Ok(Some(at)),
            None => Ok(self.current(db).await?.location),
        }
    }

    /// Goes back or forward in the active tab's history, past steps whose level is gone. `Nav::issue` is the issue
    /// to show there.
    pub async fn travel(&self, db: &Db, direction: Direction) -> AppResult<Nav> {
        loop {
            let next = self
                .store
                .update_async(move |s| {
                    normalize(s);
                    active_tab_mut(s).and_then(|tab| tab.history.take(direction))
                })
                .await?;
            let Some(next) = next else {
                return self.current(db).await;
            };
            let saved = self.store.read(Saved::clone);
            let before = saved.location;
            let (list_id, view_id) = (next.location.list_id, next.location.view_id);
            let found = db
                .read(move |conn| resolve(conn, &saved, list_id, Some(view_id)))
                .await?;
            let Some(at) = found else {
                continue;
            };
            self.store
                .update_async(move |s| {
                    normalize(s);
                    let location = s.location;
                    if let Some(tab) = active_tab_mut(s) {
                        if let Some(location) = location {
                            let issue = tab.issue.take();
                            tab.history.arrived(direction, Step { location, issue });
                        }
                        tab.issue = next.issue;
                    }
                    s.location = Some(at);
                    s.last_views.insert(at.list_id.to_string(), at.view_id);
                    normalize(s);
                })
                .await?;
            if before != Some(at) {
                self.read_here().clear();
                self.moved();
            }
            return self.current(db).await;
        }
    }

    /// The next (`step` 1) or previous (-1) tracked Space, wrapping around; from a level outside the Spaces,
    /// the first or the last.
    pub async fn step_space(&self, db: &Db, step: i64) -> AppResult<Nav> {
        let at = self.current(db).await?.location;
        let target = db
            .read(move |conn| {
                let spaces = tracked_space_nodes(conn)?;
                if spaces.is_empty() {
                    return Ok(None);
                }
                let here = match at {
                    Some(at) => space_of(conn, at.list_id)?,
                    None => None,
                };
                let len = spaces.len() as i64;
                let index = match here.and_then(|id| spaces.iter().position(|s| *s == id)) {
                    Some(i) => (i as i64 + step).rem_euclid(len),
                    None if step < 0 => len - 1,
                    None => 0,
                };
                Ok(Some(spaces[index as usize]))
            })
            .await?;
        match target {
            Some(id) => self.go(db, id, None).await,
            None => self.current(db).await,
        }
    }

    /// The next (`step` 1) or previous (-1) level of the current one's section, wrapping around: a Space and its
    /// lists, or the levels outside the Spaces.
    pub async fn step_list(&self, db: &Db, step: i64) -> AppResult<Nav> {
        let Some(at) = self.current(db).await?.location else {
            return self.current(db).await;
        };
        let target = db
            .read(move |conn| {
                let levels = section_levels(&all_nodes(conn)?, space_of(conn, at.list_id)?);
                let Some(index) = levels.iter().position(|id| *id == at.list_id) else {
                    return Ok(None);
                };
                let next = (index as i64 + step).rem_euclid(levels.len() as i64);
                Ok(Some(levels[next as usize]))
            })
            .await?;
        match target {
            Some(id) if id != at.list_id => self.go(db, id, None).await,
            _ => self.current(db).await,
        }
    }

    pub fn keep_read(&self, ids: &[String]) {
        self.read_here().extend(ids.iter().cloned());
    }

    pub fn read_here_ids(&self) -> Vec<String> {
        self.read_here().iter().cloned().collect()
    }

    fn read_here(&self) -> std::sync::MutexGuard<'_, BTreeSet<String>> {
        self.read_here.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub async fn set_expanded(&self, db: &Db, expanded: Vec<i64>) -> AppResult<Nav> {
        self.store
            .update_async(move |s| s.expanded = expanded)
            .await?;
        self.current(db).await
    }
}

/// `list_id` at `view_id` when that view is one of its views, else at its last or default view. None when the
/// level has no view.
fn resolve(
    conn: &Connection,
    saved: &Saved,
    list_id: i64,
    view_id: Option<i64>,
) -> AppResult<Option<Location>> {
    if node(conn, list_id).is_err() {
        return Ok(None);
    }
    let views = list_views(conn, list_id)?;
    let has = |id: i64| views.iter().any(|v: &View| v.id == id);
    let last = saved.last_views.get(&list_id.to_string()).copied();
    let chosen = view_id
        .filter(|id| has(*id))
        .or(last.filter(|id| has(*id)))
        .or(views.iter().find(|v| v.is_default).map(|v| v.id))
        .or(views.first().map(|v| v.id));
    Ok(chosen.map(|view_id| Location { list_id, view_id }))
}

fn everything(conn: &Connection) -> AppResult<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM filter_list WHERE kind = 'everything' ORDER BY id LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

/// The Space node of tracked projects, in sidebar order.
fn tracked_space_nodes(conn: &Connection) -> AppResult<Vec<i64>> {
    let mut query = conn.prepare(
        "SELECT f.id FROM space s
         JOIN cache.space p ON p.id = s.project_id
         JOIN filter_list f ON f.kind = 'space' AND f.project_id = s.project_id
         WHERE s.tracked = 1 ORDER BY s.position, p.key",
    )?;
    let ids = query.query_map([], |row| row.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
}

/// A section's levels in sidebar order (each before its children): the Space `space` and its lists, or without one
/// every level outside the Spaces, Everything first.
fn section_levels(nodes: &[Node], space: Option<i64>) -> Vec<i64> {
    fn walk(nodes: &[Node], parent: Option<i64>, out: &mut Vec<i64>) {
        for child in nodes.iter().filter(|n| n.parent_id == parent) {
            if parent.is_none() && matches!(child.kind, Kind::Space | Kind::Everything) {
                continue;
            }
            out.push(child.id);
            walk(nodes, Some(child.id), out);
        }
    }
    let mut levels = Vec::from_iter(space);
    if space.is_none() {
        levels.extend(
            nodes
                .iter()
                .filter(|n| n.kind == Kind::Everything)
                .map(|n| n.id),
        );
    }
    walk(nodes, space, &mut levels);
    levels
}

/// The Space a level belongs to: itself, or the Space above a filter list.
fn space_of(conn: &Connection, list_id: i64) -> AppResult<Option<i64>> {
    let mut id = Some(list_id);
    while let Some(current) = id {
        let level = node(conn, current)?;
        if level.kind == Kind::Space {
            return Ok(Some(level.id));
        }
        id = level.parent_id;
    }
    Ok(None)
}
