//! Filter conditions compiled to SQL and run against a small hand-made mirror in a temp folder: which issue
//! keys each field and operator matches, how `me` and dates resolve, and which conditions are refused.

use std::collections::HashMap;

use chrono::NaiveDate;
use rusqlite::params_from_iter;
use serde_json::{Value as Json, json};
use tempfile::TempDir;

use taskboard_lib::db::Db;
use taskboard_lib::error::{AppResult, ErrorKind};
use taskboard_lib::filter::{Condition, Ctx, CustomField, Field, Op, Value, compile};
use taskboard_lib::test_support::temp_config_dir;

const ALEX: &str = "acc-alex";

type Case = (Condition, Vec<&'static str>);

struct Fixture {
    _dir: TempDir,
    db: Db,
    custom: HashMap<String, CustomField>,
}

const ISSUE_DEFAULTS: [(&str, &str); 10] = [
    ("project_id", "'p1'"),
    ("summary", "'Demo issue'"),
    ("issue_type", "'Task'"),
    ("status_id", "'10001'"),
    ("status_category", "'new'"),
    ("created_at", "'2026-01-01T00:00:00.000Z'"),
    ("updated_at", "'2026-01-01T00:00:00.000Z'"),
    ("raw", "'{\"fields\":{}}'"),
    ("synced_at", "'2026-10-28T08:00:00.000Z'"),
    ("gone", "0"),
];

fn raw(fields: Json) -> String {
    format!(
        "'{}'",
        json!({ "fields": fields }).to_string().replace('\'', "''")
    )
}

fn issue(key: &str, set: &[(&str, &str)]) -> String {
    let mut columns: Vec<(String, String)> = ISSUE_DEFAULTS
        .iter()
        .map(|(c, v)| (c.to_string(), v.to_string()))
        .collect();
    for (column, value) in set {
        match columns.iter_mut().find(|(name, _)| name == column) {
            Some(slot) => slot.1 = value.to_string(),
            None => columns.push((column.to_string(), value.to_string())),
        }
    }
    let names: Vec<&str> = columns.iter().map(|(c, _)| c.as_str()).collect();
    let values: Vec<&str> = columns.iter().map(|(_, v)| v.as_str()).collect();
    format!(
        "INSERT INTO cache.issue (id, key, {}) VALUES ('{key}', '{key}', {});",
        names.join(", "),
        values.join(", ")
    )
}

fn fixture_sql() -> String {
    let nw1 = raw(json!({
        "customfield_10071": {"accountId": "acc-sam"},
        "customfield_10072": {"value": "Gold"},
        "customfield_10073": "Needs 100% review_ok",
        "customfield_10074": 5,
        "customfield_10075": ["red", "blue"],
        "customfield_10076": "2026-11-02",
        "customfield_10077": "2026-10-28T09:30:00.000+0100",
        "customfield_10078": [{"value": "Web"}, {"value": "API"}],
    }));
    let nw2 = raw(json!({
        "customfield_10071": {"accountId": "acc-alex"},
        "customfield_10072": {"value": "Silver"},
        "customfield_10073": "plain",
        "customfield_10074": 12,
        "customfield_10075": ["blue"],
        "customfield_10076": "2026-10-28",
        "customfield_10077": "2026-10-29T08:00:00.000+0100",
        "customfield_10078": [{"name": "Web"}],
    }));
    let nw4 = raw(json!({
        "customfield_10074": 5,
        "customfield_10075": [],
        "customfield_10078": [{"id": "77"}],
    }));
    let nw5 = raw(json!({"customfield_10073": "100 done"}));
    let mut sql = vec![
        "INSERT INTO cache.space (id, key, name, raw) VALUES ('p1', 'NW', 'Northwind', '{}'), \
         ('p2', 'OLD', 'Legacy', '{}');"
            .to_string(),
        "INSERT OR REPLACE INTO space (project_id, tracked) VALUES ('p1', 1), ('p2', 0);".into(),
        "INSERT INTO cache.field (id, name, type) VALUES \
         ('customfield_10071', 'Reviewer', 'user'), ('customfield_10072', 'Tier', 'option'), \
         ('customfield_10073', 'Notes', 'string'), ('customfield_10074', 'Story points', 'number'), \
         ('customfield_10075', 'Colors', 'array'), ('customfield_10076', 'Handover', 'date'), \
         ('customfield_10077', 'Kickoff', 'datetime'), ('customfield_10078', 'Components', 'array'), \
         ('customfield_10079', 'Sketch', 'any');"
            .into(),
    ];
    sql.push(issue(
        "NW-1",
        &[
            ("issue_type", "'Bug'"),
            ("priority", "'High'"),
            ("assignee_id", "'acc-alex'"),
            ("reporter_id", "'acc-sam'"),
            ("parent_id", "'e1'"),
            ("fix_versions", "'[\"v1\",\"v2\"]'"),
            ("labels", "'[\"backend\",\"urgent\"]'"),
            ("sprint_id", "'7'"),
            ("sprint_state", "'active'"),
            ("due_date", "'2026-10-28'"),
            ("created_at", "'2026-10-20T10:00:00.000Z'"),
            ("updated_at", "'2026-10-27T22:59:59.999Z'"),
            ("raw", nw1.as_str()),
        ],
    ));
    sql.push(issue(
        "NW-2",
        &[
            ("issue_type", "'Story'"),
            ("status_id", "'10002'"),
            ("status_category", "'indeterminate'"),
            ("priority", "'Low'"),
            ("assignee_id", "'acc-sam'"),
            ("reporter_id", "'acc-alex'"),
            ("parent_id", "'e2'"),
            ("fix_versions", "'[\"v2\"]'"),
            ("labels", "'[\"frontend\"]'"),
            ("sprint_id", "'8'"),
            ("sprint_state", "'future'"),
            ("due_date", "'2026-10-30'"),
            ("created_at", "'2026-10-27T22:59:59.999Z'"),
            ("updated_at", "'2026-10-27T23:00:00.000Z'"),
            ("raw", nw2.as_str()),
        ],
    ));
    sql.push(issue(
        "NW-3",
        &[
            ("status_id", "'10003'"),
            ("status_category", "'done'"),
            ("reporter_id", "'acc-sam'"),
            ("sprint_id", "'6'"),
            ("sprint_state", "'closed'"),
            ("created_at", "'2026-10-27T23:00:00.000Z'"),
            ("updated_at", "'2026-10-20T12:00:00.000Z'"),
        ],
    ));
    sql.push(issue(
        "NW-4",
        &[
            ("assignee_id", "'acc-alex'"),
            ("fix_versions", "'[]'"),
            ("labels", "'[]'"),
            ("due_date", "'2026-11-04'"),
            ("updated_at", "'2026-11-01T12:00:00.000Z'"),
            ("raw", nw4.as_str()),
        ],
    ));
    sql.push(issue(
        "NW-5",
        &[
            ("status_id", "'10002'"),
            ("status_category", "'indeterminate'"),
            ("priority", "'High'"),
            ("reporter_id", "'acc-sam'"),
            ("parent_id", "'e1'"),
            ("fix_versions", "'[\"v1\"]'"),
            ("labels", "'[\"backend\"]'"),
            ("sprint_id", "'9'"),
            ("sprint_state", "'active'"),
            ("due_date", "'2026-10-21'"),
            ("updated_at", "'2026-10-21T08:00:00.000Z'"),
            ("raw", nw5.as_str()),
        ],
    ));
    for (key, extra) in [("OLD-1", ("project_id", "'p2'")), ("NW-9", ("gone", "1"))] {
        sql.push(issue(
            key,
            &[
                extra,
                ("assignee_id", "'acc-alex'"),
                ("priority", "'High'"),
                ("labels", "'[\"backend\"]'"),
                ("due_date", "'2026-10-28'"),
                ("sprint_id", "'7'"),
                ("sprint_state", "'active'"),
            ],
        ));
    }
    sql.join("\n")
}

impl Fixture {
    async fn new() -> Self {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        let sql = fixture_sql();
        db.write(move |tx, _| {
            tx.execute_batch(&sql)?;
            Ok(())
        })
        .await
        .unwrap();
        let custom = db
            .read(|conn| {
                let mut stmt = conn.prepare("SELECT id, name, type FROM cache.field")?;
                let rows = stmt
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            CustomField {
                                name: r.get(1)?,
                                kind: r.get(2)?,
                            },
                        ))
                    })?
                    .collect::<Result<HashMap<_, _>, _>>()?;
                Ok(rows)
            })
            .await
            .unwrap();
        Self {
            _dir: dir,
            db,
            custom,
        }
    }

    fn ctx(&self, me: Option<&str>) -> Ctx {
        Ctx {
            me: me.map(str::to_string),
            today: NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(),
            tz: chrono_tz::Europe::Berlin,
            custom: self.custom.clone(),
        }
    }

    async fn matching(&self, conditions: &[Condition], me: Option<&str>) -> AppResult<Vec<String>> {
        let compiled = compile(conditions, &self.ctx(me))?;
        let sql = format!(
            "SELECT i.key FROM cache.issue i WHERE {} ORDER BY i.key",
            compiled.where_clause()
        );
        self.db
            .read(move |conn| {
                let mut stmt = conn.prepare(&sql)?;
                let keys = stmt
                    .query_map(params_from_iter(&compiled.params), |r| {
                        r.get::<_, String>(0)
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(keys)
            })
            .await
    }

    async fn check(&self, me: Option<&str>, cases: Vec<Case>) {
        for (condition, expected) in cases {
            let got = self
                .matching(std::slice::from_ref(&condition), me)
                .await
                .unwrap_or_else(|e| panic!("{condition:?}: {e}"));
            assert_eq!(got, expected, "{condition:?}");
        }
    }

    fn assert_refused(&self, conditions: Vec<Condition>) {
        for condition in conditions {
            let err = compile(std::slice::from_ref(&condition), &self.ctx(Some(ALEX)))
                .expect_err(&format!("{condition:?} should be refused"));
            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{condition:?}");
        }
    }
}

fn custom(id: u32) -> Field {
    Field::Custom(format!("customfield_{id}"))
}

fn one(value: &str) -> Option<Value> {
    Some(Value::One(value.to_string()))
}

fn many(values: &[&str]) -> Option<Value> {
    Some(Value::Many(values.iter().map(|v| v.to_string()).collect()))
}

fn days(count: u32) -> Option<Value> {
    Some(Value::Days(count))
}

fn case(field: Field, op: Op, value: Option<Value>, keys: &[&'static str]) -> Case {
    (Condition::new(field, op, value), keys.to_vec())
}

const ALL: [&str; 5] = ["NW-1", "NW-2", "NW-3", "NW-4", "NW-5"];

#[tokio::test]
async fn no_conditions_match_every_tracked_visible_issue() {
    let fx = Fixture::new().await;
    assert_eq!(fx.matching(&[], Some(ALEX)).await.unwrap(), ALL);
}

#[tokio::test]
async fn untracked_projects_and_gone_issues_never_match() {
    let fx = Fixture::new().await;
    fx.check(
        Some(ALEX),
        vec![
            case(Field::Status, Op::Is, one("10001"), &["NW-1", "NW-4"]),
            case(Field::Assignee, Op::Is, one("me"), &["NW-1", "NW-4"]),
            case(Field::Project, Op::Is, one("p2"), &[]),
            case(Field::Project, Op::Is, one("p1"), &ALL),
            case(Field::Labels, Op::Is, one("backend"), &["NW-1", "NW-5"]),
        ],
    )
    .await;
}

#[tokio::test]
async fn status_conditions_match_by_status_id() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(Field::Status, Op::Is, one("10001"), &["NW-1", "NW-4"]),
            case(
                Field::Status,
                Op::IsNot,
                one("10001"),
                &["NW-2", "NW-3", "NW-5"],
            ),
            case(
                Field::Status,
                Op::In,
                many(&["10002", "10003"]),
                &["NW-2", "NW-3", "NW-5"],
            ),
            case(
                Field::Status,
                Op::NotIn,
                many(&["10002", "10003"]),
                &["NW-1", "NW-4"],
            ),
            case(Field::Status, Op::In, many(&[]), &[]),
            case(Field::Status, Op::NotIn, many(&[]), &ALL),
            case(Field::Status, Op::In, one("10003"), &["NW-3"]),
            case(Field::Status, Op::IsEmpty, None, &[]),
            case(Field::Status, Op::IsNotEmpty, None, &ALL),
        ],
    )
    .await;
}

