//! A Space's own settings: the layouts its views offer, the layout and grouping of its new views, and how
//! each custom field shows (`space` columns, `space_field`). Status styles per Space live in `status_style`.

pub mod commands;

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::filter::RELEVANT_FIELD;
use crate::lists::{Kind, node};
use crate::views::columns::SHOWABLE;
use crate::views::model::{from_text, to_text};
use crate::views::{GroupBy, Layout};

pub const ALL_LAYOUTS: [Layout; 3] = [Layout::List, Layout::Board, Layout::Table];

/// Where a custom field shows in a Space; `Visible` is the default.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum FieldVisibility {
    /// A column choice in the list and shown in the Issue view when filled.
    #[default]
    Visible,
    /// Shown in the Issue view, never offered as a column.
    HiddenInList,
    /// A column choice, left out of the Issue view.
    HiddenInDetail,
    Hidden,
}

impl FieldVisibility {
    pub fn in_list(self) -> bool {
        matches!(self, Self::Visible | Self::HiddenInDetail)
    }

    pub fn in_detail(self) -> bool {
        matches!(self, Self::Visible | Self::HiddenInList)
    }
}

/// The layouts a Space's views offer, and how its new views start.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SpaceLayouts {
    /// The layouts its views can show, in List, Board, Table order; never empty.
    pub layouts: Vec<Layout>,
    /// One of `layouts`: what a new view gets, and what a view in a switched-off layout shows.
    pub default_layout: Layout,
    /// How new views group.
    pub default_group: GroupBy,
}

impl Default for SpaceLayouts {
    fn default() -> Self {
        Self {
            layouts: ALL_LAYOUTS.to_vec(),
            default_layout: Layout::List,
            default_group: GroupBy::Status,
        }
    }
}

