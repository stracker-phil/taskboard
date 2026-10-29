//! An issue as the peek and the Issue view show it, resolved and formatted here like a list row.

use std::collections::HashMap;

use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use super::attachments::{Attachment, attachments_of};
use super::{status_names, user_names};
use crate::changes::{Names, Since, since, unseen_since};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::is_filled;
use crate::lists::Viewer;
use crate::space_config::{Place, hidden_fields};
use crate::status_style::{Style, Styles};
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
    /// Jira's rendered HTML, untrusted: the page sanitizes it before showing it.
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
    /// Unseen changes, or marked unread by the owner.
    pub unread: bool,
    /// Files attached in Jira, in Jira's order.
    pub attachments: Vec<Attachment>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct CustomField {
    pub id: String,
    pub name: String,
    pub value: Option<Cell>,
    /// Jira's rendered HTML for a rich text field (acceptance criteria, steps to reproduce); `value` is then empty.
    pub html: Option<String>,
}

pub async fn issue_get(db: &Db, viewer: Viewer, id: String) -> AppResult<IssueDetail> {
    db.read(move |conn| read(conn, &viewer, &id)).await
}

const SELECT: &str = "SELECT i.id, i.key, i.issue_type, i.summary, i.status_id, COALESCE(st.name, ''),
    i.status_category, e.id, e.summary, e.epic_color, i.fix_versions, i.priority, i.assignee_id, a.name,
    i.reporter_id, r.name, i.sprint, i.labels, i.due_date, i.created_at, i.updated_at, i.description_html,
    i.comment_count, i.is_epic, i.gone, i.raw, i.synced_at, i.project_id
    FROM cache.issue i
    LEFT JOIN cache.status st ON st.id = i.status_id
    LEFT JOIN cache.user a ON a.account_id = i.assignee_id
    LEFT JOIN cache.user r ON r.account_id = i.reporter_id
    LEFT JOIN cache.issue e ON e.id = i.parent_id AND e.is_epic = 1
    WHERE i.id = ?1";

