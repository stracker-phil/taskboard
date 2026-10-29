//! What the Filter popover shows: a level's and a view's conditions with display names, and the values its
//! pickers offer.

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use super::context::names;
use super::nodes::{Kind, Node, all_nodes, node, own_conditions};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::filter::{Condition, Field, Names, Parts, RELEVANT_FIELD, parts};
use crate::views::{read_view, view_conditions};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ListConditions {
    pub kind: Kind,
    /// Conditions from the level itself (a Space's project) and its ancestors; they can't be edited here.
    pub locked: Vec<LockedCondition>,
    /// The filter list's own conditions; always empty on Everything, a Space or a Home level.
    pub own: Vec<Condition>,
    /// Why the stored filter can't be read.
    pub error: Option<String>,
    /// A filter list in a Space: the Space gives the project, so a condition can't pick one.
    pub project_fixed: bool,
    /// The view's own conditions (ADR 0033), ANDed with the level's.
    pub view: Vec<Condition>,
    /// Why the view's stored filter can't be read.
    pub view_error: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct LockedCondition {
    pub field: String,
    pub op: String,
    pub value: String,
    /// Where it comes from: "this Space" or "from Space".
    pub source: String,
}

/// `view_id`: a view of the level, whose own conditions come along.
pub async fn list_conditions(db: &Db, id: i64, view_id: Option<i64>) -> AppResult<ListConditions> {
    db.read(move |conn| {
        let mut conditions = conditions_of(conn, id)?;
        if let Some(view_id) = view_id {
            let view = read_view(conn, view_id)?;
            if view.list_id != id {
                return Err(AppError::NotFound(format!(
                    "View {view_id} isn't a view of filter list {id}"
                )));
            }
            match view_conditions(&view) {
                Ok(own) => conditions.view = own,
                Err(e) => conditions.view_error = Some(e.to_string()),
            }
        }
        Ok(conditions)
    })
    .await
}

fn conditions_of(conn: &Connection, id: i64) -> AppResult<ListConditions> {
    let nodes = all_nodes(conn)?;
    let target = node(conn, id)?;
    let names = names(conn)?;
    let mut locked = Vec::new();
    // A filter list sits under a Space or Everything, so only a Space passes conditions down.
    for ancestor in ancestors(&nodes, &target) {
        if let Ok(conditions) = own_conditions(ancestor) {
            locked.extend(conditions.iter().map(|c| lock(c, &names, "from Space")));
        }
    }
    let (own, error) = match (target.kind, own_conditions(&target)) {
        (Kind::Space, Ok(conditions)) => {
            locked.extend(conditions.iter().map(|c| lock(c, &names, "this Space")));
            (Vec::new(), None)
        }
        (Kind::List, Ok(conditions)) => (conditions, None),
        (_, Ok(_)) => (Vec::new(), None),
        (_, Err(message)) => (Vec::new(), Some(message)),
    };
    let project_fixed = ancestors(&nodes, &target)
        .iter()
        .any(|a| a.kind == Kind::Space);
    Ok(ListConditions {
        kind: target.kind,
        locked,
        own,
        error,
        project_fixed,
        view: Vec::new(),
        view_error: None,
    })
}

/// From the root down to the direct parent.
fn ancestors<'a>(nodes: &'a [Node], target: &Node) -> Vec<&'a Node> {
    let mut chain = Vec::new();
    let mut next = target.parent_id;
    while let Some(id) = next {
        // A parent link edited by hand into a cycle must not loop forever.
        if chain.len() == nodes.len() {
            break;
        }
        let Some(parent) = nodes.iter().find(|n| n.id == id) else {
            break;
        };
        chain.push(parent);
        next = parent.parent_id;
    }
    chain.reverse();
    chain
}

fn lock(condition: &Condition, names: &Names, source: &str) -> LockedCondition {
    let Parts { field, op, value } = parts(condition, names);
    LockedCondition {
        field,
        op,
        value,
        source: source.to_string(),
    }
}

/// How a field's value is picked.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum ValueKind {
    Status,
    StatusCategory,
    Type,
    User,
    Priority,
    Epic,
    Release,
    Sprint,
    Label,
    Project,
    Date,
    Text,
    Number,
    /// Words searched in the key, summary, description and comments.
    Search,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct FilterField {
    /// A filter field id (`status`, `customfield_10071`).
    pub id: String,
    pub name: String,
    pub kind: ValueKind,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Choice {
    pub id: String,
    pub name: String,
}

/// Everything the Filter popover's selects offer, from the cache.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct FilterOptions {
    pub fields: Vec<FilterField>,
    pub statuses: Vec<Choice>,
    pub types: Vec<Choice>,
    pub users: Vec<Choice>,
    pub priorities: Vec<Choice>,
    pub epics: Vec<Choice>,
    pub releases: Vec<Choice>,
    pub sprints: Vec<Choice>,
    pub labels: Vec<Choice>,
    pub projects: Vec<Choice>,
}

