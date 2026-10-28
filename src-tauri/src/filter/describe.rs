//! Conditions in words, for tooltips and the empty list ("Status = In review and Reviewer = me").

use std::collections::HashMap;

use super::model::{Condition, Field, ME, Op, Value};

/// Display names of the ids conditions hold, read from the cache. A missing id shows as itself.
#[derive(Clone, Debug, Default)]
pub struct Names {
    pub statuses: HashMap<String, String>,
    pub users: HashMap<String, String>,
    pub versions: HashMap<String, String>,
    /// Epic issue id → summary.
    pub epics: HashMap<String, String>,
    /// Project id → key.
    pub projects: HashMap<String, String>,
    pub sprints: HashMap<String, String>,
    /// Custom field id → name.
    pub fields: HashMap<String, String>,
}

pub fn describe(conditions: &[Condition], names: &Names) -> String {
    conditions
        .iter()
        .map(|c| sentence(c, names))
        .collect::<Vec<_>>()
        .join(" and ")
}

fn sentence(c: &Condition, names: &Names) -> String {
    let field = field_name(&c.field, names);
    let value = |v: &str| value_name(&c.field, v, names);
    let one = || match &c.value {
        Some(Value::One(v)) => value(v),
        Some(Value::Many(vs)) => vs.iter().map(|v| value(v)).collect::<Vec<_>>().join(", "),
        Some(Value::Days(n)) => n.to_string(),
        None => String::new(),
    };
    let days = || match c.value {
        Some(Value::Days(1)) => "day".to_string(),
        Some(Value::Days(n)) => format!("{n} days"),
        _ => "days".to_string(),
    };
    match c.op {
        Op::Is => format!("{field} = {}", one()),
        Op::IsNot => format!("{field} ≠ {}", one()),
        Op::In => format!("{field} in {}", one()),
        Op::NotIn => format!("{field} not in {}", one()),
        Op::IsEmpty => format!("{field} is empty"),
        Op::IsNotEmpty => format!("{field} is set"),
        Op::Contains => format!("{field} contains “{}”", one()),
        Op::Before => format!("{field} before {}", one()),
        Op::After => format!("{field} after {}", one()),
        Op::WithinLastDays => format!("{field} in the last {}", days()),
        Op::WithinNextDays => format!("{field} in the next {}", days()),
    }
}

fn field_name(field: &Field, names: &Names) -> String {
    match field {
        Field::Status => "Status",
        Field::StatusCategory => "Status category",
        Field::Type => "Type",
        Field::Assignee => "Assignee",
        Field::Reporter => "Reporter",
        Field::Priority => "Priority",
        Field::Epic => "Epic",
        Field::Release => "Release",
        Field::Sprint => "Sprint",
        Field::Labels => "Labels",
        Field::Due => "Due date",
        Field::Updated => "Updated",
        Field::Created => "Created",
        Field::Project => "Project",
        Field::Custom(id) => return names.fields.get(id).cloned().unwrap_or_else(|| id.clone()),
    }
    .to_string()
}

