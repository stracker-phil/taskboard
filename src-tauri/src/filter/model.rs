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
    /// The issue's own key; values are keys (`parse_keys`).
    IssueKey,
    /// The parent's key; values are keys (`parse_keys`).
    ParentKey,
    /// A fix version id.
    Release,
    /// `active`, `future`, `closed` or a sprint id.
    Sprint,
    Labels,
    Due,
    Updated,
    Created,
    /// When the owner last opened the issue in the Issue view (`activity`).
    LastView,
    /// When the owner last viewed, previewed, bookmarked or changed the issue in the app.
    LastInteraction,
    Project,
    /// Key, summary, description or comments hold every word (full-text search).
    Text,
    Custom(String),
}

const SYSTEM_FIELDS: [(Field, &str); 19] = [
    (Field::Status, "status"),
    (Field::StatusCategory, "statusCategory"),
    (Field::Type, "type"),
    (Field::Assignee, "assignee"),
    (Field::Reporter, "reporter"),
    (Field::Priority, "priority"),
    (Field::Epic, "epic"),
    (Field::IssueKey, "issueKey"),
    (Field::ParentKey, "parentKey"),
    (Field::Release, "release"),
    (Field::Sprint, "sprint"),
    (Field::Labels, "labels"),
    (Field::Due, "due"),
    (Field::Updated, "updated"),
    (Field::Created, "created"),
    (Field::LastView, "lastView"),
    (Field::LastInteraction, "lastInteraction"),
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
    /// The last n hours up to now; timestamps only.
    WithinLastHours,
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
            Op::WithinLastHours => "withinLastHours",
            Op::WithinNextDays => "withinNextDays",
        }
    }
}

/// Issue keys typed as a list: split on commas and whitespace, uppercased, duplicates dropped, in order.
/// A word that isn't a key (`NW-123`) is invalid input naming it.
pub fn parse_keys(text: &str) -> AppResult<Vec<String>> {
    let mut keys: Vec<String> = Vec::new();
    for word in text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|w| !w.is_empty())
    {
        let key = word.to_uppercase();
        if !is_key(&key) {
            return Err(AppError::InvalidInput(format!(
                "“{word}” isn't an issue key"
            )));
        }
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    Ok(keys)
}

/// `^[A-Z][A-Z0-9_]*-[0-9]+$`
fn is_key(key: &str) -> bool {
    let Some((project, number)) = key.split_once('-') else {
        return false;
    };
    let mut letters = project.chars();
    letters.next().is_some_and(|c| c.is_ascii_uppercase())
        && letters.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
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
            ("issueKey", Field::IssueKey),
            ("parentKey", Field::ParentKey),
            ("release", Field::Release),
            ("sprint", Field::Sprint),
            ("labels", Field::Labels),
            ("due", Field::Due),
            ("updated", Field::Updated),
            ("created", Field::Created),
            ("lastView", Field::LastView),
            ("lastInteraction", Field::LastInteraction),
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

    fn keys(text: &str) -> Vec<String> {
        parse_keys(text).unwrap()
    }

    #[test]
    fn keys_split_on_commas_and_whitespace() {
        for (text, expected) in [
            ("NW-1", vec!["NW-1"]),
            ("NW-1, NW-2", vec!["NW-1", "NW-2"]),
            ("NW-1,NW-2", vec!["NW-1", "NW-2"]),
            (
                "NW-1 NW-2\nOPS-7\t NW-3",
                vec!["NW-1", "NW-2", "OPS-7", "NW-3"],
            ),
            (" , NW-1 ,, OPS-7 , ", vec!["NW-1", "OPS-7"]),
            ("web2_app-15", vec!["WEB2_APP-15"]),
        ] {
            assert_eq!(keys(text), expected, "{text:?}");
        }
    }

    #[test]
    fn keys_are_uppercased() {
        assert_eq!(keys("nw-1, Ops-7"), ["NW-1", "OPS-7"]);
    }

    #[test]
    fn duplicate_keys_are_dropped_keeping_the_first_position() {
        assert_eq!(
            keys("NW-2, NW-1, nw-2, NW-1, OPS-7"),
            ["NW-2", "NW-1", "OPS-7"]
        );
    }

    #[test]
    fn no_keys_in_blank_text_is_an_empty_list() {
        for text in ["", "   ", " , ,\n"] {
            assert!(keys(text).is_empty(), "{text:?}");
        }
    }

    #[test]
    fn a_word_that_is_not_a_key_is_invalid_input_naming_it() {
        for word in [
            "NW", "NW-", "-12", "NW-1a", "1W-5", "_W-5", "NW_1", "NW-1-2", "N W-1",
        ] {
            let first_bad = word.split_whitespace().next().unwrap();
            let err = parse_keys(&format!("NW-1, {word}")).unwrap_err();
            assert!(
                matches!(&err, AppError::InvalidInput(message) if message.contains(first_bad)),
                "{word}: {err:?}"
            );
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
            (Op::WithinLastHours, "withinLastHours"),
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
