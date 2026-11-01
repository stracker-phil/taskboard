//! Who is on a Space's team. Someone ever assigned one of its synced issues is on it unless the owner switched them
//! off; anyone else with activity there (reporter, commenter, history author) or anyone the owner switched on joins
//! only when switched on (`space_member`). Settings → Team edits every Space at once.

pub mod commands;

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::space_config::project_of_level;
use crate::store::settings::Connection as JiraConnection;
use crate::views::rows::{Person, person};

/// Who the connected account is, from the last Test Connection.
#[derive(Clone, Debug, PartialEq)]
pub struct Me {
    pub account_id: String,
    pub name: String,
}

impl Me {
    pub fn of(connection: &JiraConnection) -> Option<Self> {
        Some(Self {
            account_id: connection.account_id.clone()?,
            name: connection.display_name.clone().unwrap_or_default(),
        })
    }
}

/// Everyone with activity in a tracked Space, and on which Spaces' teams they are.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TeamMatrix {
    /// The tracked Spaces' project ids, in sidebar order: the columns.
    pub spaces: Vec<String>,
    /// Sorted by name.
    pub people: Vec<TeamRow>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TeamRow {
    pub person: Person,
    /// One per column of `spaces`.
    pub cells: Vec<TeamCell>,
}

#[derive(Serialize, Clone, Copy, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TeamCell {
    pub active: bool,
    /// Ever assigned one of the Space's issues: on the team unless switched off.
    pub assignee: bool,
    /// Reported, commented on or changed one of its issues.
    pub involved: bool,
}

/// `?1`: a JSON array of project ids. One row per Space and person with activity there or switched on there: whether
/// they were ever assigned an issue of it (now, in the recorded changes or in an issue's history), and the owner's
/// choice, if any.
const MEMBERSHIPS: &str = "
    WITH scoped AS (SELECT id, project_id, assignee_id, reporter_id FROM cache.issue
                    WHERE gone = 0 AND project_id IN (SELECT value FROM json_each(?1))),
    assigned AS (
        SELECT project_id, assignee_id AS account_id FROM scoped WHERE assignee_id IS NOT NULL
        UNION SELECT s.project_id, c.new_value FROM cache.issue_change c JOIN scoped s ON s.id = c.issue_id
              WHERE c.field = 'assignee' AND c.new_value IS NOT NULL
        UNION SELECT s.project_id, h.to_value FROM cache.issue_history h JOIN scoped s ON s.id = h.issue_id
              WHERE h.field_id = 'assignee' AND h.to_value IS NOT NULL
    ),
    involved AS (
        SELECT project_id, account_id FROM assigned
        UNION SELECT project_id, reporter_id FROM scoped WHERE reporter_id IS NOT NULL
        UNION SELECT s.project_id, c.author_id FROM cache.comment c JOIN scoped s ON s.id = c.issue_id
              WHERE c.author_id IS NOT NULL
        UNION SELECT s.project_id, h.author_id FROM cache.issue_history h JOIN scoped s ON s.id = h.issue_id
              WHERE h.author_id IS NOT NULL
    ),
    listed AS (
        SELECT project_id, account_id FROM involved
        UNION SELECT project_id, account_id FROM space_member
              WHERE active = 1 AND project_id IN (SELECT value FROM json_each(?1))
    )
    SELECT l.project_id, l.account_id, u.name,
           EXISTS (SELECT 1 FROM assigned a WHERE a.project_id = l.project_id AND a.account_id = l.account_id),
           EXISTS (SELECT 1 FROM involved i WHERE i.project_id = l.project_id AND i.account_id = l.account_id),
           (SELECT m.active FROM space_member m WHERE m.project_id = l.project_id AND m.account_id = l.account_id)
    FROM listed l JOIN cache.user u ON u.account_id = l.account_id";

struct Membership {
    project_id: String,
    account_id: String,
    name: String,
    cell: TeamCell,
}

