//! The sync scope: one JQL query that decides which issues live in the cache.

use serde::Serialize;
use ts_rs::TS;

use super::projects::tracked_keys;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::{JiraClient, jql};
use crate::store::settings::{SettingsStore, SyncSettings};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ScopeView {
    /// Empty while nothing is tracked and there is no override.
    pub jql: String,
    pub is_default: bool,
}

/// The override, or the default over the tracked projects.
pub fn effective_scope(sync: &SyncSettings, tracked_keys: &[String]) -> ScopeView {
    match &sync.scope_override {
        Some(jql) => ScopeView {
            jql: jql.clone(),
            is_default: false,
        },
        None => ScopeView {
            jql: jql::default_scope(tracked_keys, sync.keep_done_days).unwrap_or_default(),
            is_default: true,
        },
    }
}

/// The scope as the Sync page shows it.
pub async fn scope_view(db: &Db, sync: &SyncSettings) -> AppResult<ScopeView> {
    Ok(effective_scope(sync, &tracked_keys(db).await?))
}

/// Saves everything but the scope, which changes only through `save_scope`.
pub async fn save_intervals(
    settings: &SettingsStore,
    sync: SyncSettings,
) -> AppResult<SyncSettings> {
    sync.validate()?;
    let saved = settings
        .edit(move |s| {
            s.sync = SyncSettings {
                scope_override: s.sync.scope_override.take(),
                ..sync
            }
        })
        .await?;
    Ok(saved.sync)
}

/// Jira's strict check; its messages come back as `InvalidJql`.
pub async fn validate_scope(client: &JiraClient, query: &str) -> AppResult<()> {
    if query.trim().is_empty() {
        return Err(AppError::InvalidJql(vec!["Enter a JQL query".into()]));
    }
    let problems = client.check_jql(query).await?;
    if problems.is_empty() {
        Ok(())
    } else {
        Err(AppError::InvalidJql(problems))
    }
}

/// Saves the owner's scope after Jira accepted it; `None` goes back to the default.
pub async fn save_scope(
    settings: &SettingsStore,
    client: Option<&JiraClient>,
    query: Option<String>,
) -> AppResult<SyncSettings> {
    let query = query.map(|q| q.trim().to_string());
    if let Some(query) = &query {
        let client = client.ok_or_else(|| AppError::NotConnected("Connect Jira first".into()))?;
        validate_scope(client, query).await?;
    }
    let saved = settings
        .edit(move |s| s.sync.scope_override = query)
        .await?;
    Ok(saved.sync)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn without_an_override_the_scope_is_the_default_over_the_tracked_projects() {
        let sync = SyncSettings {
            keep_done_days: 30,
            ..SyncSettings::default()
        };
        let tracked = keys(&["NW", "LUM"]);

        assert_eq!(
            effective_scope(&sync, &tracked),
            ScopeView {
                jql: jql::default_scope(&tracked, 30).unwrap(),
                is_default: true,
            }
        );
    }

    #[test]
    fn without_tracked_projects_or_an_override_the_scope_is_empty() {
        assert_eq!(
            effective_scope(&SyncSettings::default(), &[]),
            ScopeView {
                jql: String::new(),
                is_default: true,
            }
        );
    }

    #[test]
    fn an_override_replaces_the_default_whatever_is_tracked() {
        let sync = SyncSettings {
            scope_override: Some("assignee = currentUser()".into()),
            ..SyncSettings::default()
        };

        for tracked in [keys(&[]), keys(&["NW"])] {
            assert_eq!(
                effective_scope(&sync, &tracked),
                ScopeView {
                    jql: "assignee = currentUser()".into(),
                    is_default: false,
                },
                "{tracked:?}"
            );
        }
    }

    #[tokio::test]
    async fn saving_the_intervals_keeps_the_scope_override_and_saves_clear_on_quit() {
        let dir = crate::test_support::temp_config_dir();
        let settings = SettingsStore::open(dir.path());
        settings
            .edit(|s| s.sync.scope_override = Some("project = NW".into()))
            .await
            .unwrap();

        let saved = save_intervals(
            &settings,
            SyncSettings {
                scope_override: None,
                focused_minutes: 10,
                clear_cache_on_quit: true,
                ..SyncSettings::default()
            },
        )
        .await
        .unwrap();

        let expected = SyncSettings {
            scope_override: Some("project = NW".into()),
            focused_minutes: 10,
            clear_cache_on_quit: true,
            ..SyncSettings::default()
        };
        assert_eq!(saved, expected);
        assert_eq!(SettingsStore::open(dir.path()).get().sync, expected);
    }

    #[test]
    fn the_scope_view_uses_camel_case_names_for_the_page() {
        let view = ScopeView {
            jql: "project = NW".into(),
            is_default: false,
        };
        assert_eq!(
            serde_json::to_value(view).unwrap(),
            json!({ "jql": "project = NW", "isDefault": false })
        );
    }
}