impl SpaceLayouts {
    pub fn shown(&self, layout: Layout) -> Layout {
        if self.layouts.contains(&layout) {
            layout
        } else {
            self.default_layout
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct FieldSetting {
    pub id: String,
    pub name: String,
    pub visibility: FieldVisibility,
}

/// A Space's settings page, apart from its name and look (`SpaceUpdate`) and its statuses.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SpaceConfig {
    pub project_id: String,
    pub layouts: SpaceLayouts,
    /// The custom fields worth a setting (`RELEVANT_FIELD`), by name.
    pub fields: Vec<FieldSetting>,
}

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
        Kind::Everything | Kind::Recent | Kind::Pinned => Ok(None),
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

/// The fields a Space keeps out of `place`.
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
    let mut hidden = HashSet::new();
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
/// column while one Space shows it.
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
    Ok(hidden.unwrap_or_default())
}

/// The custom fields a level's views leave out as columns.
pub fn hidden_columns(conn: &Connection, list_id: i64) -> AppResult<HashSet<String>> {
    match project_of_level(conn, list_id)? {
        Some(project_id) => hidden_fields(conn, &project_id, Place::List),
        None => hidden_in_every_space(conn),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    List,
    Detail,
}

fn read(conn: &Connection, project_id: &str) -> AppResult<SpaceConfig> {
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
        "SELECT id, name, sf.visibility FROM cache.field
         LEFT JOIN space_field sf ON sf.field_id = id AND sf.project_id = ?1
         WHERE {SHOWABLE} AND {RELEVANT_FIELD}
         ORDER BY name COLLATE NOCASE, id"
    ))?;
    let fields = query
        .query_map([project_id], |r| {
            Ok(FieldSetting {
                id: r.get(0)?,
                name: r.get(1)?,
                visibility: r
                    .get::<_, Option<String>>(2)?
                    .as_deref()
                    .map(visibility)
                    .unwrap_or_default(),
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
    db.read(move |conn| read(conn, &project_id)).await
}

pub async fn space_set_layouts(
    db: &Db,
    project_id: String,
    layouts: SpaceLayouts,
) -> AppResult<SpaceConfig> {
    if layouts.layouts.is_empty() {
        return Err(AppError::InvalidInput(
            "A Space needs at least one layout.".into(),
        ));
    }
    if !layouts.layouts.contains(&layouts.default_layout) {
        return Err(AppError::InvalidInput(
            "The default layout must be one the Space offers.".into(),
        ));
    }
    db.write(move |tx, changes| {
        read(tx, &project_id)?;
        let offered: Vec<Layout> = ALL_LAYOUTS
            .into_iter()
            .filter(|l| layouts.layouts.contains(l))
            .collect();
        let stored = (offered.len() < ALL_LAYOUTS.len())
            .then(|| serde_json::to_string(&offered))
            .transpose()?;
        tx.execute(
            "UPDATE space SET layouts = ?2, default_layout = ?3, default_group = ?4 WHERE project_id = ?1",
            params![
                project_id,
                stored,
                to_text(layouts.default_layout),
                to_text(layouts.default_group)
            ],
        )?;
        changes.local_changed(LocalPart::Spaces);
        changes.local_changed(LocalPart::Views);
        read(tx, &project_id)
    })
    .await
}

pub async fn space_set_field(
    db: &Db,
    project_id: String,
    field_id: String,
    visibility: FieldVisibility,
) -> AppResult<SpaceConfig> {
    db.write(move |tx, changes| {
        read(tx, &project_id)?;
        let known: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM cache.field WHERE id = ?1)",
            [&field_id],
            |r| r.get(0),
        )?;
        if !known {
            return Err(AppError::NotFound(format!(
                "Field {field_id} doesn't exist"
            )));
        }
        if visibility == FieldVisibility::Visible {
            tx.execute(
                "DELETE FROM space_field WHERE project_id = ?1 AND field_id = ?2",
                [&project_id, &field_id],
            )?;
        } else {
            tx.execute(
                "INSERT INTO space_field (project_id, field_id, visibility) VALUES (?1, ?2, ?3)
                 ON CONFLICT (project_id, field_id) DO UPDATE SET visibility = excluded.visibility",
                params![project_id, field_id, to_text(visibility)],
            )?;
        }
        changes.local_changed(LocalPart::Views);
        changes.local_changed(LocalPart::Spaces);
        read(tx, &project_id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    const A: &str = "10001";
    const B: &str = "10002";
    const ZONE: &str = "customfield_10001";
    const ALPHA: &str = "customfield_10002";
    const BETA: &str = "customfield_10003";
    const STALE: &str = "customfield_10004";

    /// Two tracked Spaces and three fields the tracked issues fill; a fourth field nobody uses.
    async fn lab() -> (tempfile::TempDir, Db) {
        let dir = crate::test_support::temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| {
            tx.execute_batch(
                "INSERT INTO cache.space (id, key, name, raw) VALUES
                   ('10001', 'NW', 'Website relaunch', '{}'), ('10002', 'NWS', 'Northwind support', '{}');
                 INSERT INTO cache.field (id, name, type, used) VALUES
                   ('summary', 'Summary', 'string', 1),
                   ('customfield_10001', 'Zone', 'string', 1),
                   ('customfield_10002', 'alpha', 'string', 1),
                   ('customfield_10003', 'Beta', 'string', 1),
                   ('customfield_10004', 'Stale', 'string', 0)",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        for id in [A, B] {
            crate::connection::track_project(&db, id.into(), true)
                .await
                .unwrap();
        }
        (dir, db)
    }

    fn layouts_of(
        layouts: &[Layout],
        default_layout: Layout,
        default_group: GroupBy,
    ) -> SpaceLayouts {
        SpaceLayouts {
            layouts: layouts.to_vec(),
            default_layout,
            default_group,
        }
    }

    async fn hide(db: &Db, project: &str, field: &str, visibility: FieldVisibility) {
        space_set_field(db, project.into(), field.into(), visibility)
            .await
            .unwrap();
    }

    fn ids(items: &[&str]) -> HashSet<String> {
        items.iter().map(|id| id.to_string()).collect()
    }

    async fn id_where(db: &Db, clause: &'static str) -> i64 {
        db.read(move |conn| {
            Ok(conn.query_row(
                &format!("SELECT id FROM filter_list WHERE {clause} ORDER BY id LIMIT 1"),
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
    }

    async fn space_node_id(db: &Db, project: &'static str) -> i64 {
        db.read(move |conn| {
            Ok(conn.query_row(
                "SELECT id FROM filter_list WHERE kind = 'space' AND project_id = ?1",
                [project],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
    }

    async fn field_rows(db: &Db) -> i64 {
        db.read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM space_field", [], |r| r.get(0))?))
            .await
            .unwrap()
    }

    async fn stored_layouts(db: &Db, project: &'static str) -> Option<String> {
        db.read(move |conn| {
            Ok(conn.query_row(
                "SELECT layouts FROM space WHERE project_id = ?1",
                [project],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
    }

    #[test]
    fn a_fields_visibility_decides_where_it_shows() {
        for (visibility, in_list, in_detail) in [
            (FieldVisibility::Visible, true, true),
            (FieldVisibility::HiddenInList, false, true),
            (FieldVisibility::HiddenInDetail, true, false),
            (FieldVisibility::Hidden, false, false),
        ] {
            assert_eq!(visibility.in_list(), in_list, "{visibility:?}");
            assert_eq!(visibility.in_detail(), in_detail, "{visibility:?}");
        }
    }

    #[test]
    fn a_layout_the_space_switched_off_shows_as_its_default() {
        let layouts = SpaceLayouts {
            layouts: vec![Layout::List, Layout::Table],
            default_layout: Layout::Table,
            default_group: GroupBy::Status,
        };

        for (asked, shown) in [
            (Layout::List, Layout::List),
            (Layout::Board, Layout::Table),
            (Layout::Table, Layout::Table),
        ] {
            assert_eq!(layouts.shown(asked), shown, "{asked:?}");
        }
    }

    #[test]
    fn the_settings_serialize_for_the_page_in_camel_case() {
        let value = serde_json::to_value(SpaceLayouts::default()).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "layouts": ["list", "board", "table"],
                "defaultLayout": "list",
                "defaultGroup": "status",
            })
        );
        assert_eq!(
            serde_json::to_value(FieldVisibility::HiddenInDetail).unwrap(),
            "hiddenInDetail"
        );
    }

    #[tokio::test]
    async fn a_space_without_settings_offers_every_layout_and_every_field() {
        let (_dir, db) = lab().await;

        let config = space_config(&db, A.into()).await.unwrap();

        assert_eq!(config.project_id, A);
        assert_eq!(config.layouts, SpaceLayouts::default());
        let fields: Vec<(&str, &str, FieldVisibility)> = config
            .fields
            .iter()
            .map(|f| (f.id.as_str(), f.name.as_str(), f.visibility))
            .collect();
        assert_eq!(
            fields,
            [
                (ALPHA, "alpha", FieldVisibility::Visible),
                (BETA, "Beta", FieldVisibility::Visible),
                (ZONE, "Zone", FieldVisibility::Visible),
            ]
        );
        assert!(config.fields.iter().all(|f| f.id != STALE));
    }

    #[tokio::test]
    async fn a_subset_of_layouts_is_stored_in_list_board_table_order() {
        let (_dir, db) = lab().await;
        let wanted = layouts_of(
            &[Layout::Table, Layout::List],
            Layout::Table,
            GroupBy::Assignee,
        );

        let config = space_set_layouts(&db, A.into(), wanted).await.unwrap();

        assert_eq!(
            config.layouts,
            SpaceLayouts {
                layouts: vec![Layout::List, Layout::Table],
                default_layout: Layout::Table,
                default_group: GroupBy::Assignee,
            }
        );
        assert_eq!(config, space_config(&db, A.into()).await.unwrap());
        assert_eq!(
            stored_layouts(&db, A).await.as_deref(),
            Some(r#"["list","table"]"#)
        );
        assert_eq!(
            space_config(&db, B.into()).await.unwrap().layouts,
            SpaceLayouts::default()
        );
    }

    #[tokio::test]
    async fn offering_every_layout_again_clears_the_stored_subset() {
        let (_dir, db) = lab().await;
        let subset = layouts_of(&[Layout::Board], Layout::Board, GroupBy::Status);
        space_set_layouts(&db, A.into(), subset).await.unwrap();

        let all = layouts_of(&ALL_LAYOUTS, Layout::Board, GroupBy::Priority);
        let config = space_set_layouts(&db, A.into(), all).await.unwrap();

        assert_eq!(stored_layouts(&db, A).await, None);
        assert_eq!(config.layouts.layouts, ALL_LAYOUTS);
        assert_eq!(config.layouts.default_layout, Layout::Board);
        assert_eq!(config.layouts.default_group, GroupBy::Priority);
    }

    #[tokio::test]
    async fn no_layouts_and_a_default_outside_them_are_refused() {
        let (_dir, db) = lab().await;
        let none = layouts_of(&[], Layout::List, GroupBy::Status);
        let outside = layouts_of(&[Layout::List], Layout::Board, GroupBy::Status);

        for (wanted, what) in [(none, "no layouts"), (outside, "default outside")] {
            let err = space_set_layouts(&db, A.into(), wanted).await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{what}");
        }
        assert_eq!(
            space_config(&db, A.into()).await.unwrap().layouts,
            SpaceLayouts::default()
        );
    }

    #[tokio::test]
    async fn an_unknown_space_is_not_found() {
        let (_dir, db) = lab().await;
        let wanted = layouts_of(&ALL_LAYOUTS, Layout::List, GroupBy::Status);

        let errors = [
            space_config(&db, "99999".into()).await.unwrap_err(),
            space_set_layouts(&db, "99999".into(), wanted)
                .await
                .unwrap_err(),
            space_set_field(&db, "99999".into(), ZONE.into(), FieldVisibility::Hidden)
                .await
                .unwrap_err(),
        ];

        for err in errors {
            assert_eq!(err.kind(), ErrorKind::NotFound);
        }
    }

    #[tokio::test]
    async fn an_unknown_field_is_not_found() {
        let (_dir, db) = lab().await;

        let err = space_set_field(
            &db,
            A.into(),
            "customfield_99999".into(),
            FieldVisibility::Hidden,
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn a_fields_visibility_round_trips_per_space_and_visible_deletes_the_row() {
        let (_dir, db) = lab().await;
        let visibility_in = |config: &SpaceConfig, id: &str| {
            config
                .fields
                .iter()
                .find(|f| f.id == id)
                .unwrap()
                .visibility
        };

        for visibility in [
            FieldVisibility::HiddenInList,
            FieldVisibility::HiddenInDetail,
            FieldVisibility::Hidden,
        ] {
            let config = space_set_field(&db, A.into(), ZONE.into(), visibility)
                .await
                .unwrap();

            assert_eq!(visibility_in(&config, ZONE), visibility);
            assert_eq!(visibility_in(&config, ALPHA), FieldVisibility::Visible);
            assert_eq!(field_rows(&db).await, 1, "{visibility:?}");
            let other = space_config(&db, B.into()).await.unwrap();
            assert_eq!(visibility_in(&other, ZONE), FieldVisibility::Visible);
        }

        let config = space_set_field(&db, A.into(), ZONE.into(), FieldVisibility::Visible)
            .await
            .unwrap();

        assert_eq!(visibility_in(&config, ZONE), FieldVisibility::Visible);
        assert_eq!(field_rows(&db).await, 0);
    }

    #[tokio::test]
    async fn a_level_belongs_to_its_space_or_to_none() {
        let (_dir, db) = lab().await;
        let space = space_node_id(&db, A).await;
        let in_space = id_where(&db, "kind = 'list' AND parent_id IN (SELECT id FROM filter_list WHERE kind = 'space' AND project_id = '10001')").await;
        let everything = id_where(&db, "kind = 'everything'").await;
        let recent = id_where(&db, "kind = 'recent'").await;

        let found = db
            .read(move |conn| {
                Ok([space, in_space, everything, recent]
                    .map(|id| project_of_level(conn, id).unwrap()))
            })
            .await
            .unwrap();

        assert_eq!(
            found,
            [Some(A.to_string()), Some(A.to_string()), None, None]
        );
    }

    #[tokio::test]
    async fn levels_use_their_spaces_layouts_and_all_of_them_outside_the_spaces() {
        let (_dir, db) = lab().await;
        let wanted = layouts_of(&[Layout::List, Layout::Table], Layout::Table, GroupBy::Epic);
        space_set_layouts(&db, A.into(), wanted.clone())
            .await
            .unwrap();
        let space_a = space_node_id(&db, A).await;
        let space_b = space_node_id(&db, B).await;
        let list_in_a = id_where(&db, "kind = 'list' AND parent_id IN (SELECT id FROM filter_list WHERE kind = 'space' AND project_id = '10001')").await;
        let everything = id_where(&db, "kind = 'everything'").await;

        let found = db
            .read(move |conn| {
                Ok([space_a, list_in_a, space_b, everything]
                    .map(|id| level_layouts(conn, id).unwrap()))
            })
            .await
            .unwrap();

        assert_eq!(
            found,
            [
                wanted.clone(),
                wanted,
                SpaceLayouts::default(),
                SpaceLayouts::default()
            ]
        );
    }

    #[tokio::test]
    async fn the_fields_a_space_hides_depend_on_the_place() {
        let (_dir, db) = lab().await;
        hide(&db, A, ZONE, FieldVisibility::HiddenInList).await;
        hide(&db, A, ALPHA, FieldVisibility::HiddenInDetail).await;
        hide(&db, A, BETA, FieldVisibility::Hidden).await;
        hide(&db, B, ALPHA, FieldVisibility::Hidden).await;

        let (list, detail, other) = db
            .read(|conn| {
                Ok((
                    hidden_fields(conn, A, Place::List)?,
                    hidden_fields(conn, A, Place::Detail)?,
                    hidden_fields(conn, B, Place::List)?,
                ))
            })
            .await
            .unwrap();

        assert_eq!(list, ids(&[ZONE, BETA]));
        assert_eq!(detail, ids(&[ALPHA, BETA]));
        assert_eq!(other, ids(&[ALPHA]));
    }

    #[tokio::test]
    async fn outside_the_spaces_a_column_is_hidden_only_when_every_tracked_space_hides_it() {
        let (_dir, db) = lab().await;
        hide(&db, A, ZONE, FieldVisibility::HiddenInList).await;
        hide(&db, B, ZONE, FieldVisibility::Hidden).await;
        hide(&db, A, ALPHA, FieldVisibility::Hidden).await;
        hide(&db, A, BETA, FieldVisibility::HiddenInDetail).await;
        hide(&db, B, BETA, FieldVisibility::HiddenInDetail).await;
        let space_a = space_node_id(&db, A).await;
        let everything = id_where(&db, "kind = 'everything'").await;

        let (every, in_a, in_everything) = db
            .read(move |conn| {
                Ok((
                    hidden_in_every_space(conn)?,
                    hidden_columns(conn, space_a)?,
                    hidden_columns(conn, everything)?,
                ))
            })
            .await
            .unwrap();

        assert_eq!(every, ids(&[ZONE]));
        assert_eq!(in_everything, ids(&[ZONE]));
        assert_eq!(in_a, ids(&[ZONE, ALPHA]));
    }

    #[tokio::test]
    async fn without_tracked_spaces_no_column_is_hidden() {
        let dir = crate::test_support::temp_config_dir();
        let db = Db::open(dir.path()).unwrap();

        let hidden = db.read(hidden_in_every_space).await.unwrap();

        assert!(hidden.is_empty());
    }
}
