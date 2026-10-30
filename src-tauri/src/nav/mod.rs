//! Where the main window is: a sidebar level and one of its views (`nav.json`). Each level remembers its last
//! view; the last place comes back at launch.

pub mod commands;

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
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
struct Saved {
    location: Option<Location>,
    /// Level id (as text, a JSON key) → its last view.
    last_views: BTreeMap<String, i64>,
    expanded: Vec<i64>,
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

    /// The saved place if it still exists, else Everything.
    pub async fn current(&self, db: &Db) -> AppResult<Nav> {
        let saved = self.store.read(Saved::clone);
        let expanded = saved.expanded.clone();
        let location = db
            .read(move |conn| {
                if let Some(at) = saved.location
                    && let Some(found) = resolve(conn, &saved, at.list_id, Some(at.view_id))?
                {
                    return Ok(Some(found));
                }
                match everything(conn)? {
                    Some(id) => resolve(conn, &saved, id, None),
                    None => Ok(None),
                }
            })
            .await?;
        Ok(Nav { location, expanded })
    }

    /// Opens `list_id` at `view_id`, or at the view it showed last, or its default view.
    pub async fn go(&self, db: &Db, list_id: i64, view_id: Option<i64>) -> AppResult<Nav> {
        let saved = self.store.read(Saved::clone);
        let before = saved.location;
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
                    s.location = Some(at);
                    s.last_views.insert(at.list_id.to_string(), at.view_id);
                })
                .await?;
            if before != Some(at) {
                self.moved();
            }
        }
        self.current(db).await
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
/// every level outside the Spaces.
fn section_levels(nodes: &[Node], space: Option<i64>) -> Vec<i64> {
    fn walk(nodes: &[Node], parent: Option<i64>, out: &mut Vec<i64>) {
        for child in nodes.iter().filter(|n| n.parent_id == parent) {
            if parent.is_none() && child.kind == Kind::Space {
                continue;
            }
            out.push(child.id);
            walk(nodes, Some(child.id), out);
        }
    }
    let mut levels = Vec::from_iter(space);
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
