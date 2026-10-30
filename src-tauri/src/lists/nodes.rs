//! The rows of the sidebar tree (`filter_list`) and the conditions each one stands for.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::filter::{Condition, Field, Filter};
use crate::views::model::from_text;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts", rename = "ListKind")]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Everything,
    Space,
    List,
    Recent,
    Pinned,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub kind: Kind,
    pub project_id: Option<String>,
    pub name: String,
    pub filter: Option<String>,
    pub position: i64,
    /// A filter list's icon; none shows the folder.
    pub icon: Option<String>,
    /// Its tile color (`look`); none shows the icon plain.
    pub color: Option<String>,
}

/// Which issues a node holds: the conditions it inherits (locked in the Filter popover) and its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub inherited: Vec<Condition>,
    /// An error when the stored filter can't be read.
    pub own: Result<Vec<Condition>, String>,
}

impl Scope {
    pub fn all(&self) -> AppResult<Vec<Condition>> {
        let own = self.own.clone().map_err(AppError::InvalidInput)?;
        Ok([self.inherited.clone(), own].concat())
    }
}

const COLUMNS: &str = "id, parent_id, kind, project_id, name, filter, position, icon, color";

fn from_row(row: &Row) -> rusqlite::Result<Node> {
    Ok(Node {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        kind: from_text(&row.get::<_, String>(2)?).unwrap_or(Kind::List),
        project_id: row.get(3)?,
        name: row.get(4)?,
        filter: row.get(5)?,
        position: row.get(6)?,
        icon: row.get(7)?,
        color: row.get(8)?,
    })
}

pub fn all_nodes(conn: &Connection) -> AppResult<Vec<Node>> {
    let mut query = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM filter_list ORDER BY position, id"
    ))?;
    let nodes = query.query_map([], from_row)?;
    Ok(nodes.collect::<Result<_, _>>()?)
}

pub fn node(conn: &Connection, id: i64) -> AppResult<Node> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM filter_list WHERE id = ?1"),
        [id],
        from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("Filter list {id} doesn't exist")))
}

pub fn space_node(conn: &Connection, project_id: &str) -> AppResult<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM filter_list WHERE kind = 'space' AND project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// What `node` inherits from its ancestors and what it adds itself. A Space adds `project = X` as an
/// inherited condition, so its lists can never match another project.
pub fn scope_of(nodes: &[Node], node: &Node) -> Scope {
    let mut inherited = Vec::new();
    let mut ancestor = node.parent_id;
    let mut chain = Vec::new();
    // A cycle can't be stored through the commands; the depth bound only guards against a hand-edited file.
    while let Some(id) = ancestor.filter(|_| chain.len() < nodes.len()) {
        let Some(parent) = nodes.iter().find(|n| n.id == id) else {
            break;
        };
        chain.push(parent);
        ancestor = parent.parent_id;
    }
    for parent in chain.into_iter().rev() {
        match own_conditions(parent) {
            Ok(conditions) => inherited.extend(conditions),
            Err(message) => {
                return Scope {
                    inherited,
                    own: Err(format!("“{}”: {message}", parent.name)),
                };
            }
        }
    }
    match node.kind {
        Kind::Space => {
            inherited.extend(own_conditions(node).unwrap_or_default());
            Scope {
                inherited,
                own: Ok(Vec::new()),
            }
        }
        _ => Scope {
            inherited,
            own: own_conditions(node),
        },
    }
}

pub(super) fn own_conditions(node: &Node) -> Result<Vec<Condition>, String> {
    match (node.kind, &node.project_id) {
        (Kind::Space, Some(project_id)) => Ok(vec![Condition::is(Field::Project, project_id)]),
        _ => Filter::parse(node.filter.as_deref())
            .map(|filter| filter.conditions)
            .map_err(|e| e.to_string()),
    }
}