const STANDARD_FIELDS: &[(Field, &str, ValueKind)] = &[
    (Field::Status, "Status", ValueKind::Status),
    (
        Field::StatusCategory,
        "Status category",
        ValueKind::StatusCategory,
    ),
    (Field::Type, "Type", ValueKind::Type),
    (Field::Assignee, "Assignee", ValueKind::User),
    (Field::Reporter, "Reporter", ValueKind::User),
    (Field::Priority, "Priority", ValueKind::Priority),
    (Field::Epic, "Epic", ValueKind::Epic),
    (Field::Release, "Release", ValueKind::Release),
    (Field::Sprint, "Sprint", ValueKind::Sprint),
    (Field::Labels, "Labels", ValueKind::Label),
    (Field::Due, "Due date", ValueKind::Date),
    (Field::Updated, "Updated", ValueKind::Date),
    (Field::Created, "Created", ValueKind::Date),
    (Field::Project, "Project", ValueKind::Project),
    (Field::Text, "Text", ValueKind::Search),
];

pub async fn filter_options(db: &Db) -> AppResult<FilterOptions> {
    db.read(options).await
}

fn choices(conn: &Connection, sql: &str) -> AppResult<Vec<Choice>> {
    let mut query = conn.prepare(sql)?;
    let rows = query.query_map([], |row| {
        Ok(Choice {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The standard fields, then the site's custom fields a condition can use.
fn fields(conn: &Connection) -> AppResult<Vec<FilterField>> {
    let mut fields: Vec<FilterField> = STANDARD_FIELDS
        .iter()
        .map(|(field, name, kind)| FilterField {
            id: field.name().to_string(),
            name: name.to_string(),
            kind: *kind,
        })
        .collect();
    let mut query = conn.prepare(&format!(
        "SELECT id, name, COALESCE(type, '') FROM cache.field
         WHERE id LIKE 'customfield\\_%' ESCAPE '\\' AND COALESCE(type, '') <> 'any' AND name <> 'Sprint'
           AND {RELEVANT_FIELD}
         ORDER BY name COLLATE NOCASE, id"
    ))?;
    let custom = query.query_map([], |row| {
        let kind: String = row.get(2)?;
        Ok(FilterField {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: match kind.as_str() {
                "user" => ValueKind::User,
                "date" | "datetime" => ValueKind::Date,
                "number" => ValueKind::Number,
                _ => ValueKind::Text,
            },
        })
    })?;
    fields.extend(custom.collect::<Result<Vec<_>, _>>()?);
    Ok(fields)
}

/// The sprint states, then each cached sprint by name.
fn sprints(conn: &Connection) -> AppResult<Vec<Choice>> {
    let states = [
        ("active", "Open sprint"),
        ("future", "Future sprints"),
        ("closed", "Closed sprints"),
    ]
    .iter()
    .map(|(id, name)| Choice {
        id: id.to_string(),
        name: name.to_string(),
    });
    let named = choices(
        conn,
        "SELECT sprint_id, MAX(sprint) FROM cache.issue WHERE sprint_id IS NOT NULL
         GROUP BY sprint_id ORDER BY MAX(sprint) COLLATE NOCASE",
    )?;
    Ok(states.chain(named).collect())
}

fn options(conn: &Connection) -> AppResult<FilterOptions> {
    let tracked = "SELECT project_id FROM space WHERE tracked = 1";
    Ok(FilterOptions {
        fields: fields(conn)?,
        statuses: choices(
            conn,
            &format!(
                "SELECT id, name FROM cache.status
                 WHERE id IN (SELECT status_id FROM cache.issue WHERE gone = 0 AND project_id IN ({tracked}))
                 ORDER BY name COLLATE NOCASE"
            ),
        )?,
        types: choices(
            conn,
            &format!(
                "SELECT DISTINCT issue_type, issue_type FROM cache.issue WHERE project_id IN ({tracked})
                 ORDER BY 1 COLLATE NOCASE"
            ),
        )?,
        users: choices(conn, "SELECT account_id, name FROM cache.user ORDER BY name COLLATE NOCASE")?,
        priorities: choices(
            conn,
            "SELECT DISTINCT priority, priority FROM cache.issue WHERE priority IS NOT NULL
             ORDER BY CASE lower(priority) WHEN 'highest' THEN 1 WHEN 'high' THEN 2 WHEN 'medium' THEN 3
               WHEN 'low' THEN 4 WHEN 'lowest' THEN 5 ELSE 3 END, 1",
        )?,
        epics: choices(
            conn,
            &format!(
                "SELECT i.id, p.key || ' · ' || i.summary FROM cache.issue i JOIN cache.space p ON p.id = i.project_id
                 WHERE i.is_epic = 1 AND i.gone = 0 AND i.project_id IN ({tracked})
                 ORDER BY p.key, i.summary COLLATE NOCASE"
            ),
        )?,
        releases: choices(
            conn,
            &format!(
                "SELECT v.id, p.key || ' · ' || v.name FROM cache.version v JOIN cache.space p ON p.id = v.project_id
                 WHERE v.project_id IN ({tracked}) ORDER BY p.key, v.released, v.release_date, v.name"
            ),
        )?,
        sprints: sprints(conn)?,
        labels: choices(
            conn,
            &format!(
                "SELECT DISTINCT l.value, l.value FROM cache.issue i, json_each(i.labels) l
                 WHERE i.project_id IN ({tracked}) ORDER BY 1 COLLATE NOCASE"
            ),
        )?,
        projects: choices(
            conn,
            &format!(
                "SELECT p.id, p.key || ' · ' || p.name FROM cache.space p WHERE p.id IN ({tracked}) ORDER BY p.key"
            ),
        )?,
    })
}
