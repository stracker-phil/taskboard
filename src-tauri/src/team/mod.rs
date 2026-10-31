//! A Space's team: everyone with activity in its synced issues, minus the people switched off
//! (`space_member_off`).

pub mod commands;

use rusqlite::{Connection, params};
use serde::Serialize;
use ts_rs::TS;

use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::space_config::project_of_level;
use crate::views::rows::{Person, person};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TeamMember {
    pub person: Person,
    pub active: bool,
    /// Open issues of the Space assigned to them.
    pub assigned: u32,
}

/// Assignees, reporters, comment authors and history authors of the projects' issues.
const PEOPLE: &str = "
    WITH scoped AS (SELECT id, assignee_id, reporter_id FROM cache.issue
                    WHERE gone = 0 AND project_id IN (SELECT value FROM json_each(?1))),
    seen AS (
        SELECT assignee_id AS account_id FROM scoped
        UNION SELECT reporter_id FROM scoped
        UNION SELECT c.author_id FROM cache.comment c JOIN scoped s ON s.id = c.issue_id
        UNION SELECT h.author_id FROM cache.issue_history h JOIN scoped s ON s.id = h.issue_id
    )
    SELECT u.account_id, u.name FROM seen JOIN cache.user u ON u.account_id = seen.account_id";

/// The Space's team sorted by name, the switched-off people included.
pub fn team(conn: &Connection, project_id: &str, me: Option<&str>) -> AppResult<Vec<TeamMember>> {
    let mut query = conn.prepare(&format!(
        "SELECT p.account_id, p.name,
                NOT EXISTS (SELECT 1 FROM space_member_off o WHERE o.project_id = ?2 AND o.account_id = p.account_id),
                (SELECT COUNT(*) FROM cache.issue i WHERE i.project_id = ?2 AND i.gone = 0
                   AND i.status_category != 'done' AND i.assignee_id = p.account_id)
         FROM ({PEOPLE}) p ORDER BY lower(p.name), p.account_id"
    ))?;
    let projects = serde_json::to_string(&[project_id])?;
    let rows = query.query_map(params![projects, project_id], |r| {
        Ok(TeamMember {
            person: person(&r.get::<_, String>(0)?, &r.get::<_, String>(1)?, me),
            active: r.get(2)?,
            assigned: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Who is active in any of the projects' teams, sorted by name; `me` comes first whenever it is among them.
pub fn active(
    conn: &Connection,
    project_ids: &[String],
    me: Option<&str>,
) -> AppResult<Vec<Person>> {
    let mut query = conn.prepare(&format!(
        "SELECT p.account_id, p.name FROM ({PEOPLE}) p
         WHERE EXISTS (SELECT 1 FROM json_each(?1) j WHERE NOT EXISTS (
                 SELECT 1 FROM space_member_off o WHERE o.project_id = j.value AND o.account_id = p.account_id))
         ORDER BY lower(p.name), p.account_id"
    ))?;
    let projects = serde_json::to_string(project_ids)?;
    let rows = query.query_map([projects], |r| {
        Ok(person(&r.get::<_, String>(0)?, &r.get::<_, String>(1)?, me))
    })?;
    let mut people: Vec<Person> = rows.collect::<Result<_, _>>()?;
    if let Some(at) = people
        .iter()
        .position(|p| Some(p.account_id.as_str()) == me)
    {
        let mine = people.remove(at);
        people.insert(0, mine);
    }
    Ok(people)
}

/// The active teammates of the level's Space, or of every tracked Space for a level outside one; without me.
pub async fn level_people(db: &Db, list_id: i64, me: Option<String>) -> AppResult<Vec<Person>> {
    db.read(move |conn| {
        let projects = match project_of_level(conn, list_id)? {
            Some(project) => vec![project],
            None => tracked_projects(conn)?,
        };
        let mut people = active(conn, &projects, me.as_deref())?;
        people.retain(|p| Some(p.account_id.as_str()) != me.as_deref());
        Ok(people)
    })
    .await
}

fn tracked_projects(conn: &Connection) -> AppResult<Vec<String>> {
    let mut query = conn.prepare("SELECT project_id FROM space WHERE tracked = 1")?;
    let rows = query.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub async fn space_team(
    db: &Db,
    project_id: String,
    me: Option<String>,
) -> AppResult<Vec<TeamMember>> {
    db.read(move |conn| team(conn, &project_id, me.as_deref()))
        .await
}

/// Switches a person on or off in the Space's team; returns the team.
pub async fn space_set_member(
    db: &Db,
    project_id: String,
    account_id: String,
    active: bool,
    me: Option<String>,
) -> AppResult<Vec<TeamMember>> {
    db.write(move |tx, changes| {
        let members = team(tx, &project_id, me.as_deref())?;
        if !members.iter().any(|m| m.person.account_id == account_id) {
            return Err(AppError::NotFound(
                "That person has no activity in this Space".into(),
            ));
        }
        if active {
            tx.execute(
                "DELETE FROM space_member_off WHERE project_id = ?1 AND account_id = ?2",
                params![project_id, account_id],
            )?;
        } else {
            tx.execute(
                "INSERT INTO space_member_off (project_id, account_id) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
                params![project_id, account_id],
            )?;
        }
        changes.local_changed(LocalPart::Spaces);
        team(tx, &project_id, me.as_deref())
    })
    .await
}

#[cfg(test)]
mod tests;
