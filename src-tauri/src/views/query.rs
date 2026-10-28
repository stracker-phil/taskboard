//! A view's issues: the level's filter plus the toolbar's narrowing, sorted, grouped and shaped as rows
//! (ADR 0024). One query per call; grouping and sorting happen here, not in the page.

use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDate;
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params_from_iter};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use ts_rs::TS;

use super::columns::{CustomColumn, custom_columns};
use super::edit::read_view;
use super::groups::{Facts, Group, Lookup, SpaceInfo, Version, group};
use super::model::GroupBy;
use super::rows::{self, Row};
use super::sorting::{fold, sort};
use crate::changes::{Names, UNSEEN, chip, facts, unseen_changes};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::filter::{Ctx, compile};
use crate::issues::status_names;
use crate::lists::{Kind, Viewer, all_nodes, scope_of};
use crate::status_style::{Style, detect, styles};

/// The toolbar's quick narrowing on top of the level's filter.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct Narrowing {
    /// Only issues assigned to the connected account.
    pub me: bool,
    /// Key or summary contains it, ignoring case and accents.
    pub text: String,
    /// Only children of this epic.
    pub epic: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ViewData {
    pub group_by: GroupBy,
    pub groups: Vec<Group>,
    pub rows: HashMap<String, Row>,
    pub columns: Vec<String>,
}

pub async fn view_rows(
    db: &Db,
    viewer: Viewer,
    list_id: i64,
    view_id: i64,
    narrowing: Narrowing,
) -> AppResult<ViewData> {
    db.read(move |conn| build(conn, &viewer, list_id, view_id, &narrowing))
        .await
}

fn build(
    conn: &Connection,
    viewer: &Viewer,
    list_id: i64,
    view_id: i64,
    narrowing: &Narrowing,
) -> AppResult<ViewData> {
    let view = read_view(conn, view_id)?;
    if view.list_id != list_id {
        return Err(AppError::NotFound(format!(
            "View {view_id} isn't a view of filter list {list_id}"
        )));
    }
    let ctx = viewer.ctx(conn)?;
    let versions = versions(conn)?;
    let lookup = Lookup {
        me: ctx.me.as_deref(),
        today: ctx.today,
        spaces: &spaces(conn)?,
        versions: &versions,
    };
    let source = Source {
        ctx: &ctx,
        styles: styles(conn)?,
        versions: &versions,
        custom: custom_columns(conn, &view.columns)?,
    };
    let (condition, params) = conditions(conn, &ctx, list_id, narrowing)?;
    let mut items = fetch(conn, &source, &condition, params)?;
    mark_changed(conn, ctx.me.as_deref(), &mut items)?;
    let needle = fold(narrowing.text.trim());
    if !needle.is_empty() {
        items.retain(|(row, _)| {
            fold(&row.key).contains(&needle) || fold(&row.summary).contains(&needle)
        });
    }
    sort(&mut items, view.sort.by, view.sort.descending);
    let groups = group(&items, view.group_by, &lookup);
    Ok(ViewData {
        group_by: view.group_by,
        groups,
        rows: items
            .into_iter()
            .map(|(row, _)| (row.id.clone(), row))
            .collect(),
        columns: view.columns,
    })
}

/// Rows with unseen changes are highlighted and say what changed.
fn mark_changed(conn: &Connection, me: Option<&str>, items: &mut [(Row, Facts)]) -> AppResult<()> {
    let unseen = unseen_changes(conn)?;
    if unseen.is_empty() {
        return Ok(());
    }
    let statuses: HashMap<String, String> = status_names(conn)?
        .into_iter()
        .map(|(id, (name, _))| (id, name))
        .collect();
    let names = Names {
        me,
        statuses: &statuses,
        people: &HashMap::new(),
    };
    for (row, _) in items {
        if let Some(changes) = unseen.get(&row.id) {
            row.changed = true;
            row.change_note = chip(&facts(changes), &names);
        }
    }
    Ok(())
}

