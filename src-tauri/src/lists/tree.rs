//! The sidebar tree with open-issue counts, counted in one pass over the cache.

use rusqlite::{Connection, params_from_iter};
use serde::Serialize;
use ts_rs::TS;

use super::context::{Viewer, names};
use super::nodes::{Kind, Node, all_nodes, scope_of};
use crate::changes::UNSEEN;
use crate::db::Db;
use crate::error::AppResult;
use crate::filter::{Compiled, Ctx, Names, OPEN_ISSUES, TRACKED_ISSUES, compile, describe};
use crate::pins::active;
use crate::time::format_utc;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SidebarTree {
    pub everything: ListNode,
    /// Unread, Bookmarks and global filter lists, in order.
    pub home: Vec<ListNode>,
    /// Space lists pinned to Home, newest pin first.
    pub pinned_lists: Vec<ListNode>,
    /// Tracked projects in Space order.
    pub spaces: Vec<SpaceNode>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ListNode {
    pub id: i64,
    pub kind: Kind,
    pub name: String,
    /// Open issues, or unread ones under Unread only; for Bookmarks its active pins; none for a filter that can't
    /// be compiled.
    pub count: Option<i64>,
    /// Conditions of its own, for the Filter button's badge.
    pub own_conditions: usize,
    /// Its own conditions in words.
    pub description: String,
    /// Why its filter can't be used.
    pub error: Option<String>,
    /// The icon the owner picked for a filter list (`look`); none shows the folder.
    pub icon: Option<String>,
    /// Its tile color; none shows the icon plain.
    pub color: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SpaceNode {
    /// The Space's own node; its views hang off it.
    pub list_id: i64,
    pub project_id: String,
    pub key: String,
    pub name: String,
    /// The Jira project's own name; `name` differs when the owner renamed the Space.
    pub project_name: String,
    pub icon: Option<String>,
    /// A Space tile color token number.
    pub color: Option<String>,
    pub client: Option<String>,
    pub open_count: i64,
    pub lists: Vec<ListNode>,
}

/// `unread`: every count is of unread issues (Unread only), open or done; otherwise of open issues.
pub async fn sidebar_tree(db: &Db, viewer: Viewer, unread: bool) -> AppResult<SidebarTree> {
    db.read(move |conn| build(conn, &viewer, unread)).await
}

struct SpaceRow {
    project_id: String,
    key: String,
    name: String,
    project_name: String,
    icon: Option<String>,
    color: Option<String>,
    client: Option<String>,
}

fn tracked_spaces(conn: &Connection) -> AppResult<Vec<SpaceRow>> {
    let mut query = conn.prepare(
        "SELECT s.project_id, p.key, COALESCE(s.name, p.name), p.name, s.icon, s.color, s.client
         FROM space s JOIN cache.space p ON p.id = s.project_id
         WHERE s.tracked = 1 ORDER BY s.position, p.key",
    )?;
    let rows = query.query_map([], |row| {
        Ok(SpaceRow {
            project_id: row.get(0)?,
            key: row.get(1)?,
            name: row.get(2)?,
            project_name: row.get(3)?,
            icon: row.get(4)?,
            color: row.get(5)?,
            client: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn build(conn: &Connection, viewer: &Viewer, unread: bool) -> AppResult<SidebarTree> {
    let ctx = viewer.ctx(conn)?;
    let names = names(conn)?;
    let nodes = all_nodes(conn)?;
    let mut counter = Counter::default();

    let everything_node = nodes.iter().find(|n| n.kind == Kind::Everything);
    let everything = match everything_node {
        Some(node) => list_node(node, &nodes, &ctx, &names, &mut counter),
        None => ListNode {
            id: 0,
            kind: Kind::Everything,
            name: "Everything".into(),
            count: None,
            own_conditions: 0,
            description: String::new(),
            error: None,
            icon: None,
            color: None,
        },
    };
    let home = nodes
        .iter()
        .filter(|n| n.parent_id.is_none() && matches!(n.kind, Kind::Pinned | Kind::List))
        .map(|n| list_node(n, &nodes, &ctx, &names, &mut counter))
        .collect();

    let pinned_lists = pinned_lists(conn, viewer)?
        .into_iter()
        .filter_map(|(id, title)| Some((nodes.iter().find(|n| n.id == id)?, title)))
        // A global list already sits under Home.
        .filter(|(n, _)| n.parent_id.is_some())
        .map(|(n, title)| ListNode {
            name: title.unwrap_or_else(|| n.name.clone()),
            ..list_node(n, &nodes, &ctx, &names, &mut counter)
        })
        .collect();

    let mut spaces = Vec::new();
    for space in tracked_spaces(conn)? {
        let Some(node) = nodes
            .iter()
            .find(|n| n.kind == Kind::Space && n.project_id.as_deref() == Some(&space.project_id))
        else {
            continue;
        };
        let own = list_node(node, &nodes, &ctx, &names, &mut counter);
        let lists = nodes
            .iter()
            .filter(|n| n.parent_id == Some(node.id))
            .map(|n| list_node(n, &nodes, &ctx, &names, &mut counter))
            .collect();
        spaces.push(SpaceNode {
            list_id: node.id,
            project_id: space.project_id,
            key: space.key,
            name: space.name,
            project_name: space.project_name,
            icon: space.icon,
            color: space.color,
            client: space.client,
            open_count: own.count.unwrap_or(0),
            lists,
        });
    }

    let mut tree = SidebarTree {
        everything,
        home,
        pinned_lists,
        spaces,
    };
    counter.fill(conn, &mut tree, unread)?;
    let pin_count = active_pin_count(conn, viewer)?;
    for node in tree.home.iter_mut().filter(|n| n.kind == Kind::Pinned) {
        node.count = Some(pin_count);
    }
    Ok(tree)
}

/// The bookmarked lists in the bookmarks' order, with the bookmark's own name.
fn pinned_lists(conn: &Connection, viewer: &Viewer) -> AppResult<Vec<(i64, Option<String>)>> {
    let mut query = conn.prepare(&format!(
        "SELECT CAST(p.target_id AS INTEGER), p.title FROM pin p WHERE p.target_kind = 'filter_list' AND {}
         ORDER BY p.position, p.id",
        active("?1")
    ))?;
    let ids = query.query_map([format_utc(viewer.now)], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(ids.collect::<Result<_, _>>()?)
}

fn active_pin_count(conn: &Connection, viewer: &Viewer) -> AppResult<i64> {
    Ok(conn.query_row(
        &format!("SELECT COUNT(*) FROM pin p WHERE {}", active("?1")),
        [format_utc(viewer.now)],
        |r| r.get(0),
    )?)
}

/// A node without its count; `Counter::fill` adds it once every node is known.
fn list_node(
    node: &Node,
    nodes: &[Node],
    ctx: &Ctx,
    names: &Names,
    counter: &mut Counter,
) -> ListNode {
    let scope = scope_of(nodes, node);
    let own = scope.own.clone().unwrap_or_default();
    let mut error = scope.own.as_ref().err().cloned();
    if error.is_none() && node.kind != Kind::Pinned {
        match scope.all().and_then(|all| compile(&all, ctx)) {
            Ok(compiled) => counter.add(node.id, compiled),
            Err(e) => error = Some(e.to_string()),
        }
    }
    ListNode {
        id: node.id,
        kind: node.kind,
        name: node.name.clone(),
        count: None,
        own_conditions: own.len(),
        description: describe(&own, names),
        error,
        icon: node.icon.clone(),
        color: node.color.clone(),
    }
}

/// Collects one compiled filter per node and counts them all in one query.
#[derive(Default)]
struct Counter {
    nodes: Vec<(i64, Compiled)>,
}

impl Counter {
    fn add(&mut self, id: i64, compiled: Compiled) {
        self.nodes.push((id, compiled));
    }

    fn counts(&self, conn: &Connection, unread: bool) -> AppResult<Vec<(i64, i64)>> {
        if self.nodes.is_empty() {
            return Ok(Vec::new());
        }
        let columns: Vec<String> = self
            .nodes
            .iter()
            .map(|(_, c)| format!("COUNT(*) FILTER (WHERE {})", c.condition))
            .collect();
        let counted = if unread { UNSEEN } else { OPEN_ISSUES };
        let sql = format!(
            "SELECT {} FROM cache.issue i WHERE {TRACKED_ISSUES} AND {counted}",
            columns.join(", ")
        );
        let params = self.nodes.iter().flat_map(|(_, c)| c.params.iter());
        let counts: Vec<i64> = conn.query_row(&sql, params_from_iter(params), |row| {
            (0..self.nodes.len()).map(|i| row.get(i)).collect()
        })?;
        Ok(self.nodes.iter().map(|(id, _)| *id).zip(counts).collect())
    }

    fn fill(self, conn: &Connection, tree: &mut SidebarTree, unread: bool) -> AppResult<()> {
        let counts = self.counts(conn, unread)?;
        let count_of = |id: i64| counts.iter().find(|(n, _)| *n == id).map(|(_, c)| *c);
        let mut set = |node: &mut ListNode| node.count = count_of(node.id);
        set(&mut tree.everything);
        tree.home.iter_mut().for_each(&mut set);
        tree.pinned_lists.iter_mut().for_each(&mut set);
        for space in &mut tree.spaces {
            space.open_count = count_of(space.list_id).unwrap_or(0);
            space.lists.iter_mut().for_each(&mut set);
        }
        Ok(())
    }
}