#[tokio::test]
async fn status_category_type_and_epic_conditions_match_their_columns() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(Field::StatusCategory, Op::Is, one("done"), &["NW-3"]),
            case(
                Field::StatusCategory,
                Op::In,
                many(&["new", "indeterminate"]),
                &["NW-1", "NW-2", "NW-4", "NW-5"],
            ),
            case(Field::Type, Op::Is, one("Bug"), &["NW-1"]),
            case(Field::Type, Op::IsNot, one("Task"), &["NW-1", "NW-2"]),
            case(Field::Epic, Op::Is, one("e1"), &["NW-1", "NW-5"]),
            case(Field::Epic, Op::In, many(&["e2"]), &["NW-2"]),
            case(Field::Epic, Op::IsEmpty, None, &["NW-3", "NW-4"]),
            case(Field::Epic, Op::IsNotEmpty, None, &["NW-1", "NW-2", "NW-5"]),
        ],
    )
    .await;
}

#[tokio::test]
async fn priority_conditions_treat_a_missing_priority_as_empty() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(Field::Priority, Op::Is, one("High"), &["NW-1", "NW-5"]),
            case(
                Field::Priority,
                Op::IsNot,
                one("High"),
                &["NW-2", "NW-3", "NW-4"],
            ),
            case(Field::Priority, Op::In, many(&["Low"]), &["NW-2"]),
            case(
                Field::Priority,
                Op::NotIn,
                many(&["High"]),
                &["NW-2", "NW-3", "NW-4"],
            ),
            case(Field::Priority, Op::IsEmpty, None, &["NW-3", "NW-4"]),
            case(
                Field::Priority,
                Op::IsNotEmpty,
                None,
                &["NW-1", "NW-2", "NW-5"],
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn assignee_and_reporter_resolve_me_to_the_connected_account() {
    let fx = Fixture::new().await;
    fx.check(
        Some(ALEX),
        vec![
            case(Field::Assignee, Op::Is, one("me"), &["NW-1", "NW-4"]),
            case(Field::Assignee, Op::Is, one("acc-sam"), &["NW-2"]),
            case(
                Field::Assignee,
                Op::IsNot,
                one("me"),
                &["NW-2", "NW-3", "NW-5"],
            ),
            case(
                Field::Assignee,
                Op::In,
                many(&["me", "acc-sam"]),
                &["NW-1", "NW-2", "NW-4"],
            ),
            case(
                Field::Assignee,
                Op::NotIn,
                many(&["me"]),
                &["NW-2", "NW-3", "NW-5"],
            ),
            case(Field::Assignee, Op::IsEmpty, None, &["NW-3", "NW-5"]),
            case(
                Field::Assignee,
                Op::IsNotEmpty,
                None,
                &["NW-1", "NW-2", "NW-4"],
            ),
            case(Field::Reporter, Op::Is, one("me"), &["NW-2"]),
            case(
                Field::Reporter,
                Op::Is,
                one("acc-sam"),
                &["NW-1", "NW-3", "NW-5"],
            ),
            case(Field::Reporter, Op::IsEmpty, None, &["NW-4"]),
        ],
    )
    .await;
}

#[tokio::test]
async fn me_matches_nothing_without_a_connected_account() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(Field::Assignee, Op::Is, one("me"), &[]),
            case(Field::Assignee, Op::In, many(&["me"]), &[]),
            case(Field::Reporter, Op::Is, one("me"), &[]),
            case(custom(10071), Op::Is, one("me"), &[]),
            case(Field::Assignee, Op::Is, one("acc-sam"), &["NW-2"]),
        ],
    )
    .await;
}

