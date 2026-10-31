use std::collections::HashMap;

use serde::Deserialize;

/// `GET issue/{id}/editmeta`: the fields the issue's edit screen offers; a field it leaves out can't be changed.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub(crate) struct EditMeta {
    #[serde(default)]
    pub fields: HashMap<String, EditField>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditField {
    #[serde(default)]
    pub allowed_values: Vec<AllowedValue>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct AllowedValue {
    #[serde(default)]
    pub name: Option<String>,
}

impl EditMeta {
    /// The priorities the issue may be set to, in Jira's order; empty when its priority can't be changed.
    pub fn priorities(&self) -> Vec<String> {
        self.fields
            .get("priority")
            .map(|field| {
                field
                    .allowed_values
                    .iter()
                    .filter_map(|v| v.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn parse(value: serde_json::Value) -> EditMeta {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn priorities_are_the_allowed_names_in_jiras_order() {
        let meta = parse(json!({
            "fields": {
                "labels": { "allowedValues": [{ "name": "frontend" }] },
                "priority": {
                    "name": "Priority",
                    "allowedValues": [
                        { "id": "1", "name": "Highest" },
                        { "id": "3", "name": "Medium" },
                        { "id": "2", "name": "High" },
                    ],
                },
            }
        }));

        assert_eq!(meta.priorities(), ["Highest", "Medium", "High"]);
    }

    #[test]
    fn an_allowed_value_without_a_name_is_skipped() {
        let meta = parse(json!({
            "fields": { "priority": { "allowedValues": [{ "id": "1" }, { "name": "Low" }] } }
        }));

        assert_eq!(meta.priorities(), ["Low"]);
    }

    #[test]
    fn without_a_priority_field_or_allowed_values_there_are_no_priorities() {
        for (label, value) in [
            ("no fields", json!({})),
            (
                "another field only",
                json!({ "fields": { "labels": { "allowedValues": [{ "name": "frontend" }] } } }),
            ),
            (
                "a priority field without values",
                json!({ "fields": { "priority": { "name": "Priority" } } }),
            ),
        ] {
            assert_eq!(parse(value).priorities(), Vec::<String>::new(), "{label}");
        }
    }
}
