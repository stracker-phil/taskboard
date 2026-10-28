//! A view's rows on the demo site synced from the in-process fake Jira into a temp database: grouping, sort,
//! narrowing, custom columns, status style overrides and the columns catalogue.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use tempfile::TempDir;

use taskboard_lib::connection::{
    ConnectionInput, JiraAccess, fetch_versions, refresh_projects, save, track_project,
};
use taskboard_lib::db::Db;
use taskboard_lib::error::ErrorKind;
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::lists::{SidebarTree, Viewer, sidebar_tree};
use taskboard_lib::status_style::{Style, status_style_set, status_styles_list};
use taskboard_lib::store::settings::SettingsStore;
use taskboard_lib::sync::{Emit, SyncContext, full_sync};
use taskboard_lib::test_support::demo::{DEMO_NOW, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};
use taskboard_lib::views::groups::{Group, GroupMark};
use taskboard_lib::views::rows::{Cell, Row};
use taskboard_lib::views::{
    GroupBy, Narrowing, Sort, SortBy, ViewData, columns_catalogue, view_rows, view_set_columns,
    view_set_group, view_set_sort, views_of,
};

const ME_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
const REVIEWER: &str = "customfield_10071";
const STORY_POINTS: &str = "customfield_10016";
const PROJECT_IDS: [&str; 5] = ["10001", "10002", "10003", "10004", "10005"];
const NW: &str = "10001";

fn viewer() -> Viewer {
    Viewer {
        me: Some(ME_ACCOUNT_ID.into()),
        time_zone: Some("Europe/Vienna".into()),
        now: DEMO_NOW.parse::<DateTime<Utc>>().unwrap(),
    }
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()
}

async fn query<T: Send + 'static>(
    db: &Db,
    sql: impl Into<String>,
    map: fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Vec<T> {
    let sql = sql.into();
    db.read(move |conn| {
        let mut statement = conn.prepare(&sql)?;
        let found = statement
            .query_map([], map)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(found)
    })
    .await
    .unwrap()
}

async fn number(db: &Db, sql: impl Into<String>) -> i64 {
    query(db, sql, |row| row.get::<_, i64>(0)).await[0]
}

async fn text(db: &Db, sql: impl Into<String>) -> String {
    query(db, sql, |row| row.get::<_, String>(0))
        .await
        .remove(0)
}

async fn strings(db: &Db, sql: impl Into<String>) -> Vec<String> {
    query(db, sql, |row| row.get::<_, String>(0)).await
}

struct Site {
    _fake: FakeJira,
    _dir: TempDir,
    ctx: SyncContext,
}

impl Site {
    async fn new() -> Self {
        let dir = temp_config_dir();
        let ctx = SyncContext {
            settings: SettingsStore::open(dir.path()),
            access: JiraAccess::new(
                Arc::new(FakeSecrets::default()),
                Arc::new(Limiter::new(1000, 1000)),
            ),
            db: Db::open(dir.path()).unwrap(),
        };
        let fake = FakeJira::start_demo().await;
        let input = ConnectionInput {
            site_url: fake.url().to_string(),
            email: ME_EMAIL.into(),
            token_kind: TokenKind::Classic,
            token_expires: None,
            cloud_id: None,
        };
        save(
            &ctx.settings,
            &ctx.access,
            input,
            Some(DEMO_TOKEN.into()),
            today(),
        )
        .await
        .unwrap();
        let client = fake.client(TokenKind::Classic);
        refresh_projects(&ctx.db, &client).await.unwrap();
        for id in PROJECT_IDS {
            fetch_versions(&ctx.db, &client, id.into()).await.unwrap();
            track_project(&ctx.db, id.into(), true).await.unwrap();
        }
        let site = Self {
            _fake: fake,
            _dir: dir,
            ctx,
        };
        site.sync().await;
        site
    }

    async fn sync(&self) {
        let emit: Emit = Arc::new(|_| {});
        full_sync(&self.ctx, &*emit).await.unwrap();
    }

    fn db(&self) -> &Db {
        &self.ctx.db
    }

    async fn tree(&self) -> SidebarTree {
        sidebar_tree(self.db(), viewer()).await.unwrap()
    }

