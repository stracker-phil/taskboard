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
use super::model::{GroupBy, Layout, SubtaskMode, View};
use super::rows::Row;
use super::sorting::{fold, sort};
use super::subtasks::{Nested, as_items, descendants, expand, nest};
use crate::changes::{Names, UNSEEN, chip, facts, unseen_changes};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::filter::{Condition, Ctx, compile};
use crate::issues::status_names;
use crate::lists::{Kind, Viewer, all_nodes, scope_of};
use crate::pins::active;
use crate::status_style::Styles;
use crate::sync::map::format_utc;

/// The toolbar's narrowing on top of the level's filter, and the Filter popover's unsaved conditions (ADR 0038).
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
    /// Unsaved conditions in place of the filter list's own ones; only a filter list has its own.
    pub list_draft: Option<Vec<Condition>>,
    /// Unsaved conditions in place of the view's own ones.
    pub view_draft: Option<Vec<Condition>>,
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
    /// What the view groups: the results in Hidden, the roots in Hierarchy, the results and their descendants
    /// in List (ADR 0040).
    pub items: Vec<(Row, Facts)>,
    /// In Hierarchy, each root's subtree in depth order; it never went through the filter.
    pub below: HashMap<String, Vec<Row>>,
    pub ctx: Ctx,
}

impl Fetched {
    /// Every issue the view shows, collapsed subtrees included.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        let items = self.items.iter().map(|(row, _)| row.id.as_str());
        items.chain(self.below.values().flatten().map(|row| row.id.as_str()))
    }
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
    // In List the Me chip and the text narrow every row on its own, descendants too; elsewhere the results.
    let narrowed = RowNarrowing::new(narrowing, ctx.me.as_deref());
    let base = Narrowing {
        me: narrowing.me && mode != SubtaskMode::List,
        ..narrowing.clone()
    };
    let (condition, params) = conditions(conn, viewer, &ctx, list_id, Some(&view), &base)?;
    let mut results = fetch(conn, &source, &condition, params)?;
    if mode != SubtaskMode::List {
        results.retain(|item| narrowed.text_matches(item));
    }
    let mut descendants = match mode {
        SubtaskMode::Hidden => Vec::new(),
        _ => descendants(conn, &source, &results)?,
    };
    mark_changed(conn, ctx.me.as_deref(), &mut results, &mut descendants)?;
    let (by, descending) = (view.sort.by, view.sort.descending);
    let (items, below) = match mode {
        SubtaskMode::Hidden => {
            sort(&mut results, by, descending);
            (results, HashMap::new())
        }
        SubtaskMode::List => {
            let mut items = as_items(results, descendants);
            items.retain(|item| narrowed.matches(item));
            sort(&mut items, by, descending);
            (items, HashMap::new())
        }
        SubtaskMode::Hierarchy => {
            sort(&mut results, by, descending);
            sort(&mut descendants, by, descending);
            let Nested { roots, below } = nest(results, descendants);
            (roots, below)
        }
    };
    Ok(Fetched {
        view,
        mode,
        items,
        below,
        ctx,
    })
}

/// The Me chip and the text field as a test on one row.
struct RowNarrowing<'a> {
    only_mine: bool,
    me: Option<&'a str>,
    needle: String,
}

impl<'a> RowNarrowing<'a> {
    fn new(narrowing: &Narrowing, me: Option<&'a str>) -> Self {
        Self {
            only_mine: narrowing.me,
            me,
            needle: fold(narrowing.text.trim()),
        }
    }

    fn text_matches(&self, (row, _): &(Row, Facts)) -> bool {
        self.needle.is_empty()
            || fold(&row.key).contains(&self.needle)
            || fold(&row.summary).contains(&self.needle)
    }

    /// Without a connected account the Me chip matches nobody.
    fn matches(&self, item: &(Row, Facts)) -> bool {
        let mine =
            !self.only_mine || (self.me.is_some() && item.1.assignee_id.as_deref() == self.me);
        mine && self.text_matches(item)
    }
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
        below,
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
    expand(&mut groups, &below);
    Ok(ViewData {
        group_by: view.group_by,
        subtasks: mode,
        groups,
        rows: items
            .into_iter()
            .map(|(row, _)| row)
            .chain(below.into_values().flatten())
            .map(|row| (row.id.clone(), row))
            .collect(),
        columns: view.columns,
    })
}

/// Rows with unseen changes are highlighted and say what changed.
fn mark_changed(
    conn: &Connection,
    me: Option<&str>,
    results: &mut [(Row, Facts)],
    descendants: &mut [(Row, Facts)],
) -> AppResult<()> {
    let ids: Vec<&str> = results
        .iter()
        .chain(descendants.iter())
        .map(|(row, _)| row.id.as_str())
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    let unseen = unseen_changes(conn, &ids)?;
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
    for (row, _) in results.iter_mut().chain(descendants.iter_mut()) {
        if let Some(changes) = unseen.get(&row.id) {
            row.changed = true;
            row.change_note = chip(&facts(changes), &names);
        }
    }
    Ok(())
}

/// The view of a level and how the toolbar narrows it.
pub struct ViewAt {
    pub view_id: i64,
    pub narrowing: Narrowing,
}

/// The unread issues Mark all as read marks: exactly those the view shows in its mode (ADR 0040), collapsed
/// subtrees included; without a view, every unread issue of the filter list.
pub fn unseen_ids(
    conn: &Connection,
    viewer: &Viewer,
    list_id: i64,
    view: Option<&ViewAt>,
) -> AppResult<Vec<String>> {
    let (condition, params) = match view {
        Some(at) => {
            let board = read_view(conn, at.view_id)?.layout == Layout::Board;
            let mode = board.then_some(SubtaskMode::Hidden);
            let fetched = fetch_view(conn, viewer, list_id, at.view_id, &at.narrowing, mode)?;
            let ids: Vec<&str> = fetched.ids().collect();
            let ids = serde_json::to_string(&ids)?;
            (
                "i.id IN (SELECT value FROM json_each(?))".to_string(),
                vec![ids.into()],
            )
        }
        None => {
            let ctx = viewer.ctx(conn)?;
            conditions(conn, viewer, &ctx, list_id, None, &Narrowing::default())?
        }
    };
    let mut query = conn.prepare(&format!(
        "SELECT i.id FROM cache.issue i WHERE {condition} AND {UNSEEN}"
    ))?;
    let ids = query.query_map(params_from_iter(params), |r| r.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
}

/// The level's conditions, the view's own ones, its kind's own rule and the narrowing, as one SQL condition.
/// The narrowing's drafts stand in for the list's and the view's own conditions; they compile under the same
/// chain as `list_set_filter` and `view_set_filter` check them.
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
    let scope = scope_of(&nodes, node);
    let mut chain = match &narrowing.list_draft {
        Some(draft) if node.kind == Kind::List => [scope.inherited, draft.clone()].concat(),
        _ => scope.all()?,
    };
    match (view, &narrowing.view_draft) {
        (Some(_), Some(draft)) => chain.extend(draft.iter().cloned()),
        (Some(view), None) => chain.extend(view_conditions(view)?),
        (None, _) => {}
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