#[tokio::test]
async fn release_conditions_look_into_the_fix_version_ids() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(Field::Release, Op::Is, one("v1"), &["NW-1", "NW-5"]),
            case(Field::Release, Op::Is, one("v2"), &["NW-1", "NW-2"]),
            case(
                Field::Release,
                Op::IsNot,
                one("v2"),
                &["NW-3", "NW-4", "NW-5"],
            ),
            case(
                Field::Release,
                Op::In,
                many(&["v1", "v3"]),
                &["NW-1", "NW-5"],
            ),
            case(
                Field::Release,
                Op::NotIn,
                many(&["v1"]),
                &["NW-2", "NW-3", "NW-4"],
            ),
            case(Field::Release, Op::IsEmpty, None, &["NW-3", "NW-4"]),
            case(
                Field::Release,
                Op::IsNotEmpty,
                None,
                &["NW-1", "NW-2", "NW-5"],
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn label_conditions_look_into_the_label_list() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(Field::Labels, Op::Is, one("backend"), &["NW-1", "NW-5"]),
            case(
                Field::Labels,
                Op::IsNot,
                one("backend"),
                &["NW-2", "NW-3", "NW-4"],
            ),
            case(
                Field::Labels,
                Op::In,
                many(&["urgent", "frontend"]),
                &["NW-1", "NW-2"],
            ),
            case(
                Field::Labels,
                Op::NotIn,
                many(&["backend"]),
                &["NW-2", "NW-3", "NW-4"],
            ),
            case(
                Field::Labels,
                Op::Contains,
                one("end"),
                &["NW-1", "NW-2", "NW-5"],
            ),
            case(Field::Labels, Op::Contains, one("GENT"), &["NW-1"]),
            case(Field::Labels, Op::IsEmpty, None, &["NW-3", "NW-4"]),
            case(
                Field::Labels,
                Op::IsNotEmpty,
                None,
                &["NW-1", "NW-2", "NW-5"],
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn sprint_conditions_match_the_sprint_state_or_id() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(Field::Sprint, Op::Is, one("active"), &["NW-1", "NW-5"]),
            case(Field::Sprint, Op::Is, one("8"), &["NW-2"]),
            case(
                Field::Sprint,
                Op::IsNot,
                one("active"),
                &["NW-2", "NW-3", "NW-4"],
            ),
            case(
                Field::Sprint,
                Op::IsNot,
                one("7"),
                &["NW-2", "NW-3", "NW-4", "NW-5"],
            ),
            case(
                Field::Sprint,
                Op::In,
                many(&["future", "closed"]),
                &["NW-2", "NW-3"],
            ),
            case(
                Field::Sprint,
                Op::In,
                many(&["active", "8"]),
                &["NW-1", "NW-2", "NW-5"],
            ),
            case(
                Field::Sprint,
                Op::NotIn,
                many(&["active"]),
                &["NW-2", "NW-3", "NW-4"],
            ),
            case(
                Field::Sprint,
                Op::NotIn,
                many(&["closed", "9"]),
                &["NW-1", "NW-2", "NW-4"],
            ),
            case(Field::Sprint, Op::In, many(&[]), &[]),
            case(Field::Sprint, Op::NotIn, many(&[]), &ALL),
            case(Field::Sprint, Op::IsEmpty, None, &["NW-4"]),
            case(
                Field::Sprint,
                Op::IsNotEmpty,
                None,
                &["NW-1", "NW-2", "NW-3", "NW-5"],
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn due_date_conditions_compare_whole_days() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(Field::Due, Op::Before, one("2026-10-28"), &["NW-5"]),
            case(Field::Due, Op::After, one("2026-10-28"), &["NW-2", "NW-4"]),
            case(Field::Due, Op::After, one("2026-10-29"), &["NW-2", "NW-4"]),
            case(Field::Due, Op::WithinLastDays, days(7), &["NW-1", "NW-5"]),
            case(Field::Due, Op::WithinLastDays, days(0), &["NW-1"]),
            case(Field::Due, Op::WithinNextDays, days(2), &["NW-1", "NW-2"]),
            case(Field::Due, Op::WithinNextDays, days(0), &["NW-1"]),
            case(
                Field::Due,
                Op::WithinNextDays,
                days(7),
                &["NW-1", "NW-2", "NW-4"],
            ),
            case(Field::Due, Op::IsEmpty, None, &["NW-3"]),
            case(
                Field::Due,
                Op::IsNotEmpty,
                None,
                &["NW-1", "NW-2", "NW-4", "NW-5"],
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn updated_conditions_split_days_at_local_midnight() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(
                Field::Updated,
                Op::Before,
                one("2026-10-28"),
                &["NW-1", "NW-3", "NW-5"],
            ),
            case(
                Field::Updated,
                Op::After,
                one("2026-10-27"),
                &["NW-2", "NW-4"],
            ),
            case(Field::Updated, Op::After, one("2026-10-28"), &["NW-4"]),
            case(Field::Updated, Op::WithinLastDays, days(0), &["NW-2"]),
            case(
                Field::Updated,
                Op::WithinLastDays,
                days(7),
                &["NW-1", "NW-2", "NW-5"],
            ),
            case(
                Field::Updated,
                Op::WithinNextDays,
                days(5),
                &["NW-2", "NW-4"],
            ),
            case(Field::Updated, Op::IsEmpty, None, &[]),
            case(Field::Updated, Op::IsNotEmpty, None, &ALL),
        ],
    )
    .await;
}

#[tokio::test]
async fn created_conditions_split_days_at_local_midnight() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(Field::Created, Op::After, one("2026-10-27"), &["NW-3"]),
            case(
                Field::Created,
                Op::Before,
                one("2026-10-28"),
                &["NW-1", "NW-2", "NW-4", "NW-5"],
            ),
            case(
                Field::Created,
                Op::WithinLastDays,
                days(1),
                &["NW-2", "NW-3"],
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn custom_user_fields_match_the_account_id() {
    let fx = Fixture::new().await;
    fx.check(
        Some(ALEX),
        vec![
            case(custom(10071), Op::Is, one("me"), &["NW-2"]),
            case(custom(10071), Op::Is, one("acc-sam"), &["NW-1"]),
            case(custom(10071), Op::IsEmpty, None, &["NW-3", "NW-4", "NW-5"]),
            case(custom(10071), Op::IsNotEmpty, None, &["NW-1", "NW-2"]),
        ],
    )
    .await;
}

#[tokio::test]
async fn custom_option_fields_match_the_option_value() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(custom(10072), Op::Is, one("Gold"), &["NW-1"]),
            case(
                custom(10072),
                Op::In,
                many(&["Gold", "Silver"]),
                &["NW-1", "NW-2"],
            ),
            case(
                custom(10072),
                Op::NotIn,
                many(&["Gold"]),
                &["NW-2", "NW-3", "NW-4", "NW-5"],
            ),
            case(
                custom(10072),
                Op::IsNot,
                one("Silver"),
                &["NW-1", "NW-3", "NW-4", "NW-5"],
            ),
            case(custom(10072), Op::IsEmpty, None, &["NW-3", "NW-4", "NW-5"]),
        ],
    )
    .await;
}