    async fn everything(&self) -> i64 {
        number(
            self.db(),
            "SELECT id FROM filter_list WHERE kind = 'everything'",
        )
        .await
    }

    async fn nw_space(&self) -> i64 {
        number(
            self.db(),
            format!("SELECT id FROM filter_list WHERE kind = 'space' AND project_id = '{NW}'"),
        )
        .await
    }

    /// The level's first view (the default tab).
    async fn first_view(&self, list: i64) -> i64 {
        views_of(self.db(), list).await.unwrap()[0].id
    }

    async fn data(&self, list: i64, view: i64, narrowing: Narrowing) -> ViewData {
        view_rows(self.db(), viewer(), list, view, narrowing)
            .await
            .unwrap()
    }

    /// The Website relaunch Space and its default view.
    async fn nw(&self) -> (i64, i64) {
        let space = self.nw_space().await;
        (space, self.first_view(space).await)
    }

    async fn nw_data(&self, narrowing: Narrowing) -> ViewData {
        let (space, view) = self.nw().await;
        self.data(space, view, narrowing).await
    }

    async fn issue_id(&self, key: &str) -> String {
        text(
            self.db(),
            format!("SELECT id FROM cache.issue WHERE key = '{key}'"),
        )
        .await
    }
}

fn narrowing_text(text: &str) -> Narrowing {
    Narrowing {
        text: text.into(),
        ..Narrowing::default()
    }
}

fn style_of(group: &Group) -> Option<Style> {
    match &group.mark {
        Some(GroupMark::Status { style }) => Some(*style),
        _ => None,
    }
}

fn row_of<'a>(data: &'a ViewData, key: &str) -> &'a Row {
    data.rows
        .values()
        .find(|row| row.key == key)
        .unwrap_or_else(|| panic!("no row {key}"))
}

fn sorted_keys(data: &ViewData) -> Vec<String> {
    let mut keys: Vec<String> = data.rows.values().map(|row| row.key.clone()).collect();
    keys.sort();
    keys
}

fn issue_number(key: &str) -> u64 {
    key.rsplit_once('-').unwrap().1.parse().unwrap()
}

fn group_keys(data: &ViewData, group: &Group) -> Vec<String> {
    group
        .row_ids
        .iter()
        .map(|id| data.rows[id].key.clone())
        .collect()
}

fn assert_consistent(data: &ViewData) {
    let mut seen = 0;
    for group in &data.groups {
        assert_eq!(group.count, group.row_ids.len(), "{}", group.label);
        for id in &group.row_ids {
            assert!(data.rows.contains_key(id), "{id} in {}", group.label);
        }
        seen += group.count;
    }
    assert_eq!(seen, data.rows.len());
}

mod grouping {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_space_view_by_status_orders_groups_by_style_and_collapses_only_done() {
        let site = Site::new().await;

        let data = site.nw_data(Narrowing::default()).await;