fn memberships(conn: &Connection, project_ids: &[String]) -> AppResult<Vec<Membership>> {
    let mut query = conn.prepare(MEMBERSHIPS)?;
    let projects = serde_json::to_string(project_ids)?;
    let rows = query.query_map([projects], |r| {
        let assignee: bool = r.get(3)?;
        let chosen: Option<bool> = r.get(5)?;
        Ok(Membership {
            project_id: r.get(0)?,
            account_id: r.get(1)?,
            name: r.get(2)?,
            cell: TeamCell {
                active: chosen.unwrap_or(assignee),
                assignee,
                involved: r.get(4)?,
            },
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn tracked_projects(conn: &Connection) -> AppResult<Vec<String>> {
    let mut query = conn
        .prepare("SELECT project_id FROM space WHERE tracked = 1 ORDER BY position, project_id")?;
    let rows = query.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Every tracked Space's team in one grid, people by name.
pub fn team_matrix(conn: &Connection, me: Option<&str>) -> AppResult<TeamMatrix> {
    let spaces = tracked_projects(conn)?;
    let column: HashMap<&str, usize> = spaces
        .iter()
        .enumerate()
        .map(|(i, p)| (p.as_str(), i))
        .collect();
    let mut rows: HashMap<String, TeamRow> = HashMap::new();
    for m in memberships(conn, &spaces)? {
        let Some(&at) = column.get(m.project_id.as_str()) else {
            continue;
        };
        let row = rows.entry(m.account_id.clone()).or_insert_with(|| TeamRow {
            person: person(&m.account_id, &m.name, me),
            cells: vec![TeamCell::default(); spaces.len()],
        });
        row.cells[at] = m.cell;
    }
    let mut people: Vec<TeamRow> = rows.into_values().collect();
    people.sort_by(|a, b| {
        (a.person.name.to_lowercase(), &a.person.account_id)
            .cmp(&(b.person.name.to_lowercase(), &b.person.account_id))
    });
    Ok(TeamMatrix { spaces, people })
}

/// Who is on the team of any of the projects, sorted by name, and me first: I may always be picked, on a team or not.
pub fn active(
    conn: &Connection,
    project_ids: &[String],
    me: Option<&Me>,
) -> AppResult<Vec<Person>> {
    let me_id = me.map(|m| m.account_id.as_str());
    let mut named: HashMap<String, String> = HashMap::new();
    for m in memberships(conn, project_ids)? {
        if m.cell.active && Some(m.account_id.as_str()) != me_id {
            named.insert(m.account_id, m.name);
        }
    }
    let mut people: Vec<Person> = named
        .iter()
        .map(|(id, name)| person(id, name, me_id))
        .collect();
    people.sort_by(|a, b| {
        (a.name.to_lowercase(), &a.account_id).cmp(&(b.name.to_lowercase(), &b.account_id))
    });
    if let Some(me) = me {
        people.insert(0, person(&me.account_id, &my_name(conn, me)?, me_id));
    }
    Ok(people)
}

/// The name the cache knows me by, as other rows show it; the connection's name before the first sync.
fn my_name(conn: &Connection, me: &Me) -> AppResult<String> {
    let cached: Option<String> = conn
        .query_row(
            "SELECT name FROM cache.user WHERE account_id = ?1",
            [&me.account_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(cached.unwrap_or_else(|| me.name.clone()))
}

/// The person chip's choices: me, then the active teammates of the level's Space, or of every tracked Space for a
/// level outside one.
pub async fn level_people(db: &Db, list_id: i64, me: Option<Me>) -> AppResult<Vec<Person>> {
    db.read(move |conn| {
        let projects = match project_of_level(conn, list_id)? {
            Some(project) => vec![project],
            None => tracked_projects(conn)?,
        };
        active(conn, &projects, me.as_ref())
    })
    .await
}

pub async fn team_matrix_of(db: &Db, me: Option<String>) -> AppResult<TeamMatrix> {
    db.read(move |conn| team_matrix(conn, me.as_deref())).await
}

/// Puts a person on a Space's team or takes them off; returns the grid. A choice that matches the default (on for
/// someone ever assigned there) is forgotten, so the default keeps following the issues.
pub async fn space_set_member(
    db: &Db,
    project_id: String,
    account_id: String,
    active: bool,
    me: Option<String>,
) -> AppResult<TeamMatrix> {
    db.write(move |tx, changes| {
        if !tracked_projects(tx)?.contains(&project_id) {
            return Err(AppError::NotFound("That Space isn't tracked".into()));
        }
        let known: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM cache.user WHERE account_id = ?1)",
            [&account_id],
            |r| r.get(0),
        )?;
        if !known {
            return Err(AppError::NotFound(
                "That person isn't known from the synced issues".into(),
            ));
        }
        let assignee = memberships(tx, std::slice::from_ref(&project_id))?
            .iter()
            .any(|m| m.account_id == account_id && m.cell.assignee);
        if active == assignee {
            tx.execute(
                "DELETE FROM space_member WHERE project_id = ?1 AND account_id = ?2",
                params![project_id, account_id],
            )?;
        } else {
            tx.execute(
                "INSERT INTO space_member (project_id, account_id, active) VALUES (?1, ?2, ?3)
                 ON CONFLICT (project_id, account_id) DO UPDATE SET active = excluded.active",
                params![project_id, account_id, active],
            )?;
        }
        changes.local_changed(LocalPart::Spaces);
        team_matrix(tx, me.as_deref())
    })
    .await
}

#[cfg(test)]
mod tests;
