use std::collections::HashMap;

use serde::Deserialize;

/// `GET mypermissions`: which of the asked permissions the token's user has.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub(crate) struct MyPermissions {
    #[serde(default)]
    pub permissions: HashMap<String, Permission>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Permission {
    #[serde(default)]
    pub have_permission: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn jiras_answer_maps_each_permission_key_to_whether_it_is_granted() {
        let answer: MyPermissions = serde_json::from_value(json!({
            "permissions": {
                "EDIT_ISSUES": { "key": "EDIT_ISSUES", "name": "Edit", "havePermission": true },
                "ADD_COMMENTS": { "key": "ADD_COMMENTS", "havePermission": false },
            }
        }))
        .unwrap();

        let granted: HashMap<&str, bool> = answer
            .permissions
            .iter()
            .map(|(key, permission)| (key.as_str(), permission.have_permission))
            .collect();
        assert_eq!(
            granted,
            HashMap::from([("EDIT_ISSUES", true), ("ADD_COMMENTS", false)])
        );
    }

    #[test]
    fn a_missing_flag_or_permissions_object_means_nothing_is_granted() {
        let no_flag: MyPermissions =
            serde_json::from_value(json!({ "permissions": { "LINK_ISSUES": {} } })).unwrap();
        let empty: MyPermissions = serde_json::from_value(json!({})).unwrap();

        assert!(!no_flag.permissions["LINK_ISSUES"].have_permission);
        assert!(empty.permissions.is_empty());
    }
}
