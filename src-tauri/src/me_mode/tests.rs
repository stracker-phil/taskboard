use std::collections::HashMap;

use chrono_tz::Tz;
use rusqlite::params_from_iter;
use serde_json::json;
use tempfile::TempDir;

use super::*;
use crate::db::Db;
use crate::filter::CustomField;
use crate::test_support::temp_config_dir;

const ME: &str = "acc-me";
const LEA: &str = "acc-lea";
const REVIEWER: &str = "customfield_10071";

fn mention(account_id: &str) -> String {
    json!({"type": "doc", "version": 1, "content": [
        {"type": "paragraph", "content": [
            {"type": "text", "text": "Please look "},
            {"type": "mention", "attrs": {"id": account_id, "text": "@Someone"}}
        ]}
    ]})
    .to_string()
}

fn ctx(me: Option<&str>) -> Ctx {
    Ctx {
        me: me.map(Into::into),
        today: "2026-10-28".parse().unwrap(),
        tz: Tz::Europe__Berlin,
        custom: HashMap::from([(
            REVIEWER.to_string(),
            CustomField {
                name: "Reviewer".into(),
                kind: "user".into(),
            },
        )]),
    }
}

fn on(rule: MeMatch) -> MeRule {
    MeRule { rule, on: true }
}

fn reporter() -> MeMatch {
    MeMatch::Field {
        field: "reporter".into(),
    }
}

fn reviewer() -> MeMatch {
    MeMatch::Field {
        field: REVIEWER.into(),
    }
}

/// Issues by key:
/// 1 assigned to me; 2 reported by me; 3 reviewed by me; 4 mentions me; 5 mentions Lea;
/// 6 subtask of 1; 7 epic assigned to me; 8 child of 7 (not a subtask); 9 unrelated;
/// 10 subtask of 2.
struct Site {
    _dir: TempDir,
    db: Db,
}

