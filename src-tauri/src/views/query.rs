//! A view's issues: the level's filter (ADR 0024), the view's own conditions (ADR 0033) and the toolbar's
//! narrowing, sorted, grouped and shaped as rows. One query per call; grouping and sorting happen here, not in
//! the page.

use std::collections::HashMap;

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params_from_iter};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::columns::custom_columns;
use super::edit::read_view;
use super::fetch::{Source, fetch};
use super::filter::view_conditions;
use super::groups::{Facts, Group, Lookup, group};
use super::lookup::{spaces, versions};
use super::model::{GroupBy, SubtaskMode, View};
use super::rows::Row;
use super::sorting::{fold, sort};
use super::subtasks::{nest, subtasks_of, top_level_only};
use crate::changes::{Names, UNSEEN, chip, facts, unseen_changes};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::filter::{Ctx, compile};
use crate::issues::status_names;
use crate::lists::{Kind, Viewer, all_nodes, scope_of};
use crate::pins::active;
use crate::status_style::Styles;
use crate::sync::map::format_utc;

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
    /// Unread keeps these read issues (read since the owner came here); set by Rust, never by the page.
    #[serde(skip)]
    #[ts(skip)]
    pub read_here: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ViewData {
    pub group_by: GroupBy,
    pub subtasks: SubtaskMode,
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

/// The view and its issues: the level's filter, its kind's rule and the narrowing, highlighted and sorted.
pub(super) struct Fetched {
    pub view: View,
    /// The subtask mode the rows follow.
    pub mode: SubtaskMode,
    pub items: Vec<(Row, Facts)>,
    /// In Hierarchy, every subtask of the items, sorted; they never went through the filter.
    pub subtasks: Vec<(Row, Facts)>,
    pub ctx: Ctx,
}

/// `mode` overrides the view's subtask mode (the Board always hides subtasks).
pub(super) fn fetch_view(
    conn: &Connection,
    viewer: &Viewer,
    list_id: i64,
    view_id: i64,
    narrowing: &Narrowing,
    mode: Option<SubtaskMode>,
) -> AppResult<Fetched> {
    let view = read_view(conn, view_id)?;
    if view.list_id != list_id {
        return Err(AppError::NotFound(format!(
            "View {view_id} isn't a view of filter list {list_id}"
        )));
    }
    let ctx = viewer.ctx(conn)?;
    let versions = versions(conn)?;
    let source = Source {
        ctx: &ctx,
        styles: Styles::load(conn)?,
        versions: &versions,
        custom: custom_columns(conn, &view.columns)?,
    };
    let mode = mode.unwrap_or(view.subtasks);
    let (mut condition, params) = conditions(conn, viewer, &ctx, list_id, Some(&view), narrowing)?;
    if let Some(top_level) = top_level_only(mode) {
        condition.push_str(&format!(" AND {top_level}"));
    }
    let mut items = fetch(conn, &source, &condition, params)?;
    let needle = fold(narrowing.text.trim());
    if !needle.is_empty() {
        items.retain(|(row, _)| {
            fold(&row.key).contains(&needle) || fold(&row.summary).contains(&needle)
        });
    }
    let mut subtasks = match mode {
        SubtaskMode::Hierarchy => subtasks_of(conn, &source, &items)?,
        _ => Vec::new(),
    };
    mark_changed(conn, ctx.me.as_deref(), &mut items)?;
    mark_changed(conn, ctx.me.as_deref(), &mut subtasks)?;
    sort(&mut items, view.sort.by, view.sort.descending);
    sort(&mut subtasks, view.sort.by, view.sort.descending);
    Ok(Fetched {
        view,
        mode,
        items,
        subtasks,
        ctx,
    })
}

fn build(
    conn: &Connection,
    viewer: &Viewer,
    list_id: i64,
    view_id: i64,
    narrowing: &Narrowing,
) -> AppResult<ViewData> {
    let Fetched {
        view,
        mode,
        items,
        subtasks,
        ctx,
    } = fetch_view(conn, viewer, list_id, view_id, narrowing, None)?;
    let versions = versions(conn)?;
    let lookup = Lookup {
        me: ctx.me.as_deref(),
        today: ctx.today,
        spaces: &spaces(conn)?,
        versions: &versions,
    };
    let mut groups = group(&items, view.group_by, &lookup);
    let subtasks: Vec<Row> = subtasks.into_iter().map(|(row, _)| row).collect();
    nest(&mut groups, &subtasks);
    Ok(ViewData {
        group_by: view.group_by,
        subtasks: mode,
        groups,
        rows: items
            .into_iter()
            .map(|(row, _)| row)
            .chain(subtasks)
            .map(|row| (row.id.clone(), row))
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

/// The filter list's unread issues, for Mark all as read; its views' own conditions don't narrow them.
pub fn unseen_ids(conn: &Connection, viewer: &Viewer, list_id: i64) -> AppResult<Vec<String>> {
    let ctx = viewer.ctx(conn)?;
    let (condition, params) = conditions(conn, viewer, &ctx, list_id, None, &Narrowing::default())?;
    let mut query = conn.prepare(&format!(
        "SELECT i.id FROM cache.issue i WHERE {condition} AND {UNSEEN}"
    ))?;
    let ids = query.query_map(params_from_iter(params), |r| r.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
}

/// The level's conditions, the view's own ones, its kind's own rule and the narrowing, as one SQL condition.
fn conditions(
    conn: &Connection,
    viewer: &Viewer,
    ctx: &Ctx,
    list_id: i64,
    view: Option<&View>,
    narrowing: &Narrowing,
) -> AppResult<(String, Vec<SqlValue>)> {
    let nodes = all_nodes(conn)?;
    let node = nodes
        .iter()
        .find(|n| n.id == list_id)
        .ok_or_else(|| AppError::NotFound(format!("Filter list {list_id} doesn't exist")))?;
    let mut chain = scope_of(&nodes, node).all()?;
    if let Some(view) = view {
        chain.extend(view_conditions(view)?);
    }
    let compiled = compile(&chain, ctx)?;
    let mut condition = compiled.where_clause();
    let mut params = compiled.params;
    match node.kind {
        Kind::Recent => {
            let kept = vec!["?"; narrowing.read_here.len()].join(", ");
            condition.push_str(&format!(" AND ({UNSEEN} OR i.id IN ({kept}))"));
            params.extend(narrowing.read_here.iter().map(|id| id.clone().into()));
        }
        Kind::Pinned => {
            condition.push_str(&format!(
                " AND i.id IN (SELECT p.target_id FROM pin p WHERE p.target_kind = 'issue' AND {})",
                active("?")
            ));
            params.push(format_utc(viewer.now).into());
        }
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
