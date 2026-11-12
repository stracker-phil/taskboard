//! What changed between the cached and the fetched version of an issue: the rows of `issue_change`.

use super::map::IssueRow;
use crate::notify::people::named_in;

/// The tracked fields of the cached version.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub status_id: String,
    pub assignee_id: Option<String>,
    pub due_date: Option<String>,
    pub priority: Option<String>,
    pub comment_count: i64,
    /// The custom field values as a JSON object keyed by field id.
    pub custom: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChangeRow {
    /// `status`, `assignee`, `due_date`, `priority`, `comment_added`, `created`, `updated` or `people`.
    pub field: &'static str,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    /// The new version's `updated`.
    pub changed_at: String,
}

impl ChangeRow {
    /// How many comments a `comment_added` row counts; 0 for any other.
    pub fn added_comments(&self) -> i64 {
        let count = |v: &Option<String>| v.as_deref().and_then(|n| n.parse::<i64>().ok());
        match (self.field, count(&self.old_value), count(&self.new_value)) {
            ("comment_added", Some(old), Some(new)) => new - old,
            _ => 0,
        }
    }
}

/// `old` is `None` for an issue new to the cache.
pub fn diff(old: Option<&Snapshot>, new: &IssueRow) -> Vec<ChangeRow> {
    let change = |field, old_value: Option<String>, new_value: Option<String>| ChangeRow {
        field,
        old_value,
        new_value,
        changed_at: new.updated_at.clone(),
    };
    let Some(old) = old else {
        return vec![change("created", None, Some(new.key.clone()))];
    };
    let mut changes = Vec::new();
    if old.status_id != new.status_id {
        changes.push(change(
            "status",
            Some(old.status_id.clone()),
            Some(new.status_id.clone()),
        ));
    }
    if old.assignee_id != new.assignee_id {
        changes.push(change(
            "assignee",
            old.assignee_id.clone(),
            new.assignee_id.clone(),
        ));
    }
    if old.due_date != new.due_date {
        changes.push(change(
            "due_date",
            old.due_date.clone(),
            new.due_date.clone(),
        ));
    }
    if old.priority != new.priority {
        changes.push(change(
            "priority",
            old.priority.clone(),
            new.priority.clone(),
        ));
    }
    if new.comment_count > old.comment_count {
        changes.push(change(
            "comment_added",
            Some(old.comment_count.to_string()),
            Some(new.comment_count.to_string()),
        ));
    }
    changes
}

