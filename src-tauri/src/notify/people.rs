//! People fields: opted-in custom fields of a user type (Developer, Reviewer, …). Naming the connected account in
//! one makes the owner involved, and being added to one is the "Adds me to a people field" event.

use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use crate::error::AppResult;

/// A people field, as Settings → Notifications lists it.
#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct PeopleField {
    /// The custom field's id (`customfield_10042`).
    pub id: String,
    pub name: String,
}

/// The opted-in (`field_choice`) custom fields holding one user or a list of users, by name.
pub fn people_fields(conn: &Connection) -> AppResult<Vec<PeopleField>> {
    let mut query = conn.prepare(
        "SELECT f.id, f.name FROM cache.field f JOIN field_choice c ON c.field_id = f.id
         WHERE f.type = 'user' OR (f.type = 'array' AND f.items = 'user')
         ORDER BY f.name COLLATE NOCASE, f.id",
    )?;
    let fields = query.query_map([], |r| {
        Ok(PeopleField {
            id: r.get(0)?,
            name: r.get(1)?,
        })
    })?;
    Ok(fields.collect::<Result<_, _>>()?)
}

pub fn people_field_ids(conn: &Connection) -> AppResult<Vec<String>> {
    Ok(people_fields(conn)?.into_iter().map(|f| f.id).collect())
}

/// The `fields` among an issue's custom values (`issue.custom`) that name `me`, as a user or in a list of users.
pub fn named_in(custom: Option<&str>, me: &str, fields: &[String]) -> Vec<String> {
    let Some(Value::Object(values)) = custom.and_then(|json| serde_json::from_str(json).ok())
    else {
        return Vec::new();
    };
    let is_me = |value: &Value| value.get("accountId").and_then(Value::as_str) == Some(me);
    fields
        .iter()
        .filter(|id| match values.get(id.as_str()) {
            Some(Value::Array(users)) => users.iter().any(is_me),
            Some(user) => is_me(user),
            None => false,
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::test_support::temp_config_dir;

    const ME: &str = "account-me";

    fn fields(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    const CUSTOM: &str = r#"{
        "customfield_10042": {"accountId": "account-me", "displayName": "Alex Morgan"},
        "customfield_10043": [{"accountId": "account-marta"}, {"accountId": "account-me"}],
        "customfield_10044": "account-me",
        "customfield_10045": {"accountId": "account-marta"},
        "customfield_10046": [{"accountId": "account-marta"}],
        "customfield_10047": []
    }"#;

    #[test]
    fn a_user_and_a_list_of_users_naming_me_are_found_in_field_order() {
        let all = fields(&[
            "customfield_10043",
            "customfield_10042",
            "customfield_10045",
            "customfield_10046",
            "customfield_10047",
        ]);

        assert_eq!(
            named_in(Some(CUSTOM), ME, &all),
            ["customfield_10043", "customfield_10042"]
        );
    }

    #[test]
    fn only_the_listed_fields_are_looked_at() {
        let only_list = fields(&["customfield_10043"]);

        assert_eq!(
            named_in(Some(CUSTOM), ME, &only_list),
            ["customfield_10043"]
        );
        assert!(named_in(Some(CUSTOM), ME, &[]).is_empty());
    }

    #[test]
    fn values_that_are_not_users_never_match() {
        let others = fields(&[
            "customfield_10044",
            "customfield_10045",
            "customfield_10046",
            "customfield_10047",
            "customfield_10099",
        ]);

        assert!(named_in(Some(CUSTOM), ME, &others).is_empty());
    }

    #[test]
    fn missing_or_unreadable_custom_values_name_nobody() {
        let listed = fields(&["customfield_10042"]);
        for custom in [None, Some(""), Some("not json"), Some("[]"), Some("null")] {
            assert!(named_in(custom, ME, &listed).is_empty(), "{custom:?}");
        }
    }

    async fn db_with_fields() -> (tempfile::TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| {
            tx.execute_batch(
                "INSERT INTO cache.field (id, name, type, items) VALUES
                   ('customfield_10042', 'Developer', 'user', NULL),
                   ('customfield_10043', 'reviewers', 'array', 'user'),
                   ('customfield_10044', 'Approver', 'user', NULL),
                   ('customfield_10045', 'Tester', 'user', NULL),
                   ('customfield_10046', 'Tags', 'array', 'string'),
                   ('customfield_10047', 'Estimate', 'number', NULL);
                 INSERT INTO field_choice (field_id, in_list, in_detail) VALUES
                   ('customfield_10042', 1, 1),
                   ('customfield_10043', 0, 1),
                   ('customfield_10044', 1, 0),
                   ('customfield_10046', 1, 1),
                   ('customfield_10047', 1, 1);",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        (dir, db)
    }

    #[tokio::test]
    async fn people_fields_are_the_opted_in_user_and_user_list_fields_by_name() {
        let (_dir, db) = db_with_fields().await;

        let found = db.read(|conn| people_fields(conn)).await.unwrap();

        assert_eq!(
            found,
            [
                PeopleField {
                    id: "customfield_10044".into(),
                    name: "Approver".into()
                },
                PeopleField {
                    id: "customfield_10042".into(),
                    name: "Developer".into()
                },
                PeopleField {
                    id: "customfield_10043".into(),
                    name: "reviewers".into()
                },
            ]
        );
    }

    #[tokio::test]
    async fn people_field_ids_follow_the_same_order() {
        let (_dir, db) = db_with_fields().await;

        let ids = db.read(|conn| people_field_ids(conn)).await.unwrap();

        assert_eq!(
            ids,
            ["customfield_10044", "customfield_10042", "customfield_10043"]
        );
    }

    #[tokio::test]
    async fn without_opted_in_fields_there_are_no_people_fields() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();

        assert!(db.read(|conn| people_fields(conn)).await.unwrap().is_empty());
    }

    #[test]
    fn a_people_field_serializes_with_its_id_and_name() {
        let field = PeopleField {
            id: "customfield_10042".into(),
            name: "Developer".into(),
        };

        assert_eq!(
            serde_json::to_value(&field).unwrap(),
            serde_json::json!({"id": "customfield_10042", "name": "Developer"})
        );
    }
}