fn read(conn: &Connection, viewer: &Viewer, id: &str) -> AppResult<IssueDetail> {
    let ctx = viewer.ctx(conn)?;
    let me = ctx.me.as_deref();
    let styles = Styles::load(conn)?;
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
            style: styles.of(
                &r.get::<_, String>(27)?,
                &status_id,
                &category,
                &status_name,
            ),
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
            unread: false,
            attachments: Vec::new(),
        };
        let raw: String = r.get(25)?;
        let project_id: String = r.get(27)?;
        Ok((detail, raw, project_id))
    })
    .optional()?
    .map(|(mut detail, raw, project_id)| -> AppResult<IssueDetail> {
        detail.attachments = attachments_of(&raw);
        detail.custom = custom_fields(conn, &project_id, &raw, me)?;
        detail.since = since_banner(conn, viewer, id)?;
        detail.unread = unseen_since(conn, id)?.is_some();
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

/// The custom fields the owner shows (always, empty or not), then every other one the issue fills, by name;
/// none its Space hides from the Issue view.
fn custom_fields(
    conn: &Connection,
    project_id: &str,
    raw: &str,
    me: Option<&str>,
) -> AppResult<Vec<CustomField>> {
    let raw: serde_json::Value = serde_json::from_str(raw).unwrap_or_default();
    let hidden = hidden_fields(conn, project_id, Place::Detail)?;
    let mut shown = shown_fields(conn)?;
    shown.retain(|id| !hidden.contains(id));
    let filled: Vec<String> = raw["fields"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(id, value)| {
            id.starts_with("customfield_")
                && !shown.contains(id)
                && !hidden.contains(id.as_str())
                && is_filled(value)
        })
        .map(|(id, _)| id.clone())
        .collect();
    let mut names = conn.prepare("SELECT name FROM cache.field WHERE id = ?1")?;
    let mut field = |id: String, kind: &str| -> AppResult<CustomField> {
        let name: String = names
            .query_row([&id], |r| r.get(0))
            .optional()?
            .unwrap_or_else(|| id.clone());
        let value = &raw["fields"][&id];
        // Rich text arrives as an ADF document; Jira's rendering of it is in `renderedFields`.
        let html = (value["type"] == "doc")
            .then(|| raw["renderedFields"][&id].as_str())
            .flatten()
            .filter(|html| !html.trim().is_empty())
            .map(str::to_string);
        Ok(CustomField {
            value: if html.is_some() || value["type"] == "doc" {
                None
            } else {
                rows::cell(kind, value, me)
            },
            html,
            name,
            id,
        })
    };
    let mut fields = Vec::new();
    for column in custom_columns(conn, &shown)? {
        fields.push(field(column.id, &column.kind)?);
    }
    let mut extra = Vec::new();
    for column in custom_columns(conn, &filled)? {
        let found = field(column.id, &column.kind)?;
        if found.value.is_some() || found.html.is_some() {
            extra.push(found);
        }
    }
    extra.sort_by_key(|f| f.name.to_lowercase());
    fields.extend(extra);
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

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use tempfile::TempDir;

    use super::*;
    use crate::test_support::temp_config_dir;

    const ZONE: &str = "customfield_10001";
    const ALPHA: &str = "customfield_10002";
    const NOTES: &str = "customfield_10003";
    const RANK: &str = "customfield_10019";
    const SPRINT: &str = "customfield_10020";
    const PROJECT: &str = "10001";

    async fn lab() -> (TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| {
            tx.execute_batch(
                "INSERT INTO cache.field (id, name, type) VALUES
                   ('summary', 'Summary', 'string'),
                   ('customfield_10001', 'Zone', 'string'),
                   ('customfield_10002', 'alpha', 'string'),
                   ('customfield_10003', 'Notes', 'string'),
                   ('customfield_10019', 'Rank', 'any'),
                   ('customfield_10020', 'Sprint', 'array')",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        (dir, db)
    }

    async fn show(db: &Db, id: &'static str) {
        db.write(move |tx, _| {
            tx.execute("UPDATE cache.field SET shown = 1 WHERE id = ?1", [id])?;
            Ok(())
        })
        .await
        .unwrap();
    }

    async fn custom(db: &Db, raw: Value) -> Vec<CustomField> {
        db.read(move |conn| custom_fields(conn, PROJECT, &raw.to_string(), None))
            .await
            .unwrap()
    }

    fn text(id: &str, name: &str, value: &str) -> CustomField {
        CustomField {
            id: id.into(),
            name: name.into(),
            value: Some(Cell::Text(value.into())),
            html: None,
        }
    }

    fn empty(id: &str, name: &str) -> CustomField {
        CustomField {
            id: id.into(),
            name: name.into(),
            value: None,
            html: None,
        }
    }

    fn doc() -> Value {
        json!({ "type": "doc", "version": 1, "content": [] })
    }

    #[tokio::test]
    async fn an_unshown_field_the_issue_fills_is_listed() {
        let (_dir, db) = lab().await;

        let found = custom(&db, json!({ "fields": { ZONE: "North" } })).await;

        assert_eq!(found, [text(ZONE, "Zone", "North")]);
    }

    #[tokio::test]
    async fn an_unshown_field_the_issue_leaves_empty_is_not_listed() {
        let (_dir, db) = lab().await;

        let found = custom(
            &db,
            json!({ "fields": { ZONE: null, ALPHA: "", NOTES: [] } }),
        )
        .await;

        assert_eq!(found, []);
    }

    #[tokio::test]
    async fn a_shown_field_is_listed_even_when_the_issue_leaves_it_empty() {
        let (_dir, db) = lab().await;
        show(&db, ZONE).await;

        let found = custom(&db, json!({ "fields": { ZONE: null } })).await;

        assert_eq!(found, [empty(ZONE, "Zone")]);
    }

    #[tokio::test]
    async fn a_rich_text_field_gives_its_rendered_html_and_no_value() {
        let (_dir, db) = lab().await;
        let raw = json!({
            "fields": { NOTES: doc() },
            "renderedFields": { NOTES: "<p>Check the totals</p>" },
        });

        let found = custom(&db, raw).await;

        assert_eq!(
            found,
            [CustomField {
                id: NOTES.into(),
                name: "Notes".into(),
                value: None,
                html: Some("<p>Check the totals</p>".into()),
            }]
        );
    }

    #[tokio::test]
    async fn an_unshown_rich_text_field_with_blank_html_is_left_out() {
        let (_dir, db) = lab().await;

        for rendered in [json!({ NOTES: "  " }), json!({})] {
            let raw = json!({ "fields": { NOTES: doc() }, "renderedFields": rendered });

            assert_eq!(custom(&db, raw.clone()).await, [], "{raw}");
        }
    }

    #[tokio::test]
    async fn a_shown_rich_text_field_with_blank_html_is_listed_without_value_or_html() {
        let (_dir, db) = lab().await;
        show(&db, NOTES).await;
        let raw = json!({ "fields": { NOTES: doc() }, "renderedFields": { NOTES: "" } });

        let found = custom(&db, raw).await;

        assert_eq!(found, [empty(NOTES, "Notes")]);
    }

    #[tokio::test]
    async fn the_shown_fields_come_first_then_the_others_by_name_ignoring_case() {
        let (_dir, db) = lab().await;
        show(&db, ZONE).await;
        let raw = json!({ "fields": { ZONE: "North", NOTES: "Late", ALPHA: "First" } });

        let found = custom(&db, raw).await;

        assert_eq!(
            found,
            [
                text(ZONE, "Zone", "North"),
                text(ALPHA, "alpha", "First"),
                text(NOTES, "Notes", "Late"),
            ]
        );
    }

    #[tokio::test]
    async fn internal_fields_the_sprint_field_and_system_fields_stay_out() {
        let (_dir, db) = lab().await;
        let raw = json!({ "fields": {
            RANK: "0|i0001:",
            SPRINT: [{ "name": "Sprint 22" }],
            "summary": "Checkout",
        } });

        assert_eq!(custom(&db, raw).await, []);
    }

    async fn set_visibility(
        db: &Db,
        project: &'static str,
        field: &'static str,
        visibility: &'static str,
    ) {
        db.write(move |tx, _| {
            tx.execute(
                "INSERT INTO space_field (project_id, field_id, visibility) VALUES (?1, ?2, ?3)",
                [project, field, visibility],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn fields_the_space_hides_from_the_detail_are_left_out_filled_or_shown() {
        for (visibility, listed) in [
            ("hiddenInDetail", false),
            ("hidden", false),
            ("hiddenInList", true),
        ] {
            let (_dir, db) = lab().await;
            show(&db, ZONE).await;
            set_visibility(&db, PROJECT, ZONE, visibility).await;
            set_visibility(&db, PROJECT, NOTES, visibility).await;
            let raw = json!({ "fields": { ZONE: null, NOTES: "Late", ALPHA: "First" } });

            let found = custom(&db, raw).await;

            let expected = if listed {
                vec![
                    empty(ZONE, "Zone"),
                    text(ALPHA, "alpha", "First"),
                    text(NOTES, "Notes", "Late"),
                ]
            } else {
                vec![text(ALPHA, "alpha", "First")]
            };
            assert_eq!(found, expected, "{visibility}");
        }
    }

    #[tokio::test]
    async fn another_spaces_hidden_fields_do_not_affect_the_issue() {
        let (_dir, db) = lab().await;
        set_visibility(&db, "10002", NOTES, "hidden").await;

        let found = custom(&db, json!({ "fields": { NOTES: "Late" } })).await;

        assert_eq!(found, [text(NOTES, "Notes", "Late")]);
    }
}
