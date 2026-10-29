//! A view's issues as rows: one query over the cache, each result turned into a `Row` and its grouping facts.

use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDate;
use rusqlite::Connection;
use rusqlite::params_from_iter;
use rusqlite::types::Value as SqlValue;
use serde_json::Value as Json;

use super::columns::CustomColumn;
use super::groups::{Facts, Version};
use super::rows::{self, Row};
use crate::error::AppResult;
use crate::filter::Ctx;
use crate::status_style::Styles;

/// What turning a result row into a `Row` looks up.
pub(super) struct Source<'a> {
    pub ctx: &'a Ctx,
    pub styles: Styles,
    pub versions: &'a HashMap<String, Version>,
    pub custom: Vec<CustomColumn>,
}

/// The columns `row_from` reads, by position; the custom fields' values follow from `CUSTOM_AT`.
const SELECT: &str = "i.id, i.key, i.summary, i.status_id, COALESCE(st.name, ''), i.status_category,
    i.comment_count, i.assignee_id, u.name, e.id, e.summary, e.epic_color, i.fix_versions, i.priority,
    i.labels, i.sprint, i.due_date, i.updated_at, i.created_at, i.project_id, i.type_is_subtask,
    CASE WHEN i.type_is_subtask = 1 THEN i.parent_id END,
    (SELECT COUNT(*) FROM cache.issue s WHERE s.parent_id = i.id AND s.type_is_subtask = 1 AND s.gone = 0)";
const CUSTOM_AT: usize = 23;

pub(super) fn fetch(
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
        style: source.styles.of(
            &r.get::<_, String>(19)?,
            &status_id,
            &category,
            &status_name,
        ),
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
        is_subtask: r.get(20)?,
        parent_id: r.get(21)?,
        subtask_count: r.get(22)?,
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