#[tokio::test]
async fn custom_text_fields_match_whole_values_or_parts() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(custom(10073), Op::Is, one("plain"), &["NW-2"]),
            case(custom(10073), Op::Contains, one("100"), &["NW-1", "NW-5"]),
            case(custom(10073), Op::Contains, one("NEEDS"), &["NW-1"]),
            case(custom(10073), Op::Contains, one("0%"), &["NW-1"]),
            case(custom(10073), Op::Contains, one("w_o"), &["NW-1"]),
            case(custom(10073), Op::Contains, one("d_n"), &[]),
            case(custom(10073), Op::IsEmpty, None, &["NW-3", "NW-4"]),
            case(
                custom(10073),
                Op::IsNotEmpty,
                None,
                &["NW-1", "NW-2", "NW-5"],
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn custom_number_fields_compare_numerically() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(custom(10074), Op::Is, one("5"), &["NW-1", "NW-4"]),
            case(custom(10074), Op::Is, one("5.0"), &["NW-1", "NW-4"]),
            case(
                custom(10074),
                Op::In,
                many(&["12", "5"]),
                &["NW-1", "NW-2", "NW-4"],
            ),
            case(
                custom(10074),
                Op::IsNot,
                one("5"),
                &["NW-2", "NW-3", "NW-5"],
            ),
            case(custom(10074), Op::IsEmpty, None, &["NW-3", "NW-5"]),
            case(
                custom(10074),
                Op::IsNotEmpty,
                None,
                &["NW-1", "NW-2", "NW-4"],
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn custom_array_fields_match_string_and_object_elements() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(custom(10075), Op::Is, one("blue"), &["NW-1", "NW-2"]),
            case(custom(10075), Op::Is, one("red"), &["NW-1"]),
            case(
                custom(10075),
                Op::IsNot,
                one("blue"),
                &["NW-3", "NW-4", "NW-5"],
            ),
            case(custom(10075), Op::Contains, one("ed"), &["NW-1"]),
            case(custom(10075), Op::IsEmpty, None, &["NW-3", "NW-4", "NW-5"]),
            case(custom(10075), Op::IsNotEmpty, None, &["NW-1", "NW-2"]),
            case(custom(10078), Op::Is, one("Web"), &["NW-1", "NW-2"]),
            case(custom(10078), Op::Is, one("API"), &["NW-1"]),
            case(custom(10078), Op::Is, one("77"), &["NW-4"]),
            case(
                custom(10078),
                Op::In,
                many(&["API", "77"]),
                &["NW-1", "NW-4"],
            ),
            case(
                custom(10078),
                Op::NotIn,
                many(&["Web"]),
                &["NW-3", "NW-4", "NW-5"],
            ),
            case(custom(10078), Op::Contains, one("e"), &["NW-1", "NW-2"]),
        ],
    )
    .await;
}