/// The filter list's issues with unseen changes, for Mark all seen.
pub fn unseen_ids(conn: &Connection, viewer: &Viewer, list_id: i64) -> AppResult<Vec<String>> {
    let ctx = viewer.ctx(conn)?;
    let (condition, params) = conditions(conn, &ctx, list_id, &Narrowing::default())?;
    let mut query = conn.prepare(&format!(
        "SELECT i.id FROM cache.issue i WHERE {condition} AND {UNSEEN}"
    ))?;
    let ids = query.query_map(params_from_iter(params), |r| r.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
}

/// The level's conditions with its kind's own rule and the narrowing, as one SQL condition.
fn conditions(
    conn: &Connection,
    ctx: &Ctx,
    list_id: i64,
    narrowing: &Narrowing,
) -> AppResult<(String, Vec<SqlValue>)> {
    let nodes = all_nodes(conn)?;
    let node = nodes
        .iter()
        .find(|n| n.id == list_id)
        .ok_or_else(|| AppError::NotFound(format!("Filter list {list_id} doesn't exist")))?;
    let compiled = compile(&scope_of(&nodes, node).all()?, ctx)?;
    let mut condition = compiled.where_clause();
    let mut params = compiled.params;
    match node.kind {
        Kind::Recent => {
            condition.push_str(" AND ");
            condition.push_str(UNSEEN);
        }
        Kind::Pinned => condition.push_str(
            " AND i.id IN (SELECT target_id FROM pin WHERE target_kind = 'issue' AND unpinned_at IS NULL)",
        ),
        _ => {}
    }
    if narrowing.me {
        condition.push_str(" AND i.assignee_id = ?");
        params.push(ctx.me.clone().unwrap_or_default().into());
    }
    if let Some(epic) = &narrowing.epic {
        condition.push_str(" AND i.parent_id = ?");
        params.push(epic.clone().into());
    }
    Ok((condition, params))
}

/// What turning a result row into a `Row` looks up.
struct Source<'a> {
    ctx: &'a Ctx,
    styles: HashMap<String, Style>,
    versions: &'a HashMap<String, Version>,
    custom: Vec<CustomColumn>,
}

/// The columns `row_from` reads, by position; the custom fields' values follow from `CUSTOM_AT`.
const SELECT: &str = "i.id, i.key, i.summary, i.status_id, COALESCE(st.name, ''), i.status_category,
    i.comment_count, i.assignee_id, u.name, e.id, e.summary, e.epic_color, i.fix_versions, i.priority,
    i.labels, i.sprint, i.due_date, i.updated_at, i.created_at, i.project_id";
const CUSTOM_AT: usize = 20;

fn fetch(
    conn: &Connection,
    source: &Source,
    condition: &str,
    params: Vec<SqlValue>,
) -> AppResult<Vec<(Row, Facts)>> {
    let custom_sql: String = source
        .custom
        .iter()
        .map(|_| ", json_extract(i.raw, ?)")
        .collect();
    // The custom fields' paths come first in the statement, so their parameters go first too.
    let paths = source
        .custom
        .iter()
        .map(|c| SqlValue::Text(format!("$.fields.{}", c.id)));
    let mut query = conn.prepare(&format!(
        "SELECT {SELECT} {custom_sql}
         FROM cache.issue i
         LEFT JOIN cache.status st ON st.id = i.status_id
         LEFT JOIN cache.user u ON u.account_id = i.assignee_id
         LEFT JOIN cache.issue e ON e.id = i.parent_id AND e.is_epic = 1
         WHERE {condition}"
    ))?;
    let mut rows = query.query(params_from_iter(paths.chain(params)))?;
    let mut items = Vec::new();
    while let Some(r) = rows.next()? {
        items.push(row_from(r, source)?);
    }
    Ok(items)
}

