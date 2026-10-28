//! An issue as the peek and the Issue view show it, resolved and formatted here like a list row.

use std::collections::HashMap;

use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use super::{status_names, user_names};
use crate::changes::{Names, Since, since};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::lists::Viewer;
use crate::status_style::{Style, detect, styles};
use crate::sync::store::{forget_issue, shown_fields};
use crate::views::columns::custom_columns;
use crate::views::rows::{self, Cell, EpicChip, Label, Person, Priority};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct IssueDetail {
    pub id: String,
    pub key: String,
    /// Jira's type name as plain text ("Story", "Bug", "Epic").
    pub issue_type: String,
    pub summary: String,
    pub style: Style,
    pub status_name: String,
    pub epic: Option<EpicChip>,
    /// Fix versions by name, in Jira's order.
    pub releases: Vec<String>,
    pub priority: Option<Priority>,
    pub assignees: Vec<Person>,
    pub reporter: Option<Person>,
    pub sprint: Option<String>,
    pub labels: Vec<Label>,
    /// As the list shows it ("Oct 30").
    pub due: Option<String>,
    /// UTC, RFC 3339.
    pub created_at: String,
    pub updated_at: String,
    /// Jira's rendered HTML, untrusted: the page sanitises it before showing it.
    pub description_html: Option<String>,
    pub comment_count: i64,
    pub is_epic: bool,
    /// Deleted in Jira or no longer visible.
    pub gone: bool,
    /// When the sync last stored this issue (UTC, RFC 3339): the gone notice's "Last synced".
    pub synced_at: String,
    /// The custom fields the owner shows (marked shown or used as a view column), with their value if any.
    pub custom: Vec<CustomField>,
    /// The banner over an issue with unseen changes.
    pub since: Option<Since>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct CustomField {
    pub id: String,
    pub name: String,
    pub value: Option<Cell>,
}

pub async fn issue_get(db: &Db, viewer: Viewer, id: String) -> AppResult<IssueDetail> {
    db.read(move |conn| read(conn, &viewer, &id)).await
}

const SELECT: &str = "SELECT i.id, i.key, i.issue_type, i.summary, i.status_id, COALESCE(st.name, ''),
    i.status_category, e.id, e.summary, e.epic_color, i.fix_versions, i.priority, i.assignee_id, a.name,
    i.reporter_id, r.name, i.sprint, i.labels, i.due_date, i.created_at, i.updated_at, i.description_html,
    i.comment_count, i.is_epic, i.gone, i.raw, i.synced_at
    FROM cache.issue i
    LEFT JOIN cache.status st ON st.id = i.status_id
    LEFT JOIN cache.user a ON a.account_id = i.assignee_id
    LEFT JOIN cache.user r ON r.account_id = i.reporter_id
    LEFT JOIN cache.issue e ON e.id = i.parent_id AND e.is_epic = 1
    WHERE i.id = ?1";

fn read(conn: &Connection, viewer: &Viewer, id: &str) -> AppResult<IssueDetail> {
    let ctx = viewer.ctx(conn)?;
    let me = ctx.me.as_deref();
    let styles = styles(conn)?;
    let versions = version_names(conn)?;
    let person = |id: Option<String>, name: Option<String>| {
        id.map(|id| rows::person(&id, name.as_deref().unwrap_or(&id), me))
    };
    conn.query_row(SELECT, [id], |r| {
        let status_id: String = r.get(4)?;
        let status_name: String = r.get(5)?;
        let category: String = r.get(6)?;
        let epic = match (
            r.get::<_, Option<String>>(7)?,
            r.get::<_, Option<String>>(8)?,
        ) {
            (Some(id), Some(name)) => Some(EpicChip {
                color: rows::epic_color(r.get::<_, Option<String>>(9)?.as_deref(), &id),
                id,
                name,
            }),
            _ => None,
        };
        let detail = IssueDetail {
            id: r.get(0)?,
            key: r.get(1)?,
            issue_type: r.get(2)?,
            summary: r.get(3)?,
            style: styles
                .get(&status_id)
                .copied()
                .unwrap_or_else(|| detect(&category, &status_name)),
            status_name,
            epic,
            releases: string_list(r.get(10)?)
                .iter()
                .filter_map(|id| versions.get(id).cloned())
                .collect(),
            priority: r
                .get::<_, Option<String>>(11)?
                .as_deref()
                .map(rows::priority),
            assignees: person(r.get(12)?, r.get(13)?).into_iter().collect(),
            reporter: person(r.get(14)?, r.get(15)?),
            sprint: r.get(16)?,
            labels: string_list(r.get(17)?)
                .iter()
                .map(|name| rows::label(name))
                .collect(),
            due: r
                .get::<_, Option<String>>(18)?
                .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                .map(|d| rows::format_due(d, ctx.today)),
            created_at: r.get(19)?,
            updated_at: r.get(20)?,
            description_html: r.get(21)?,
            comment_count: r.get(22)?,
            is_epic: r.get::<_, i64>(23)? != 0,
            gone: r.get::<_, i64>(24)? != 0,
            synced_at: r.get(26)?,
            custom: Vec::new(),
            since: None,
        };
        let raw: String = r.get(25)?;
        Ok((detail, raw))
    })
    .optional()?
    .map(|(mut detail, raw)| -> AppResult<IssueDetail> {
        detail.custom = custom_fields(conn, &raw, me)?;
        detail.since = since_banner(conn, viewer, id)?;
        Ok(detail)
    })
    .transpose()?
    .ok_or_else(|| AppError::NotFound(format!("Issue {id} isn't in the cache")))
}

fn since_banner(conn: &Connection, viewer: &Viewer, id: &str) -> AppResult<Option<Since>> {
    let statuses = status_names(conn)?
        .into_iter()
        .map(|(id, (name, _))| (id, name))
        .collect();
    let names = Names {
        me: viewer.me.as_deref(),
        statuses: &statuses,
        people: &user_names(conn)?,
    };
    since(conn, viewer, id, &names)
}

fn custom_fields(conn: &Connection, raw: &str, me: Option<&str>) -> AppResult<Vec<CustomField>> {
    let raw: serde_json::Value = serde_json::from_str(raw).unwrap_or_default();
    let mut names = conn.prepare("SELECT name FROM cache.field WHERE id = ?1")?;
    let mut fields = Vec::new();
    for column in custom_columns(conn, &shown_fields(conn)?)? {
        let name: String = names
            .query_row([&column.id], |r| r.get(0))
            .optional()?
            .unwrap_or_else(|| column.id.clone());
        fields.push(CustomField {
            value: rows::cell(&column.kind, &raw["fields"][&column.id], me),
            name,
            id: column.id,
        });
    }
    Ok(fields)
}

fn string_list(json: Option<String>) -> Vec<String> {
    json.and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default()
}

fn version_names(conn: &Connection) -> AppResult<HashMap<String, String>> {
    let mut query = conn.prepare("SELECT id, name FROM cache.version")?;
    let rows = query.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Remove from cache: only a gone issue can go (a live one would come back with the next sync).
pub async fn issue_forget(db: &Db, id: String) -> AppResult<()> {
    db.write(move |tx, changes| {
        let gone: Option<bool> = tx
            .query_row("SELECT gone FROM cache.issue WHERE id = ?1", [&id], |r| {
                r.get(0)
            })
            .optional()?;
        match gone {
            None => Err(AppError::NotFound(format!("Issue {id} isn't in the cache"))),
            Some(false) => Err(AppError::InvalidInput(
                "Only an issue that is gone from Jira can be removed from the cache".into(),
            )),
            Some(true) => {
                forget_issue(tx, &id)?;
                changes.issue_changed(id.as_str());
                Ok(())
            }
        }
    })
    .await
}