#[tokio::test]
async fn custom_date_and_datetime_fields_compare_the_day_as_written() {
    let fx = Fixture::new().await;
    fx.check(
        None,
        vec![
            case(custom(10076), Op::Before, one("2026-10-30"), &["NW-2"]),
            case(custom(10076), Op::After, one("2026-10-28"), &["NW-1"]),
            case(
                custom(10076),
                Op::WithinNextDays,
                days(7),
                &["NW-1", "NW-2"],
            ),
            case(custom(10076), Op::WithinLastDays, days(3), &["NW-2"]),
            case(custom(10076), Op::IsEmpty, None, &["NW-3", "NW-4", "NW-5"]),
            case(custom(10077), Op::Before, one("2026-10-29"), &["NW-1"]),
            case(custom(10077), Op::After, one("2026-10-28"), &["NW-2"]),
            case(custom(10077), Op::WithinNextDays, days(0), &["NW-1"]),
            case(custom(10077), Op::IsNotEmpty, None, &["NW-1", "NW-2"]),
        ],
    )
    .await;
}

#[tokio::test]
async fn several_conditions_are_anded_with_their_parameters_in_order() {
    let fx = Fixture::new().await;
    let both = fx
        .matching(
            &[
                Condition::new(custom(10073), Op::Contains, one("100")),
                Condition::is(Field::Labels, "backend"),
                Condition::new(Field::Status, Op::In, many(&["10001", "10002"])),
            ],
            Some(ALEX),
        )
        .await
        .unwrap();
    assert_eq!(both, ["NW-1", "NW-5"]);

    let narrowed = fx
        .matching(
            &[
                Condition::new(custom(10072), Op::Is, one("Gold")),
                Condition::is(Field::Release, "v2"),
                Condition::is(Field::Assignee, "me"),
                Condition::new(Field::Updated, Op::Before, one("2026-10-28")),
                Condition::is(Field::Sprint, "active"),
            ],
            Some(ALEX),
        )
        .await
        .unwrap();
    assert_eq!(narrowed, ["NW-1"]);

    let none = fx
        .matching(
            &[
                Condition::is(Field::Status, "10003"),
                Condition::is(Field::Assignee, "me"),
            ],
            Some(ALEX),
        )
        .await
        .unwrap();
    assert!(none.is_empty());
}

