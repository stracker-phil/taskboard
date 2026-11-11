//! A view's issues: the level's filter (ADR 0024), the view's own conditions (ADR 0033) and the toolbar's
//! narrowing, sorted, grouped and shaped as rows. One query per call; grouping and sorting happen here, not in
//! the page.

use std::collections::{HashMap, HashSet};

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, params_from_iter};

use super::columns::custom_columns;
use serde::Serialize;
use ts_rs::TS;

use super::fetch::{Source, fetch};
use super::filter::view_conditions;
use super::groups::{Facts, Group, Lookup, group};
use super::lookup::{spaces, versions};
use super::model::{GroupBy, SubtaskMode, View};
use super::narrowing::Narrowing;
use super::read::read_view;
use super::rows::{Person, Row};
use super::sorting::{fold, sort};
use super::subtasks::{Nested, as_items, descendants, expand, nest};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::filter::{Ctx, compile};
use crate::lists::{Kind, Viewer, all_nodes, scope_of};
use crate::me_mode::{self, MeRule};
use crate::pins::active;
use crate::status_style::Styles;
use crate::team::{Me, with_me_first};
use crate::time::format_utc;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ViewData {
    pub group_by: GroupBy,
    pub subtasks: SubtaskMode,
    pub groups: Vec<Group>,
    pub rows: HashMap<String, Row>,
    pub columns: Vec<String>,
    /// How many issues the view shows: every row, subtasks included.
    pub total: u32,
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

/// The view and its issues: the level's filter, its kind's rule and the narrowing, sorted.
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
    let mine = match narrowing.me && mode == SubtaskMode::List {
        true => Some(my_issue_ids(conn, &narrowing.me_rules, &ctx)?),
        false => None,
    };
    let narrowed = RowNarrowing::new(narrowing, mine);
    let base = Narrowing {
        me: narrowing.me && mode != SubtaskMode::List,
        person: narrowing
            .person
            .clone()
            .filter(|_| mode != SubtaskMode::List),
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

/// The Me chip, the person chip and the text field as a test on one row.
struct RowNarrowing<'a> {
    /// My issues while Me mode is on.
    mine: Option<HashSet<String>>,
    person: Option<&'a str>,
    needle: String,
}

impl<'a> RowNarrowing<'a> {
    fn new(narrowing: &'a Narrowing, mine: Option<HashSet<String>>) -> Self {
        Self {
            mine,
            person: narrowing.person.as_deref(),
            needle: fold(narrowing.text.trim()),
        }
    }

    fn text_matches(&self, (row, _): &(Row, Facts)) -> bool {
        self.needle.is_empty()
            || fold(&row.key).contains(&self.needle)
            || fold(&row.summary).contains(&self.needle)
    }

    fn matches(&self, item: &(Row, Facts)) -> bool {
        let assignee = item.1.assignee_id.as_deref();
        let mine = self
            .mine
            .as_ref()
            .is_none_or(|ids| ids.contains(&item.0.id));
        let theirs = self.person.is_none() || assignee == self.person;
        mine && theirs && self.text_matches(item)
    }
}

/// Whom the person chip offers: me, then whoever has one of the view's issues as its filter and Me mode leave
/// them, before the person chip narrows them.
pub async fn view_people(
    db: &Db,
    viewer: Viewer,
    me: Option<Me>,
    list_id: i64,
    view_id: i64,
    narrowing: Narrowing,
) -> AppResult<Vec<Person>> {
    db.read(move |conn| {
        let unpicked = Narrowing {
            person: None,
            ..narrowing
        };
        let fetched = fetch_view(conn, &viewer, list_id, view_id, &unpicked, None)?;
        let assignees: HashSet<String> = fetched
            .items
            .iter()
            .filter_map(|(_, facts)| facts.assignee_id.clone())
            .collect();
        with_me_first(conn, &assignees, me.as_ref())
    })
    .await
}

/// Every issue Me mode keeps, for narrowing rows one by one.
fn my_issue_ids(conn: &Connection, rules: &[MeRule], ctx: &Ctx) -> AppResult<HashSet<String>> {
    let (condition, params) = me_mode::condition(rules, ctx)?;
    let mut query = conn.prepare(&format!("SELECT i.id FROM cache.issue i WHERE {condition}"))?;
    let ids = query.query_map(params_from_iter(params), |r| r.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
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
    let rows: HashMap<String, Row> = items
        .into_iter()
        .map(|(row, _)| row)
        .chain(below.into_values().flatten())
        .map(|row| (row.id.clone(), row))
        .collect();
    Ok(ViewData {
        group_by: view.group_by,
        subtasks: mode,
        groups,
        total: rows.len() as u32,
        rows,
        columns: view.columns,
    })
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
    if node.kind == Kind::Pinned {
        condition.push_str(&format!(
            " AND i.id IN (SELECT p.target_id FROM pin p WHERE p.target_kind = 'issue' AND {})",
            active("?")
        ));
        params.push(format_utc(viewer.now).into());
    }
    if narrowing.me {
        let (mine, mine_params) = me_mode::condition(&narrowing.me_rules, ctx)?;
        condition.push_str(&format!(" AND {mine}"));
        params.extend(mine_params);
    }
    if let Some(person) = &narrowing.person {
        condition.push_str(" AND i.assignee_id = ?");
        params.push(person.clone().into());
    }
    if let Some(epic) = &narrowing.epic {
        condition.push_str(" AND i.parent_id = ?");
        params.push(epic.clone().into());
    }
    Ok((condition, params))
}