/// One `people` row per people field (`fields`) that newly names `me`: the field id as the old value, the
/// account as the new. Only a newer version counts: a field fetched for the first time (just opted in) fills
/// in values without Jira's `updated` moving.
pub fn people_added(old: &Snapshot, new: &IssueRow, me: &str, fields: &[String]) -> Vec<ChangeRow> {
    if new.updated_at <= old.updated_at {
        return Vec::new();
    }
    let before = named_in(old.custom.as_deref(), me, fields);
    named_in(new.custom.as_deref(), me, fields)
        .into_iter()
        .filter(|field| !before.contains(field))
        .map(|field| ChangeRow {
            field: "people",
            old_value: Some(field),
            new_value: Some(me.to_string()),
            changed_at: new.updated_at.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const UPDATED: &str = "2026-10-28T10:42:00.000Z";

    fn cached() -> Snapshot {
        Snapshot {
            status_id: "3".into(),
            assignee_id: Some("user-1".into()),
            due_date: Some("2026-10-30".into()),
            priority: Some("High".into()),
            comment_count: 4,
            ..Snapshot::default()
        }
    }

    fn fetched() -> IssueRow {
        IssueRow {
            id: "101042".into(),
            key: "NW-1042".into(),
            project_id: "10001".into(),
            parent_id: None,
            fix_versions: "[]".into(),
            epic_color: None,
            summary: "Checkout: address validation for DACH postcodes".into(),
            issue_type: "Story".into(),
            type_is_subtask: false,
            is_epic: false,
            status_id: "3".into(),
            status_category: "indeterminate".into(),
            priority: Some("High".into()),
            assignee_id: Some("user-1".into()),
            reporter_id: None,
            sprint: None,
            sprint_id: None,
            sprint_state: None,
            labels: "[]".into(),
            due_date: Some("2026-10-30".into()),
            created_at: "2026-10-12T07:14:00.000Z".into(),
            updated_at: UPDATED.into(),
            resolved_at: None,
            description_html: None,
            description_adf: None,
            comment_count: 4,
            comments_complete: true,
            custom: None,
            raw: "{}".into(),
        }
    }

    fn change(field: &'static str, old: Option<&str>, new: Option<&str>) -> ChangeRow {
        ChangeRow {
            field,
            old_value: old.map(String::from),
            new_value: new.map(String::from),
            changed_at: UPDATED.into(),
        }
    }

    #[test]
    fn an_issue_equal_to_the_cached_one_changes_nothing() {
        assert_eq!(diff(Some(&cached()), &fetched()), vec![]);
    }

    #[test]
    fn each_tracked_field_change_is_exactly_one_row_with_old_and_new() {
        type Edit = fn(&mut IssueRow);
        let cases: [(&str, Edit, ChangeRow); 9] = [
            (
                "status",
                |r| r.status_id = "10001".into(),
                change("status", Some("3"), Some("10001")),
            ),
            (
                "assignee reassigned",
                |r| r.assignee_id = Some("user-2".into()),
                change("assignee", Some("user-1"), Some("user-2")),
            ),
            (
                "assignee cleared",
                |r| r.assignee_id = None,
                change("assignee", Some("user-1"), None),
            ),
            (
                "due date moved",
                |r| r.due_date = Some("2026-11-02".into()),
                change("due_date", Some("2026-10-30"), Some("2026-11-02")),
            ),
            (
                "due date cleared",
                |r| r.due_date = None,
                change("due_date", Some("2026-10-30"), None),
            ),
            (
                "priority raised",
                |r| r.priority = Some("Highest".into()),
                change("priority", Some("High"), Some("Highest")),
            ),
            (
                "priority cleared",
                |r| r.priority = None,
                change("priority", Some("High"), None),
            ),
            (
                "one comment added",
                |r| r.comment_count = 5,
                change("comment_added", Some("4"), Some("5")),
            ),
            (
                "three comments added",
                |r| r.comment_count = 7,
                change("comment_added", Some("4"), Some("7")),
            ),
        ];
        for (label, edit, expected) in cases {
            let mut row = fetched();
            edit(&mut row);
            assert_eq!(diff(Some(&cached()), &row), vec![expected], "{label}");
        }
    }

    #[test]
    fn the_change_is_dated_with_the_new_versions_updated() {
        let mut row = fetched();
        row.status_id = "10001".into();
        row.updated_at = "2026-10-28T13:00:00.000Z".into();

        let changes = diff(Some(&cached()), &row);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].changed_at, "2026-10-28T13:00:00.000Z");
    }

    #[test]
    fn fewer_comments_than_cached_is_not_a_change() {
        for count in [3, 0] {
            let mut row = fetched();
            row.comment_count = count;
            assert_eq!(diff(Some(&cached()), &row), vec![], "{count}");
        }
    }

    #[test]
    fn fields_the_diff_does_not_track_change_nothing() {
        let mut row = fetched();
        row.summary = "Checkout: postcode validation".into();
        row.labels = r#"["frontend"]"#.into();
        row.sprint_id = Some("22".into());
        row.description_html = Some("<p>New text</p>".into());
        row.raw = r#"{"changed":true}"#.into();
        assert_eq!(diff(Some(&cached()), &row), vec![]);
    }

    #[test]
    fn several_changes_at_once_come_in_a_fixed_order() {
        let mut row = fetched();
        row.comment_count = 5;
        row.priority = Some("Low".into());
        row.due_date = None;
        row.assignee_id = None;
        row.status_id = "10001".into();

        assert_eq!(
            diff(Some(&cached()), &row),
            vec![
                change("status", Some("3"), Some("10001")),
                change("assignee", Some("user-1"), None),
                change("due_date", Some("2026-10-30"), None),
                change("priority", Some("High"), Some("Low")),
                change("comment_added", Some("4"), Some("5")),
            ]
        );
    }

    const ME: &str = "account-me";
    const DEVELOPER: &str = "customfield_10042";
    const REVIEWERS: &str = "customfield_10043";

    fn people_fields() -> Vec<String> {
        vec![DEVELOPER.to_string(), REVIEWERS.to_string()]
    }

    fn naming_me_in(fields: &[&str]) -> Option<String> {
        let values: serde_json::Map<String, serde_json::Value> = fields
            .iter()
            .map(|id| {
                let value = if *id == REVIEWERS {
                    serde_json::json!([{"accountId": "account-marta"}, {"accountId": ME}])
                } else {
                    serde_json::json!({"accountId": ME})
                };
                (id.to_string(), value)
            })
            .collect();
        Some(serde_json::Value::Object(values).to_string())
    }

    fn people_row(field: &str) -> ChangeRow {
        change("people", Some(field), Some(ME))
    }

    fn older(custom: Option<String>) -> Snapshot {
        Snapshot {
            custom,
            updated_at: "2026-10-27T08:00:00.000Z".into(),
            ..cached()
        }
    }

    fn newer(custom: Option<String>) -> IssueRow {
        IssueRow {
            custom,
            ..fetched()
        }
    }

    #[test]
    fn being_newly_named_in_a_people_field_is_one_row_with_the_field_and_the_account() {
        let rows = people_added(
            &older(None),
            &newer(naming_me_in(&[DEVELOPER])),
            ME,
            &people_fields(),
        );

        assert_eq!(rows, vec![people_row(DEVELOPER)]);
    }

    #[test]
    fn being_named_in_a_list_of_users_counts_too() {
        let rows = people_added(
            &older(None),
            &newer(naming_me_in(&[REVIEWERS])),
            ME,
            &people_fields(),
        );

        assert_eq!(rows, vec![people_row(REVIEWERS)]);
    }

    #[test]
    fn each_newly_named_field_is_its_own_row_in_field_order() {
        let rows = people_added(
            &older(None),
            &newer(naming_me_in(&[REVIEWERS, DEVELOPER])),
            ME,
            &people_fields(),
        );

        assert_eq!(rows, vec![people_row(DEVELOPER), people_row(REVIEWERS)]);
    }

    #[test]
    fn only_the_field_that_is_new_makes_a_row() {
        let rows = people_added(
            &older(naming_me_in(&[DEVELOPER])),
            &newer(naming_me_in(&[DEVELOPER, REVIEWERS])),
            ME,
            &people_fields(),
        );

        assert_eq!(rows, vec![people_row(REVIEWERS)]);
    }

    #[test]
    fn being_named_already_or_removed_makes_no_row() {
        let cases = [
            ("already named", naming_me_in(&[DEVELOPER]), naming_me_in(&[DEVELOPER])),
            ("removed", naming_me_in(&[DEVELOPER]), None),
            ("never named", None, None),
        ];
        for (label, before, after) in cases {
            assert_eq!(
                people_added(&older(before), &newer(after), ME, &people_fields()),
                vec![],
                "{label}"
            );
        }
    }

    #[test]
    fn a_version_that_is_not_newer_makes_no_row() {
        for updated in [UPDATED, "2026-10-28T11:00:00.000Z"] {
            let old = Snapshot {
                custom: None,
                updated_at: updated.into(),
                ..cached()
            };

            assert_eq!(
                people_added(
                    &old,
                    &newer(naming_me_in(&[DEVELOPER])),
                    ME,
                    &people_fields()
                ),
                vec![],
                "{updated}"
            );
        }
    }

    #[test]
    fn fields_outside_the_list_and_other_accounts_make_no_row() {
        let someone_else = Some(
            serde_json::json!({(DEVELOPER): {"accountId": "account-marta"}}).to_string(),
        );
        let other_field = Some(serde_json::json!({"customfield_10099": {"accountId": ME}}).to_string());

        for (label, custom) in [("someone else", someone_else), ("unlisted field", other_field)] {
            assert_eq!(
                people_added(&older(None), &newer(custom), ME, &people_fields()),
                vec![],
                "{label}"
            );
        }
        assert_eq!(
            people_added(&older(None), &newer(naming_me_in(&[DEVELOPER])), ME, &[]),
            vec![]
        );
    }

    #[test]
    fn an_issue_new_to_the_cache_is_one_created_row_with_its_key() {
        assert_eq!(
            diff(None, &fetched()),
            vec![change("created", None, Some("NW-1042"))]
        );
    }
}