#[tokio::test]
async fn values_are_data_and_never_sql() {
    let fx = Fixture::new().await;
    let hostile = "x' OR 1=1 --";
    fx.check(
        Some(ALEX),
        vec![
            case(Field::Status, Op::Is, one(hostile), &[]),
            case(Field::Assignee, Op::In, many(&[hostile]), &[]),
            case(Field::Labels, Op::Is, one(hostile), &[]),
            case(Field::Labels, Op::Contains, one(hostile), &[]),
            case(Field::Sprint, Op::Is, one(hostile), &[]),
            case(
                Field::Release,
                Op::NotIn,
                many(&[hostile]),
                &["NW-1", "NW-2", "NW-3", "NW-4", "NW-5"],
            ),
            case(custom(10073), Op::Contains, one(hostile), &[]),
            case(custom(10072), Op::Is, one(hostile), &[]),
            case(
                Field::Priority,
                Op::IsNot,
                one(hostile),
                &["NW-1", "NW-2", "NW-3", "NW-4", "NW-5"],
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn conditions_that_cannot_be_compiled_are_invalid_input() {
    let fx = Fixture::new().await;
    fx.assert_refused(vec![
        Condition::is(Field::Custom("customfield_99999".into()), "x"),
        Condition::is(custom(10079), "x"),
        Condition::is(Field::Custom("status; DROP TABLE issue".into()), "x"),
    ]);
    fx.assert_refused(
        [
            (Field::Status, Op::Contains, one("x")),
            (Field::Status, Op::Before, one("2026-10-28")),
            (Field::Assignee, Op::Contains, one("x")),
            (Field::Priority, Op::WithinLastDays, days(3)),
            (Field::Labels, Op::Before, one("2026-10-28")),
            (Field::Release, Op::Contains, one("v")),
            (Field::Sprint, Op::Contains, one("act")),
            (Field::Sprint, Op::Before, one("2026-10-28")),
            (Field::Due, Op::Is, one("2026-10-28")),
            (Field::Due, Op::Contains, one("2026")),
            (Field::Updated, Op::In, many(&["2026-10-28"])),
            (custom(10072), Op::Contains, one("Go")),
            (custom(10074), Op::Contains, one("5")),
            (custom(10076), Op::Contains, one("2026")),
            (custom(10071), Op::Contains, one("acc")),
        ]
        .into_iter()
        .map(|(field, op, value)| Condition::new(field, op, value))
        .collect(),
    );
}

#[tokio::test]
async fn values_of_the_wrong_shape_are_invalid_input() {
    let fx = Fixture::new().await;
    fx.assert_refused(
        [
            (Field::Status, Op::Is, many(&["10001"])),
            (Field::Status, Op::Is, None),
            (Field::Status, Op::Is, days(3)),
            (Field::Status, Op::IsNot, many(&["10001", "10002"])),
            (Field::Status, Op::In, None),
            (Field::Status, Op::In, days(3)),
            (Field::Labels, Op::Is, many(&["a"])),
            (Field::Labels, Op::Contains, None),
            (Field::Sprint, Op::Is, None),
            (Field::Sprint, Op::In, days(1)),
            (Field::Due, Op::WithinLastDays, one("7")),
            (Field::Due, Op::WithinNextDays, None),
            (Field::Due, Op::Before, days(3)),
            (Field::Due, Op::After, None),
            (Field::Updated, Op::WithinNextDays, many(&["3"])),
            (custom(10073), Op::Contains, many(&["a"])),
        ]
        .into_iter()
        .map(|(field, op, value)| Condition::new(field, op, value))
        .collect(),
    );
}

#[tokio::test]
async fn bad_dates_numbers_and_day_counts_are_invalid_input() {
    let fx = Fixture::new().await;
    fx.assert_refused(
        [
            (Field::Due, Op::Before, one("28.10.2026")),
            (Field::Due, Op::After, one("")),
            (Field::Updated, Op::After, one("2026-13-40")),
            (Field::Created, Op::Before, one("yesterday")),
            (custom(10076), Op::Before, one("soon")),
            (custom(10074), Op::Is, one("abc")),
            (custom(10074), Op::In, many(&["5", "many"])),
            (Field::Due, Op::WithinLastDays, days(u32::MAX)),
            (Field::Due, Op::WithinNextDays, days(u32::MAX)),
        ]
        .into_iter()
        .map(|(field, op, value)| Condition::new(field, op, value))
        .collect(),
    );
}