fn value_name(field: &Field, value: &str, names: &Names) -> String {
    let lookup = |map: &HashMap<String, String>| map.get(value).cloned();
    let user = || {
        if value == ME {
            Some(ME.to_string())
        } else {
            lookup(&names.users)
        }
    };
    let named = match field {
        Field::Status => lookup(&names.statuses),
        Field::StatusCategory => match value {
            "new" => Some("To do".into()),
            "indeterminate" => Some("In progress".into()),
            "done" => Some("Done".into()),
            _ => None,
        },
        Field::Assignee | Field::Reporter => user(),
        Field::Epic => lookup(&names.epics),
        Field::Release => lookup(&names.versions),
        Field::Project => lookup(&names.projects),
        Field::Sprint => match value {
            "active" => Some("open sprint".into()),
            "future" => Some("future sprint".into()),
            "closed" => Some("closed sprint".into()),
            _ => lookup(&names.sprints),
        },
        Field::Custom(_) => user(),
        _ => None,
    };
    named.unwrap_or_else(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Names {
        let map = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>()
        };
        Names {
            statuses: map(&[("10001", "Open"), ("10002", "In Review")]),
            users: map(&[("acc-alex", "Alex Example")]),
            versions: map(&[("v1", "1.0")]),
            epics: map(&[("e1", "Onboarding")]),
            projects: map(&[("p1", "NW")]),
            sprints: map(&[("7", "Sprint 7")]),
            fields: map(&[
                ("customfield_10071", "Reviewer"),
                ("customfield_10073", "Notes"),
            ]),
        }
    }

    fn one(value: &str) -> Option<Value> {
        Some(Value::One(value.into()))
    }

    fn many(values: &[&str]) -> Option<Value> {
        Some(Value::Many(values.iter().map(|v| v.to_string()).collect()))
    }

    fn custom(id: &str) -> Field {
        Field::Custom(id.into())
    }

    fn sentence_of(field: Field, op: Op, value: Option<Value>) -> String {
        describe(&[Condition::new(field, op, value)], &names())
    }

    #[test]
    fn conditions_read_as_sentences() {
        for (field, op, value, expected) in [
            (Field::Status, Op::Is, one("10002"), "Status = In Review"),
            (Field::Status, Op::IsNot, one("10001"), "Status ≠ Open"),
            (
                Field::Status,
                Op::In,
                many(&["10001", "10002"]),
                "Status in Open, In Review",
            ),
            (
                Field::Status,
                Op::NotIn,
                many(&["10001", "10002"]),
                "Status not in Open, In Review",
            ),
            (Field::Due, Op::IsEmpty, None, "Due date is empty"),
            (Field::Labels, Op::IsNotEmpty, None, "Labels is set"),
            (
                Field::Labels,
                Op::Contains,
                one("urgent"),
                "Labels contains “urgent”",
            ),
            (
                Field::Due,
                Op::Before,
                one("2026-11-01"),
                "Due date before 2026-11-01",
            ),
            (
                Field::Updated,
                Op::After,
                one("2026-10-01"),
                "Updated after 2026-10-01",
            ),
            (
                Field::Created,
                Op::WithinLastDays,
                Some(Value::Days(7)),
                "Created in the last 7 days",
            ),
            (
                Field::Created,
                Op::WithinLastDays,
                Some(Value::Days(1)),
                "Created in the last day",
            ),
            (
                Field::Due,
                Op::WithinNextDays,
                Some(Value::Days(3)),
                "Due date in the next 3 days",
            ),
            (
                Field::Due,
                Op::WithinNextDays,
                Some(Value::Days(1)),
                "Due date in the next day",
            ),
        ] {
            assert_eq!(sentence_of(field, op, value), expected);
        }
    }

    #[test]
    fn field_names_read_in_words() {
        for (field, expected) in [
            (Field::Status, "Status"),
            (Field::StatusCategory, "Status category"),
            (Field::Type, "Type"),
            (Field::Assignee, "Assignee"),
            (Field::Reporter, "Reporter"),
            (Field::Priority, "Priority"),
            (Field::Epic, "Epic"),
            (Field::Release, "Release"),
            (Field::Sprint, "Sprint"),
            (Field::Labels, "Labels"),
            (Field::Due, "Due date"),
            (Field::Updated, "Updated"),
            (Field::Created, "Created"),
            (Field::Project, "Project"),
            (custom("customfield_10073"), "Notes"),
            (custom("customfield_99999"), "customfield_99999"),
        ] {
            assert_eq!(
                sentence_of(field, Op::IsEmpty, None),
                format!("{expected} is empty")
            );
        }
    }

    #[test]
    fn values_read_as_the_names_the_cache_knows() {
        for (field, value, expected) in [
            (Field::Assignee, "acc-alex", "Alex Example"),
            (Field::Reporter, "acc-alex", "Alex Example"),
            (Field::Assignee, "acc-unknown", "acc-unknown"),
            (Field::Release, "v1", "1.0"),
            (Field::Epic, "e1", "Onboarding"),
            (Field::Project, "p1", "NW"),
            (Field::Status, "99999", "99999"),
            (Field::Type, "Bug", "Bug"),
            (Field::Priority, "High", "High"),
            (custom("customfield_10071"), "acc-alex", "Alex Example"),
            (custom("customfield_10073"), "plain", "plain"),
        ] {
            assert_eq!(
                sentence_of(field.clone(), Op::Is, one(value)),
                format!("{} = {expected}", field_name(&field, &names())),
                "{value}"
            );
        }
    }

    #[test]
    fn me_stays_me_for_user_fields() {
        for field in [
            Field::Assignee,
            Field::Reporter,
            custom("customfield_10071"),
        ] {
            let name = field_name(&field, &names());
            assert_eq!(sentence_of(field, Op::Is, one(ME)), format!("{name} = me"));
        }
    }

    #[test]
    fn status_categories_read_as_board_columns() {
        for (value, expected) in [
            ("new", "To do"),
            ("indeterminate", "In progress"),
            ("done", "Done"),
            ("other", "other"),
        ] {
            assert_eq!(
                sentence_of(Field::StatusCategory, Op::Is, one(value)),
                format!("Status category = {expected}")
            );
        }
    }

    #[test]
    fn sprint_states_read_as_sprints_and_ids_by_name() {
        for (value, expected) in [
            ("active", "open sprint"),
            ("future", "future sprint"),
            ("closed", "closed sprint"),
            ("7", "Sprint 7"),
            ("8", "8"),
        ] {
            assert_eq!(
                sentence_of(Field::Sprint, Op::Is, one(value)),
                format!("Sprint = {expected}")
            );
        }
    }

    #[test]
    fn conditions_are_joined_with_and() {
        let conditions = [
            Condition::is(Field::Status, "10002"),
            Condition::is(custom("customfield_10071"), ME),
        ];
        assert_eq!(
            describe(&conditions, &names()),
            "Status = In Review and Reviewer = me"
        );
    }

    #[test]
    fn no_conditions_read_as_nothing() {
        assert_eq!(describe(&[], &names()), "");
    }

    #[test]
    fn missing_names_fall_back_to_the_ids() {
        assert_eq!(
            describe(&[Condition::is(Field::Status, "10002")], &Names::default()),
            "Status = 10002"
        );
    }
}