impl Site {
    async fn new() -> Self {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| {
            tx.execute(
                "INSERT INTO cache.space (id, key, name, raw) VALUES ('1', 'DEMO', 'Demo', '{}')",
                [],
            )?;
            let issues = [
                (1, Some(ME), None, None, None, 0),
                (2, Some(LEA), Some(ME), None, None, 0),
                (3, Some(LEA), None, Some(ME), None, 0),
                (4, Some(LEA), None, None, None, 0),
                (5, Some(LEA), None, None, None, 0),
                (6, Some(LEA), None, None, Some(1), 1),
                (7, Some(ME), None, None, None, 0),
                (8, Some(LEA), None, None, Some(7), 0),
                (9, Some(LEA), None, None, None, 0),
                (10, Some(LEA), None, None, Some(2), 1),
            ];
            for (id, assignee, reported_by, reviewed_by, parent, subtask) in issues {
                let custom = reviewed_by
                    .map(|account| json!({REVIEWER: {"accountId": account}}).to_string());
                tx.execute(
                    "INSERT INTO cache.issue (id, key, project_id, parent_id, summary, issue_type,
                        type_is_subtask, status_id, status_category, assignee_id, reporter_id,
                        custom, created_at, updated_at, synced_at)
                     VALUES (?1, ?2, '1', ?3, 'Demo issue', 'Task', ?4, 's1', 'new', ?5, ?6, ?7,
                        '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z')",
                    rusqlite::params![
                        id.to_string(),
                        format!("DEMO-{id}"),
                        parent.map(|p: i32| p.to_string()),
                        subtask,
                        assignee,
                        reported_by,
                        custom
                    ],
                )?;
            }
            for (id, issue, who) in [("c1", "4", ME), ("c2", "5", LEA)] {
                tx.execute(
                    "INSERT INTO cache.comment (id, issue_id, body_adf, created_at, updated_at)
                     VALUES (?1, ?2, ?3, '2026-10-02T09:00:00Z', '2026-10-02T09:00:00Z')",
                    rusqlite::params![id, issue, mention(who)],
                )?;
            }
            tx.execute(
                "INSERT INTO cache.field (id, name, type, shown) VALUES
                    ('customfield_10071', 'Reviewer', 'user', 1),
                    ('customfield_10072', 'Approver', 'user', 1),
                    ('customfield_10073', 'Notes', 'string', 1),
                    ('summary', 'Summary', 'user', 1)",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
        Self { _dir: dir, db }
    }

    async fn matching(&self, rules: Vec<MeRule>, ctx: Ctx) -> Vec<String> {
        self.db
            .read(move |conn| {
                let (sql, params) = condition(&rules, &ctx)?;
                let mut query = conn.prepare(&format!(
                    "SELECT i.key FROM cache.issue i WHERE {sql} ORDER BY CAST(i.id AS INTEGER)"
                ))?;
                let keys = query.query_map(params_from_iter(params), |row| row.get(0))?;
                Ok(keys.collect::<Result<Vec<String>, _>>()?)
            })
            .await
            .unwrap()
    }

    async fn mine(&self, rules: Vec<MeRule>) -> Vec<String> {
        self.matching(rules, ctx(Some(ME))).await
    }
}

fn keys(numbers: &[u32]) -> Vec<String> {
    numbers.iter().map(|n| format!("DEMO-{n}")).collect()
}

#[tokio::test]
async fn without_rules_only_my_assigned_issues_are_mine() {
    let site = Site::new().await;

    assert_eq!(site.mine(Vec::new()).await, keys(&[1, 7]));
    assert_eq!(site.mine(default_rules()).await, keys(&[1, 7]));
}

#[tokio::test]
async fn the_reporter_rule_adds_the_issues_i_reported() {
    let site = Site::new().await;

    assert_eq!(site.mine(vec![on(reporter())]).await, keys(&[1, 2, 7]));
}

#[tokio::test]
async fn a_rule_switched_off_adds_nothing() {
    let site = Site::new().await;
    let rules = vec![MeRule {
        rule: reporter(),
        on: false,
    }];

    assert_eq!(site.mine(rules).await, keys(&[1, 7]));
}

#[tokio::test]
async fn a_custom_user_field_rule_adds_the_issues_whose_field_holds_me() {
    let site = Site::new().await;

    assert_eq!(site.mine(vec![on(reviewer())]).await, keys(&[1, 3, 7]));
}

#[tokio::test]
async fn a_custom_field_the_cache_does_not_know_is_ignored() {
    let site = Site::new().await;
    let unknown = on(MeMatch::Field {
        field: "customfield_99999".into(),
    });

    assert_eq!(site.mine(vec![unknown]).await, keys(&[1, 7]));
}

#[tokio::test]
async fn an_assignee_field_rule_adds_nothing_to_the_assignee_match() {
    let site = Site::new().await;
    let assignee = on(MeMatch::Field {
        field: "assignee".into(),
    });

    assert_eq!(site.mine(vec![assignee]).await, keys(&[1, 7]));
}

#[tokio::test]
async fn the_mentioned_rule_finds_comments_that_mention_me_and_not_someone_else() {
    let site = Site::new().await;

    assert_eq!(
        site.mine(vec![on(MeMatch::Mentioned)]).await,
        keys(&[1, 4, 7])
    );
}

#[tokio::test]
async fn the_subtasks_rule_adds_the_subtasks_of_my_issues_but_not_other_children() {
    let site = Site::new().await;

    assert_eq!(
        site.mine(vec![on(MeMatch::Subtasks)]).await,
        keys(&[1, 6, 7])
    );
}

#[tokio::test]
async fn the_subtasks_rule_follows_the_other_rules_for_the_parent() {
    let site = Site::new().await;

    assert_eq!(
        site.mine(vec![on(MeMatch::Subtasks), on(reporter())]).await,
        keys(&[1, 2, 6, 7, 10])
    );
}

#[tokio::test]
async fn every_rule_on_adds_up() {
    let site = Site::new().await;
    let rules = vec![
        on(reporter()),
        on(reviewer()),
        on(MeMatch::Mentioned),
        on(MeMatch::Subtasks),
    ];

    assert_eq!(site.mine(rules).await, keys(&[1, 2, 3, 4, 6, 7, 10]));
}

#[tokio::test]
async fn without_an_account_nothing_is_mine() {
    let site = Site::new().await;
    let rules = vec![
        on(reporter()),
        on(MeMatch::Mentioned),
        on(MeMatch::Subtasks),
    ];

    assert_eq!(site.matching(rules, ctx(None)).await, Vec::<String>::new());
}

#[test]
fn the_default_rules_are_reporter_mentioned_and_subtasks_all_off() {
    assert_eq!(
        default_rules(),
        vec![
            MeRule {
                rule: reporter(),
                on: false
            },
            MeRule {
                rule: MeMatch::Mentioned,
                on: false
            },
            MeRule {
                rule: MeMatch::Subtasks,
                on: false
            },
        ]
    );
}

fn app_rules() -> Vec<MeRule> {
    vec![
        on(reporter()),
        MeRule {
            rule: MeMatch::Mentioned,
            on: false,
        },
        on(MeMatch::Subtasks),
    ]
}

#[test]
fn rules_for_without_own_rules_keeps_the_apps_switches() {
    assert_eq!(rules_for(&app_rules(), None), app_rules());
}

#[test]
fn rules_for_switches_exactly_the_listed_rules_on_in_the_apps_order() {
    let rules = rules_for(&app_rules(), Some(&[MeMatch::Mentioned, MeMatch::Subtasks]));

    assert_eq!(
        rules,
        vec![
            MeRule {
                rule: reporter(),
                on: false
            },
            on(MeMatch::Mentioned),
            on(MeMatch::Subtasks),
        ]
    );
}

#[test]
fn rules_for_an_empty_list_switches_every_rule_off() {
    let rules = rules_for(&app_rules(), Some(&[]));

    assert_eq!(rules.len(), 3);
    assert!(rules.iter().all(|rule| !rule.on), "{rules:?}");
}

#[test]
fn rules_for_ignores_listed_rules_the_app_does_not_offer() {
    let rules = rules_for(&app_rules(), Some(&[reviewer()]));

    assert_eq!(rules.len(), 3);
    assert!(rules.iter().all(|rule| !rule.on), "{rules:?}");
}

#[test]
fn rules_serialize_with_a_kind_tag_and_camel_case_names() {
    let rules = vec![on(reviewer()), on(MeMatch::Mentioned)];

    assert_eq!(
        serde_json::to_value(&rules).unwrap(),
        json!([
            {"rule": {"kind": "field", "field": "customfield_10071"}, "on": true},
            {"rule": {"kind": "mentioned"}, "on": true}
        ])
    );
}

#[test]
fn switching_a_rule_sets_only_that_rule() {
    for (rule, on) in [(MeMatch::Mentioned, true), (reporter(), true)] {
        let edited = edited(
            default_rules(),
            MeRuleEdit::Switch {
                rule: rule.clone(),
                on,
            },
        );

        let expected: Vec<MeRule> = default_rules()
            .into_iter()
            .map(|r| MeRule {
                on: r.rule == rule,
                ..r
            })
            .collect();
        assert_eq!(edited, expected, "{rule:?}");
    }
}

#[test]
fn switching_a_rule_that_is_not_offered_changes_nothing() {
    let rules = edited(
        default_rules(),
        MeRuleEdit::Switch {
            rule: reviewer(),
            on: true,
        },
    );

    assert_eq!(rules, default_rules());
}

#[test]
fn adding_a_field_appends_it_switched_on() {
    let rules = edited(
        default_rules(),
        MeRuleEdit::AddField {
            field: REVIEWER.into(),
        },
    );

    let mut expected = default_rules();
    expected.push(on(reviewer()));
    assert_eq!(rules, expected);
}

#[test]
fn adding_a_field_already_offered_keeps_its_place_and_state() {
    let start = vec![
        MeRule {
            rule: reviewer(),
            on: false,
        },
        on(MeMatch::Mentioned),
    ];

    let rules = edited(
        start.clone(),
        MeRuleEdit::AddField {
            field: REVIEWER.into(),
        },
    );

    assert_eq!(rules, start);
}

#[test]
fn removing_a_field_drops_only_that_field_rule() {
    let mut start = default_rules();
    start.push(on(reviewer()));

    let rules = edited(
        start,
        MeRuleEdit::RemoveField {
            field: REVIEWER.into(),
        },
    );

    assert_eq!(rules, default_rules());
}

#[test]
fn removing_a_field_never_removes_mentioned_or_subtasks() {
    let rules = edited(
        default_rules(),
        MeRuleEdit::RemoveField {
            field: "mentioned".into(),
        },
    );

    assert_eq!(rules, default_rules());
}

#[test]
fn rule_edits_deserialize_from_a_kind_tag_with_camel_case_names() {
    let edits: Vec<MeRuleEdit> = serde_json::from_value(json!([
        {"kind": "switch", "rule": {"kind": "subtasks"}, "on": true},
        {"kind": "addField", "field": "customfield_10071"},
        {"kind": "removeField", "field": "customfield_10071"}
    ]))
    .unwrap();

    assert_eq!(
        edits,
        vec![
            MeRuleEdit::Switch {
                rule: MeMatch::Subtasks,
                on: true
            },
            MeRuleEdit::AddField {
                field: REVIEWER.into()
            },
            MeRuleEdit::RemoveField {
                field: REVIEWER.into()
            },
        ]
    );
}

#[tokio::test]
async fn user_fields_list_reporter_first_then_the_custom_user_fields_by_name() {
    let site = Site::new().await;

    let fields = site.db.read(|conn| user_fields(conn)).await.unwrap();

    let expected = [
        ("reporter", "Reporter"),
        ("customfield_10072", "Approver"),
        ("customfield_10071", "Reviewer"),
    ]
    .map(|(field, name)| UserField {
        field: field.into(),
        name: name.into(),
    });
    assert_eq!(fields, expected);
}
