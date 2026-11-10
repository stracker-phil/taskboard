use std::collections::HashSet;

use super::*;
use crate::db::Db;
use crate::error::ErrorKind;
use crate::views::{GroupBy, Layout};

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
             INSERT INTO cache.field (id, name, type) VALUES
               ('summary', 'Summary', 'string'),
               ('customfield_10001', 'Zone', 'string'),
               ('customfield_10002', 'alpha', 'string'),
               ('customfield_10003', 'Beta', 'string'),
               ('customfield_10004', 'Stale', 'string')",
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
    for id in [ZONE, ALPHA, BETA] {
        crate::field_choice::field_choice_set(&db, id.into(), true, true)
            .await
            .unwrap();
    }
    (dir, db)
}

fn layouts_of(layouts: &[Layout], default_layout: Layout, default_group: GroupBy) -> SpaceLayouts {
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
    let fields: Vec<(&str, &str, bool, bool, bool, bool)> = config
        .fields
        .iter()
        .map(|f| {
            (
                f.id.as_str(),
                f.name.as_str(),
                f.app_list,
                f.app_detail,
                f.in_list,
                f.in_detail,
            )
        })
        .collect();
    assert_eq!(
        fields,
        [
            (ALPHA, "alpha", true, true, true, true),
            (BETA, "Beta", true, true, true, true),
            (ZONE, "Zone", true, true, true, true),
        ]
    );
    assert!(config.fields.iter().all(|f| f.id != STALE));
}

#[tokio::test]
async fn the_settings_page_lists_only_the_fields_opted_in_app_wide() {
    let (_dir, db) = lab().await;
    crate::field_choice::field_choice_set(&db, BETA.into(), false, false)
        .await
        .unwrap();
    crate::field_choice::field_choice_set(&db, ZONE.into(), false, true)
        .await
        .unwrap();

    let config = space_config(&db, A.into()).await.unwrap();

    let fields: Vec<(&str, bool, bool)> = config
        .fields
        .iter()
        .map(|f| (f.id.as_str(), f.app_list, f.app_detail))
        .collect();
    assert_eq!(fields, [(ALPHA, true, true), (ZONE, false, true)]);
}

#[test]
fn a_field_setting_serializes_for_the_page_in_camel_case() {
    let setting = FieldSetting {
        id: ZONE.into(),
        name: "Zone".into(),
        app_list: true,
        app_detail: false,
        in_list: false,
        in_detail: true,
    };

    assert_eq!(
        serde_json::to_value(&setting).unwrap(),
        serde_json::json!({
            "id": ZONE,
            "name": "Zone",
            "appList": true,
            "appDetail": false,
            "inList": false,
            "inDetail": true,
        })
    );
}

#[test]
fn the_places_a_space_keeps_a_field_in_map_to_its_visibility() {
    for (in_list, in_detail, expected) in [
        (true, true, FieldVisibility::Visible),
        (false, true, FieldVisibility::HiddenInList),
        (true, false, FieldVisibility::HiddenInDetail),
        (false, false, FieldVisibility::Hidden),
    ] {
        assert_eq!(
            FieldVisibility::from_places(in_list, in_detail),
            expected,
            "list {in_list}, detail {in_detail}"
        );
    }
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
        let field = config.fields.iter().find(|f| f.id == id).unwrap();
        FieldVisibility::from_places(field.in_list, field.in_detail)
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
    let pinned = id_where(&db, "kind = 'pinned'").await;

    let found = db
        .read(move |conn| {
            Ok([space, in_space, everything, pinned].map(|id| project_of_level(conn, id).unwrap()))
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
            Ok(
                [space_a, list_in_a, space_b, everything]
                    .map(|id| level_layouts(conn, id).unwrap()),
            )
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

    assert_eq!(list, ids(&[ZONE, BETA, STALE]));
    assert_eq!(detail, ids(&[ALPHA, BETA, STALE]));
    assert_eq!(other, ids(&[ALPHA, STALE]));
}

#[tokio::test]
async fn a_field_not_opted_in_for_a_place_is_hidden_there_in_every_space() {
    let (_dir, db) = lab().await;
    crate::field_choice::field_choice_set(&db, ZONE.into(), false, true)
        .await
        .unwrap();
    crate::field_choice::field_choice_set(&db, ALPHA.into(), true, false)
        .await
        .unwrap();
    hide(&db, A, BETA, FieldVisibility::HiddenInDetail).await;

    let (list_a, detail_a, list_b, detail_b) = db
        .read(|conn| {
            Ok((
                hidden_fields(conn, A, Place::List)?,
                hidden_fields(conn, A, Place::Detail)?,
                hidden_fields(conn, B, Place::List)?,
                hidden_fields(conn, B, Place::Detail)?,
            ))
        })
        .await
        .unwrap();

    assert_eq!(list_a, ids(&[ZONE, STALE]));
    assert_eq!(detail_a, ids(&[ALPHA, BETA, STALE]));
    assert_eq!(list_b, ids(&[ZONE, STALE]));
    assert_eq!(detail_b, ids(&[ALPHA, STALE]));
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

    assert_eq!(every, ids(&[ZONE, STALE]));
    assert_eq!(in_everything, ids(&[ZONE, STALE]));
    assert_eq!(in_a, ids(&[ZONE, ALPHA, STALE]));
}

#[tokio::test]
async fn without_tracked_spaces_only_the_fields_not_opted_in_are_hidden() {
    let dir = crate::test_support::temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    db.write(|tx, _| {
        tx.execute_batch(
            "INSERT INTO cache.field (id, name, type) VALUES
               ('customfield_10001', 'Zone', 'string'),
               ('customfield_10002', 'alpha', 'string')",
        )?;
        Ok(())
    })
    .await
    .unwrap();
    crate::field_choice::field_choice_set(&db, ZONE.into(), true, false)
        .await
        .unwrap();

    let hidden = db.read(hidden_in_every_space).await.unwrap();

    assert_eq!(hidden, ids(&[ALPHA]));
}

#[tokio::test]
async fn without_tracked_spaces_and_fields_no_column_is_hidden() {
    let dir = crate::test_support::temp_config_dir();
    let db = Db::open(dir.path()).unwrap();

    let hidden = db.read(hidden_in_every_space).await.unwrap();

    assert!(hidden.is_empty());
}