        assert_eq!(data.group_by, GroupBy::Status);
        assert_consistent(&data);
        let styles: Vec<Style> = data
            .groups
            .iter()
            .map(|g| style_of(g).expect("a status mark"))
            .collect();
        assert!(styles.windows(2).all(|w| w[0] <= w[1]), "{styles:?}");
        let mut distinct = styles.clone();
        distinct.dedup();
        assert_eq!(
            distinct,
            [
                Style::Progress,
                Style::Review,
                Style::Blocked,
                Style::Todo,
                Style::Done
            ]
        );
        for group in &data.groups {
            assert_eq!(
                group.collapsed_by_default,
                style_of(group) == Some(Style::Done),
                "{}",
                group.label
            );
        }
        assert!(data.groups.iter().any(|g| g.collapsed_by_default));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_groups_of_a_space_hold_every_visible_issue_of_the_project() {
        let site = Site::new().await;

        let data = site.nw_data(Narrowing::default()).await;

        let in_cache = number(
            site.db(),
            format!("SELECT COUNT(*) FROM cache.issue WHERE project_id = '{NW}' AND gone = 0"),
        )
        .await;
        let in_groups: usize = data.groups.iter().map(|g| g.count).sum();
        assert_eq!(in_groups as i64, in_cache);
        assert_eq!(data.rows.len() as i64, in_cache);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn everything_by_project_lists_the_tracked_spaces_in_space_order() {
        let site = Site::new().await;
        let everything = site.everything().await;
        let view = site.first_view(everything).await;

        let data = site.data(everything, view, Narrowing::default()).await;

        assert_eq!(data.group_by, GroupBy::Project);
        assert_consistent(&data);
        let tree = site.tree().await;
        let keys: Vec<&str> = data.groups.iter().map(|g| g.key.as_str()).collect();
        assert_eq!(keys, PROJECT_IDS);
        let labels: Vec<&str> = data.groups.iter().map(|g| g.label.as_str()).collect();
        let names: Vec<&str> = tree.spaces.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(labels, names);
        assert!(
            data.groups
                .iter()
                .all(|g| matches!(g.mark, Some(GroupMark::Space { .. })))
        );
        assert!(data.groups.iter().all(|g| !g.collapsed_by_default));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn every_grouping_splits_everything_and_puts_the_missing_value_group_last() {
        let site = Site::new().await;
        let everything = site.everything().await;
        let view = site.first_view(everything).await;

        for (by, none_key, required) in [
            (GroupBy::Due, "none", true),
            (GroupBy::Epic, "", true),
            (GroupBy::Release, "", true),
            (GroupBy::Assignee, "", false),
            (GroupBy::Priority, "", false),
            (GroupBy::Client, "", false),
            (GroupBy::Project, "", false),
            (GroupBy::Status, "", false),
            (GroupBy::None, "", false),
        ] {
            view_set_group(site.db(), view, by).await.unwrap();

            let data = site.data(everything, view, Narrowing::default()).await;

            assert_eq!(data.group_by, by, "{by:?}");
            assert!(!data.groups.is_empty(), "{by:?}");
            assert_consistent(&data);
            let position = data.groups.iter().position(|g| g.key == none_key);
            if by == GroupBy::None {
                assert_eq!(data.groups.len(), 1, "{by:?}");
            } else if let Some(position) = position {
                assert_eq!(position, data.groups.len() - 1, "{by:?}");
            } else {
                assert!(!required, "{by:?} has no group for a missing value");
            }
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn grouping_by_epic_marks_each_group_with_its_epic_colour() {
        let site = Site::new().await;
        let (space, view) = site.nw().await;
        view_set_group(site.db(), view, GroupBy::Epic)
            .await
            .unwrap();

        let data = site.data(space, view, Narrowing::default()).await;

        let checkout = data
            .groups
            .iter()
            .find(|g| g.label == "Checkout")
            .expect("the Checkout epic group");
        assert_eq!(checkout.mark, Some(GroupMark::Epic { color: 1 }));
        assert_eq!(checkout.key, site.issue_id("NW-1020").await);
        assert!(group_keys(&data, checkout).contains(&"NW-1042".to_string()));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn grouping_by_assignee_puts_me_first() {
        let site = Site::new().await;
        let (space, view) = site.nw().await;
        view_set_group(site.db(), view, GroupBy::Assignee)
            .await
            .unwrap();

        let data = site.data(space, view, Narrowing::default()).await;

        assert_eq!(data.groups[0].key, ME_ACCOUNT_ID);
        assert_eq!(data.groups[0].label, "Alex Morgan");
    }
}

mod rows {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_row_shows_its_epic_assignee_release_and_priority_resolved() {
        let site = Site::new().await;

        let data = site.nw_data(Narrowing::default()).await;

        let row = row_of(&data, "NW-1042");
        assert_eq!(
            row.summary,
            "Checkout: address validation for DACH postcodes"
        );
        let epic = row.epic.as_ref().expect("an epic chip");
        assert_eq!(epic.name, "Checkout");
        assert_eq!(epic.id, site.issue_id("NW-1020").await);
        assert_eq!(epic.color, 1);
        assert_eq!(row.assignees.len(), 1);
        assert_eq!(row.assignees[0].account_id, ME_ACCOUNT_ID);
        assert_eq!(row.assignees[0].name, "Alex Morgan");
        assert_eq!(row.assignees[0].initials, "AM");
        assert_eq!(row.assignees[0].color, 1);
        assert_eq!(row.release.as_deref(), Some("2.4.0"));
        let priority = row.priority.as_ref().expect("a priority");
        assert_eq!((priority.name.as_str(), priority.level), ("High", 2));
        assert_eq!(row.due.as_deref(), Some("Oct 30"));
        assert_eq!(row.style, Style::Progress);
        assert_eq!(row.status_name, "In Progress");
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_highest_priority_is_level_one_and_epic_chips_use_the_five_tints() {
        let site = Site::new().await;

        let data = site.nw_data(Narrowing::default()).await;

        let highest = row_of(&data, "NW-1049").priority.clone().unwrap();
        assert_eq!((highest.name.as_str(), highest.level), ("Highest", 1));
        for row in data.rows.values() {
            if let Some(epic) = &row.epic {
                assert!((1..=5).contains(&epic.color), "{}", row.key);
            }
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn nobody_is_marked_changed_before_change_tracking_exists() {
        let site = Site::new().await;

        let data = site.nw_data(Narrowing::default()).await;

        assert!(data.rows.values().all(|row| !row.changed));
        assert!(data.rows.values().all(|row| row.change_note.is_none()));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_view_of_another_list_is_not_found() {
        let site = Site::new().await;
        let (_, view) = site.nw().await;
        let everything = site.everything().await;

        let err = view_rows(site.db(), viewer(), everything, view, Narrowing::default())
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_unknown_view_is_not_found() {
        let site = Site::new().await;
        let (space, _) = site.nw().await;

        let err = view_rows(site.db(), viewer(), space, 99_999, Narrowing::default())
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }
}

mod sorting {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn by_key_ascending_orders_numerically_inside_each_group() {
        let site = Site::new().await;
        let (space, view) = site.nw().await;
        view_set_sort(
            site.db(),
            view,
            Sort {
                by: SortBy::Key,
                descending: false,
            },
        )
        .await
        .unwrap();

        let data = site.data(space, view, Narrowing::default()).await;

        for group in &data.groups {
            let numbers: Vec<u64> = group_keys(&data, group)
                .iter()
                .map(|k| issue_number(k))
                .collect();
            assert!(
                numbers.windows(2).all(|w| w[0] < w[1]),
                "{}: {numbers:?}",
                group.label
            );
        }
        let done = data.groups.iter().find(|g| g.label == "Done").unwrap();
        assert_eq!(group_keys(&data, done)[0], "NW-977");
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn by_updated_descending_puts_the_latest_change_first_inside_each_group() {
        let site = Site::new().await;
        let (space, view) = site.nw().await;
        view_set_sort(
            site.db(),
            view,
            Sort {
                by: SortBy::Updated,
                descending: true,
            },
        )
        .await
        .unwrap();
        let updated: HashMap<String, String> =
            query(site.db(), "SELECT id, updated_at FROM cache.issue", |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .await
            .into_iter()
            .collect();

        let data = site.data(space, view, Narrowing::default()).await;

        for group in &data.groups {
            let stamps: Vec<&String> = group.row_ids.iter().map(|id| &updated[id]).collect();
            assert!(stamps.windows(2).all(|w| w[0] >= w[1]), "{}", group.label);
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn two_calls_return_the_same_order() {
        let site = Site::new().await;
        let (space, view) = site.nw().await;
        view_set_sort(
            site.db(),
            view,
            Sort {
                by: SortBy::Priority,
                descending: false,
            },
        )
        .await
        .unwrap();

        let first = site.data(space, view, Narrowing::default()).await;
        let second = site.data(space, view, Narrowing::default()).await;

        assert_eq!(first, second);
    }
}

mod narrowing {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn me_keeps_only_the_issues_assigned_to_me() {
        let site = Site::new().await;

        let data = site
            .nw_data(Narrowing {
                me: true,
                ..Narrowing::default()
            })
            .await;

        let mine = number(
            site.db(),
            format!(
                "SELECT COUNT(*) FROM cache.issue
                 WHERE project_id = '{NW}' AND gone = 0 AND assignee_id = '{ME_ACCOUNT_ID}'"
            ),
        )
        .await;
        assert!(mine > 0);
        assert_eq!(data.rows.len() as i64, mine);
        assert_consistent(&data);
        assert!(
            data.rows
                .values()
                .all(|row| row.assignees.first().map(|p| p.account_id.as_str())
                    == Some(ME_ACCOUNT_ID))
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn text_ignores_case_and_accents() {
        let site = Site::new().await;
        site.db()
            .write(|tx, _| {
                tx.execute(
                    "UPDATE cache.issue SET summary = 'Neue Säule im Footer' WHERE key = 'NW-1042'",
                    [],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        for needle in ["saule", "SAULE", "säule", "SÄULE", "  Säule "] {
            let data = site.nw_data(narrowing_text(needle)).await;

            assert_eq!(sorted_keys(&data), ["NW-1042"], "{needle}");
            assert_consistent(&data);
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn text_also_matches_the_key() {
        let site = Site::new().await;

        let data = site.nw_data(narrowing_text("nw-104")).await;

        let expected = strings(
            site.db(),
            format!(
                "SELECT key FROM cache.issue
                 WHERE project_id = '{NW}' AND gone = 0 AND key LIKE '%NW-104%' ORDER BY key"
            ),
        )
        .await;
        assert!(expected.len() > 1);
        assert_eq!(sorted_keys(&data), expected);
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn text_that_matches_nothing_gives_no_groups() {
        let site = Site::new().await;

        let data = site.nw_data(narrowing_text("zzzz-no-such-text")).await;

        assert!(data.groups.is_empty());
        assert!(data.rows.is_empty());
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn an_epic_keeps_only_its_children() {
        let site = Site::new().await;
        let epic = site.issue_id("NW-1020").await;

        let data = site
            .nw_data(Narrowing {
                epic: Some(epic.clone()),
                ..Narrowing::default()
            })
            .await;

        let children = strings(
            site.db(),
            format!(
                "SELECT key FROM cache.issue
                 WHERE project_id = '{NW}' AND gone = 0 AND parent_id = '{epic}' ORDER BY key"
            ),
        )
        .await;
        assert!(!children.is_empty());
        assert_eq!(sorted_keys(&data), children);
        assert!(
            data.rows
                .values()
                .all(|row| row.epic.as_ref().map(|e| e.id.as_str()) == Some(epic.as_str()))
        );
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_narrowings_combine() {
        let site = Site::new().await;
        let epic = site.issue_id("NW-1020").await;

        let data = site
            .nw_data(Narrowing {
                me: true,
                text: "checkout".into(),
                epic: Some(epic),
            })
            .await;

        assert!(!data.rows.is_empty());
        for row in data.rows.values() {
            assert_eq!(row.assignees[0].account_id, ME_ACCOUNT_ID, "{}", row.key);
            assert_eq!(row.epic.as_ref().unwrap().name, "Checkout", "{}", row.key);
        }
    }
}

mod custom_columns {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_user_field_is_a_person_cell_and_a_number_field_is_text() {
        let site = Site::new().await;
        let (space, view) = site.nw().await;
        view_set_columns(
            site.db(),
            view,
            vec!["assignee".into(), REVIEWER.into(), STORY_POINTS.into()],
        )
        .await
        .unwrap();
        site.sync().await;
        let reviewer = query(
            site.db(),
            format!(
                "SELECT json_extract(raw, '$.fields.{REVIEWER}.accountId')
                 FROM cache.issue WHERE key = 'NW-1042'"
            ),
            |row| row.get::<_, Option<String>>(0),
        )
        .await
        .remove(0)
        .expect("the sync fetched the reviewer field");

        let data = site.data(space, view, Narrowing::default()).await;

        assert_eq!(data.columns, ["assignee", REVIEWER, STORY_POINTS]);
        let row = row_of(&data, "NW-1042");
        match &row.custom[REVIEWER] {
            Cell::Person(person) => assert_eq!(person.account_id, reviewer),
            other => panic!("expected a person, got {other:?}"),
        }
        match &row.custom[STORY_POINTS] {
            Cell::Text(points) => assert_eq!(points.parse::<f64>().unwrap(), 5.0, "{points}"),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_field_without_a_value_has_no_cell() {
        let site = Site::new().await;
        let (space, view) = site.nw().await;
        view_set_columns(site.db(), view, vec![REVIEWER.into(), STORY_POINTS.into()])
            .await
            .unwrap();
        site.sync().await;

        let data = site.data(space, view, Narrowing::default()).await;

        let row = row_of(&data, "NW-1049");
        assert!(!row.custom.contains_key(REVIEWER));
        assert!(row.custom.contains_key(STORY_POINTS));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn custom_cells_appear_only_for_the_columns_the_view_shows() {
        let site = Site::new().await;

        let data = site.nw_data(Narrowing::default()).await;

        assert!(data.rows.values().all(|row| row.custom.is_empty()));
    }
}

mod status_styles {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn a_chosen_style_moves_the_status_into_that_group_and_automatic_restores_it() {
        let site = Site::new().await;
        let status = text(
            site.db(),
            "SELECT status_id FROM cache.issue WHERE key = 'NW-1038'",
        )
        .await;
        let group_of = |data: &ViewData| -> Group {
            data.groups
                .iter()
                .find(|g| g.key == status)
                .expect("the status group")
                .clone()
        };
        let before = group_of(&site.nw_data(Narrowing::default()).await);
        assert_eq!(style_of(&before), Some(Style::Review));

        let rows = status_style_set(site.db(), status.clone(), Some(Style::Blocked))
            .await
            .unwrap();
        let chosen = site.nw_data(Narrowing::default()).await;

        let row = rows.iter().find(|r| r.id == status).unwrap();
        assert_eq!(row.chosen, Some(Style::Blocked));
        assert_eq!(row.detected, Style::Review);
        let moved = group_of(&chosen);
        assert_eq!(
            moved.mark,
            Some(GroupMark::Status {
                style: Style::Blocked
            })
        );
        assert_eq!(moved.row_ids, before.row_ids);
        assert_eq!(row_of(&chosen, "NW-1038").style, Style::Blocked);
        let listed = status_styles_list(site.db()).await.unwrap();
        assert_eq!(
            listed.iter().find(|r| r.id == status).unwrap().chosen,
            Some(Style::Blocked)
        );
        assert_eq!(listed.iter().filter(|r| r.chosen.is_some()).count(), 1);

        status_style_set(site.db(), status.clone(), None)
            .await
            .unwrap();
        let restored = group_of(&site.nw_data(Narrowing::default()).await);

        assert_eq!(restored, before);
        let listed = status_styles_list(site.db()).await.unwrap();
        assert!(listed.iter().all(|r| r.chosen.is_none()));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn styling_an_unknown_status_is_not_found() {
        let site = Site::new().await;

        let err = status_style_set(site.db(), "99999".into(), Some(Style::Done))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }
}

mod catalogue {
    use super::*;

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn the_standard_columns_come_first_then_the_custom_fields() {
        let site = Site::new().await;

        let columns = columns_catalogue(site.db()).await.unwrap();

        let ids: Vec<&str> = columns.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids[..8],
            [
                "status", "assignee", "epic", "release", "priority", "labels", "sprint", "due"
            ]
        );
        assert!(columns[..8].iter().all(|c| !c.custom));
        assert!(columns[8..].iter().all(|c| c.custom));
        let reviewer = columns.iter().find(|c| c.id == REVIEWER).unwrap();
        assert_eq!(reviewer.name, "Reviewer");
        assert!(reviewer.custom);
        assert!(ids.contains(&STORY_POINTS));
    }

    #[tokio::test]
    #[ignore = "syncs the demo site into a temp database"]
    async fn internal_fields_and_the_sprint_field_are_left_out() {
        let site = Site::new().await;

        let columns = columns_catalogue(site.db()).await.unwrap();

        for id in [
            "customfield_10020",
            "customfield_10017",
            "customfield_10019",
        ] {
            assert!(columns.iter().all(|c| c.id != id), "{id}");
        }
    }
}
