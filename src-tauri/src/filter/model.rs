//! A filter list's conditions as stored in `filter_list.filter` (docs/data.md): a typed list, ANDed.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Filter {
    pub conditions: Vec<Condition>,
}

impl Filter {
    /// A stored filter; null or empty text is no condition.
    pub fn parse(json: Option<&str>) -> AppResult<Self> {
        match json.map(str::trim) {
            None | Some("") => Ok(Self::default()),
            Some(json) => serde_json::from_str(json)
                .map_err(|e| AppError::InvalidInput(format!("The filter can't be read: {e}"))),
        }
    }

    /// Null when there is no condition, so an empty filter is stored like none.
    pub fn to_json(&self) -> AppResult<Option<String>> {
        if self.conditions.is_empty() {
            return Ok(None);
        }
        Ok(Some(serde_json::to_string(self)?))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Condition {
    pub field: Field,
    pub op: Op,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<Value>,
}

impl Condition {
    pub fn new(field: Field, op: Op, value: Option<Value>) -> Self {
        Self { field, op, value }
    }

    pub fn is(field: Field, value: impl Into<String>) -> Self {
        Self::new(field, Op::Is, Some(Value::One(value.into())))
    }
}

/// An issue field. Jira's custom fields go by their id (`customfield_10071`); a name the cache doesn't know
/// is reported when the filter is compiled.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, TS)]
#[serde(from = "String", into = "String")]
#[ts(export, export_to = "src/api/types.gen.ts", type = "string")]
pub enum Field {
    Status,
    StatusCategory,
    Type,
    Assignee,
    Reporter,
    Priority,
    /// The parent epic's issue id.
    Epic,
    /// A fix version id.
    Release,
    /// `active`, `future`, `closed` or a sprint id.
    Sprint,
    Labels,
    Due,
    Updated,
    Created,
    Project,
    /// Key, summary, description or comments hold every word (full-text search).
    Text,
    Custom(String),
}

const SYSTEM_FIELDS: [(Field, &str); 15] = [
    (Field::Status, "status"),
    (Field::StatusCategory, "statusCategory"),
    (Field::Type, "type"),
    (Field::Assignee, "assignee"),
    (Field::Reporter, "reporter"),
    (Field::Priority, "priority"),
    (Field::Epic, "epic"),
    (Field::Release, "release"),
    (Field::Sprint, "sprint"),
    (Field::Labels, "labels"),
    (Field::Due, "due"),
    (Field::Updated, "updated"),
    (Field::Created, "created"),
    (Field::Project, "project"),
    (Field::Text, "text"),
];

impl From<String> for Field {
    fn from(name: String) -> Self {
        SYSTEM_FIELDS
            .iter()
            .find(|(_, n)| *n == name)
            .map(|(field, _)| field.clone())
            .unwrap_or(Field::Custom(name))
    }
}

impl Field {
    /// The name it is stored under.
    pub fn name(&self) -> &str {
        match self {
            Field::Custom(id) => id,
            system => SYSTEM_FIELDS
                .iter()
                .find(|(f, _)| f == system)
                .map_or("", |(_, name)| name),
        }
    }
}

impl From<Field> for String {
    fn from(field: Field) -> Self {
        field.name().to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Op {
    Is,
    IsNot,
    In,
    NotIn,
    IsEmpty,
    IsNotEmpty,
    Contains,
    Before,
    After,
    WithinLastDays,
    WithinNextDays,
}

impl Op {
    /// The name it is stored under.
    pub fn name(self) -> &'static str {
        match self {
            Op::Is => "is",
            Op::IsNot => "isNot",
            Op::In => "in",
            Op::NotIn => "notIn",
            Op::IsEmpty => "isEmpty",
            Op::IsNotEmpty => "isNotEmpty",
            Op::Contains => "contains",
            Op::Before => "before",
            Op::After => "after",
            Op::WithinLastDays => "withinLastDays",
            Op::WithinNextDays => "withinNextDays",
        }
    }
}

/// `me` as a user field's value is the connected account.
pub const ME: &str = "me";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(untagged)]
pub enum Value {
    Days(u32),
    One(String),
    Many(Vec<String>),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn status_is(id: &str) -> Condition {
        Condition::is(Field::Status, id)
    }

    #[test]
    fn a_filter_serializes_to_the_stored_shape() {
        let filter = Filter {
            conditions: vec![status_is("10002")],
        };
        assert_eq!(
            serde_json::to_value(&filter).unwrap(),
            json!({"conditions": [{"field": "status", "op": "is", "value": "10002"}]})
        );
    }

    #[test]
    fn a_filter_survives_a_json_round_trip() {
        let filter = Filter {
            conditions: vec![
                status_is("10002"),
                Condition::new(
                    Field::Labels,
                    Op::In,
                    Some(Value::Many(vec!["a".into(), "b".into()])),
                ),
                Condition::new(Field::Due, Op::WithinNextDays, Some(Value::Days(7))),
                Condition::new(Field::Custom("customfield_10071".into()), Op::IsEmpty, None),
            ],
        };
        let json = filter.to_json().unwrap().unwrap();
        assert_eq!(Filter::parse(Some(&json)).unwrap(), filter);
    }

    #[test]
    fn field_names_map_to_fields_and_back() {
        for (name, field) in [
            ("status", Field::Status),
            ("statusCategory", Field::StatusCategory),
            ("type", Field::Type),
            ("assignee", Field::Assignee),
            ("reporter", Field::Reporter),
            ("priority", Field::Priority),
            ("epic", Field::Epic),
            ("release", Field::Release),
            ("sprint", Field::Sprint),
            ("labels", Field::Labels),
            ("due", Field::Due),
            ("updated", Field::Updated),
            ("created", Field::Created),
            ("project", Field::Project),
            ("text", Field::Text),
            (
                "customfield_10071",
                Field::Custom("customfield_10071".into()),
            ),
        ] {
            assert_eq!(Field::from(name.to_string()), field, "{name}");
            assert_eq!(String::from(field), name, "{name}");
        }
    }

    #[test]
    fn an_unknown_field_name_is_a_custom_field() {
        assert_eq!(
            Field::from("nonsense".to_string()),
            Field::Custom("nonsense".into())
        );
    }

    #[test]
    fn operators_use_camel_case_names() {
        for (op, name) in [
            (Op::Is, "is"),
            (Op::IsNot, "isNot"),
            (Op::In, "in"),
            (Op::NotIn, "notIn"),
            (Op::IsEmpty, "isEmpty"),
            (Op::IsNotEmpty, "isNotEmpty"),
            (Op::Contains, "contains"),
            (Op::Before, "before"),
            (Op::After, "after"),
            (Op::WithinLastDays, "withinLastDays"),
            (Op::WithinNextDays, "withinNextDays"),
        ] {
            assert_eq!(serde_json::to_value(op).unwrap(), json!(name), "{name}");
        }
    }

    #[test]
    fn values_read_as_days_one_or_many_by_their_json_type() {
        for (json, expected) in [
            (json!(7), Value::Days(7)),
            (json!("10002"), Value::One("10002".into())),
            (json!(["a", "b"]), Value::Many(vec!["a".into(), "b".into()])),
        ] {
            let condition: Condition =
                serde_json::from_value(json!({"field": "due", "op": "is", "value": json})).unwrap();
            assert_eq!(condition.value, Some(expected), "{json}");
        }
    }

    #[test]
    fn a_condition_without_a_value_leaves_the_value_out() {
        let condition = Condition::new(Field::Assignee, Op::IsEmpty, None);
        let json = serde_json::to_value(&condition).unwrap();
        assert_eq!(json, json!({"field": "assignee", "op": "isEmpty"}));
        assert_eq!(
            serde_json::from_value::<Condition>(json).unwrap(),
            condition
        );
    }

    #[test]
    fn no_stored_filter_is_no_condition() {
        for input in [None, Some(""), Some("  \n")] {
            assert_eq!(
                Filter::parse(input).unwrap(),
                Filter::default(),
                "{input:?}"
            );
        }
    }

    #[test]
    fn a_filter_that_cannot_be_read_is_invalid_input() {
        for input in [
            "not json",
            "{\"conditions\": 3}",
            "{\"conditions\":[{\"field\":\"status\"}]}",
        ] {
            let err = Filter::parse(Some(input)).unwrap_err();
            assert!(matches!(err, AppError::InvalidInput(_)), "{input}");
        }
    }

    #[test]
    fn an_empty_filter_is_stored_as_null() {
        assert_eq!(Filter::default().to_json().unwrap(), None);
    }

    #[test]
    fn a_filter_with_conditions_is_stored_as_json_text() {
        let filter = Filter {
            conditions: vec![status_is("10002")],
        };
        assert_eq!(
            filter.to_json().unwrap().as_deref(),
            Some(r#"{"conditions":[{"field":"status","op":"is","value":"10002"}]}"#)
        );
    }
}
