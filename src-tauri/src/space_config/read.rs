//! Reading a Space's settings: its layouts, the fields it hides, and its settings page.

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension};

use super::types::{ALL_LAYOUTS, FieldSetting, FieldVisibility, Place, SpaceConfig, SpaceLayouts};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::field_choice::not_chosen;
use crate::lists::{Kind, node};
use crate::views::columns::SHOWABLE;
use crate::views::model::from_text;
use crate::views::{GroupBy, Layout};

/// The Space a level belongs to: its own project for a Space, its Space's for one of its filter lists; none
/// for Everything and Home's lists.
pub fn project_of_level(conn: &Connection, list_id: i64) -> AppResult<Option<String>> {
    let level = node(conn, list_id)?;
    match level.kind {
        Kind::Space => Ok(level.project_id),
        Kind::List => match level.parent_id {
            Some(parent) => {
                let parent = node(conn, parent)?;
                Ok((parent.kind == Kind::Space)
                    .then_some(parent.project_id)
                    .flatten())
            }
            None => Ok(None),
        },
        Kind::Everything | Kind::Pinned | Kind::Viewed => Ok(None),
    }
}

/// A Space's layouts; the defaults for a project without settings.
pub fn space_layouts(conn: &Connection, project_id: &str) -> AppResult<SpaceLayouts> {
    let row: Option<(Option<String>, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT layouts, default_layout, default_group FROM space WHERE project_id = ?1",
            [project_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((layouts, default_layout, default_group)) = row else {
        return Ok(SpaceLayouts::default());
    };
    let stored: Vec<Layout> = layouts
        .and_then(|text| serde_json::from_str::<Vec<Layout>>(&text).ok())
        .unwrap_or_default();
    let layouts: Vec<Layout> = ALL_LAYOUTS
        .into_iter()
        .filter(|l| stored.is_empty() || stored.contains(l))
        .collect();
    let default_layout = default_layout
        .as_deref()
        .and_then(from_text)
        .filter(|l| layouts.contains(l))
        .unwrap_or(layouts[0]);
    Ok(SpaceLayouts {
        layouts,
        default_layout,
        default_group: default_group
            .as_deref()
            .and_then(from_text)
            .unwrap_or(GroupBy::Status),
    })
}

/// The layouts of a level's views: its Space's, or all of them outside the Spaces.
pub fn level_layouts(conn: &Connection, list_id: i64) -> AppResult<SpaceLayouts> {
    match project_of_level(conn, list_id)? {
        Some(project_id) => space_layouts(conn, &project_id),
        None => Ok(SpaceLayouts::default()),
    }
}

fn visibility(text: &str) -> FieldVisibility {
    serde_json::from_value(serde_json::Value::String(text.to_string())).unwrap_or_default()
}

/// The fields a Space keeps out of `place`: those not opted in app-wide, and its own opt-outs.
pub fn hidden_fields(
    conn: &Connection,
    project_id: &str,
    place: Place,
) -> AppResult<HashSet<String>> {
    let mut query =
        conn.prepare("SELECT field_id, visibility FROM space_field WHERE project_id = ?1")?;
    let rows = query.query_map([project_id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut hidden = not_chosen(conn, place)?;
    for row in rows {
        let (id, text) = row?;
        let shown = match place {
            Place::List => visibility(&text).in_list(),
            Place::Detail => visibility(&text).in_detail(),
        };
        if !shown {
            hidden.insert(id);
        }
    }
    Ok(hidden)
}

/// The fields every tracked Space keeps out of the list: a level outside the Spaces offers a field as a
/// column while one Space shows it. Without a tracked Space, the fields not opted in.
pub fn hidden_in_every_space(conn: &Connection) -> AppResult<HashSet<String>> {
    let mut query = conn.prepare("SELECT project_id FROM space WHERE tracked = 1")?;
    let spaces: Vec<String> = query
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut hidden: Option<HashSet<String>> = None;
    for project_id in spaces {
        let here = hidden_fields(conn, &project_id, Place::List)?;
        hidden = Some(match hidden {
            Some(so_far) => so_far.intersection(&here).cloned().collect(),
            None => here,
        });
    }
    match hidden {
        Some(hidden) => Ok(hidden),
        None => not_chosen(conn, Place::List),
    }
}

/// The custom fields a level's views leave out as columns.
pub fn hidden_columns(conn: &Connection, list_id: i64) -> AppResult<HashSet<String>> {
    match project_of_level(conn, list_id)? {
        Some(project_id) => hidden_fields(conn, &project_id, Place::List),
        None => hidden_in_every_space(conn),
    }
}

/// A tracked Space's settings page; NotFound for any other project.
pub(super) fn config(conn: &Connection, project_id: &str) -> AppResult<SpaceConfig> {
    let tracked: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM space WHERE project_id = ?1)",
        [project_id],
        |r| r.get(0),
    )?;
    if !tracked {
        return Err(AppError::NotFound(format!(
            "No Space for project {project_id}"
        )));
    }
    let mut query = conn.prepare(&format!(
        "SELECT id, name, c.in_list, c.in_detail, sf.visibility
         FROM field_choice c JOIN cache.field ON id = c.field_id
         LEFT JOIN space_field sf ON sf.field_id = id AND sf.project_id = ?1
         WHERE {SHOWABLE}
         ORDER BY name COLLATE NOCASE, id"
    ))?;
    let fields = query
        .query_map([project_id], |r| {
            let here = r
                .get::<_, Option<String>>(4)?
                .as_deref()
                .map(visibility)
                .unwrap_or_default();
            Ok(FieldSetting {
                id: r.get(0)?,
                name: r.get(1)?,
                app_list: r.get(2)?,
                app_detail: r.get(3)?,
                in_list: here.in_list(),
                in_detail: here.in_detail(),
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(SpaceConfig {
        project_id: project_id.to_string(),
        layouts: space_layouts(conn, project_id)?,
        fields,
    })
}

pub async fn space_config(db: &Db, project_id: String) -> AppResult<SpaceConfig> {
    db.read(move |conn| config(conn, &project_id)).await
}