fn row_from(r: &rusqlite::Row, source: &Source) -> AppResult<(Row, Facts)> {
    let me = source.ctx.me.as_deref();
    let status_id: String = r.get(3)?;
    let status_name: String = r.get(4)?;
    let category: String = r.get(5)?;
    let assignee_id: Option<String> = r.get(7)?;
    let assignee_name: Option<String> = r.get(8)?;
    let release_id = string_list(r.get::<_, Option<String>>(12)?.as_deref())
        .into_iter()
        .next();
    let due_date = r
        .get::<_, Option<String>>(16)?
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());
    let epic = match (
        r.get::<_, Option<String>>(9)?,
        r.get::<_, Option<String>>(10)?,
    ) {
        (Some(id), Some(name)) => Some(rows::EpicChip {
            color: rows::epic_color(r.get::<_, Option<String>>(11)?.as_deref(), &id),
            id,
            name,
        }),
        _ => None,
    };
    let row = Row {
        id: r.get(0)?,
        key: r.get(1)?,
        summary: r.get(2)?,
        style: source
            .styles
            .get(&status_id)
            .copied()
            .unwrap_or_else(|| detect(&category, &status_name)),
        status_name,
        comment_count: r.get(6)?,
        changed: false,
        change_note: None,
        assignees: assignee_id
            .as_deref()
            .map(|id| rows::person(id, assignee_name.as_deref().unwrap_or(id), me))
            .into_iter()
            .collect(),
        epic,
        release: release_id
            .as_ref()
            .and_then(|id| source.versions.get(id))
            .map(|v| v.name.clone()),
        priority: r
            .get::<_, Option<String>>(13)?
            .as_deref()
            .map(rows::priority),
        labels: string_list(r.get::<_, Option<String>>(14)?.as_deref())
            .iter()
            .map(|name| rows::label(name))
            .collect(),
        sprint: r.get(15)?,
        due: due_date.map(|d| rows::format_due(d, source.ctx.today)),
        custom: custom_cells(r, source)?,
    };
    let facts = Facts {
        status_id,
        project_id: r.get(19)?,
        assignee_id,
        release_id,
        due_date,
        updated_at: r.get(17)?,
        created_at: r.get(18)?,
    };
    Ok((row, facts))
}

fn custom_cells(r: &rusqlite::Row, source: &Source) -> AppResult<BTreeMap<String, rows::Cell>> {
    let mut cells = BTreeMap::new();
    for (index, column) in source.custom.iter().enumerate() {
        let value = json_of(r.get(CUSTOM_AT + index)?);
        if let Some(cell) = rows::cell(&column.kind, &value, source.ctx.me.as_deref()) {
            cells.insert(column.id.clone(), cell);
        }
    }
    Ok(cells)
}

fn json_of(value: SqlValue) -> Json {
    match value {
        SqlValue::Null | SqlValue::Blob(_) => Json::Null,
        SqlValue::Integer(n) => n.into(),
        SqlValue::Real(n) => n.into(),
        // json_extract returns objects and arrays as JSON text, strings as plain text.
        SqlValue::Text(text) => serde_json::from_str(&text)
            .ok()
            .filter(|v: &Json| v.is_object() || v.is_array())
            .unwrap_or(Json::String(text)),
    }
}

fn string_list(json: Option<&str>) -> Vec<String> {
    json.and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

/// The owner's icon letter, else the name's first letter.
fn space_letter(icon: Option<String>, name: &str) -> String {
    icon.filter(|i| !i.is_empty())
        .unwrap_or_else(|| name.chars().take(1).flat_map(char::to_uppercase).collect())
}

fn spaces(conn: &Connection) -> AppResult<HashMap<String, SpaceInfo>> {
    let mut query = conn.prepare(
        "SELECT s.project_id, COALESCE(s.name, p.name), s.icon, s.color, s.client
         FROM space s JOIN cache.project p ON p.id = s.project_id
         WHERE s.tracked = 1 ORDER BY s.position, p.key",
    )?;
    let rows = query.query_map([], |row| {
        let name: String = row.get(1)?;
        let icon: Option<String> = row.get(2)?;
        Ok((
            row.get::<_, String>(0)?,
            SpaceInfo {
                position: 0,
                letter: space_letter(icon, &name),
                name,
                color: row.get(3)?,
                client: row.get(4)?,
            },
        ))
    })?;
    let mut spaces = HashMap::new();
    for (position, row) in rows.enumerate() {
        let (project_id, space) = row?;
        spaces.insert(project_id, SpaceInfo { position, ..space });
    }
    Ok(spaces)
}

fn versions(conn: &Connection) -> AppResult<HashMap<String, Version>> {
    let mut query = conn.prepare("SELECT id, name, release_date FROM cache.version")?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get(0)?,
            Version {
                name: row.get(1)?,
                release_date: row.get(2)?,
            },
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}
