//! Connecting to Jira against the loopback fake Jira on temp folders: saving the connection and its token,
//! Test Connection, the tracked projects and the sync scope.

use std::fs;
use std::sync::Arc;

use chrono::NaiveDate;
use serde_json::json;
use tempfile::TempDir;

use taskboard_lib::connection::{
    self, ConnectionInput, ConnectionTest, JiraAccess, TokenExpiry, effective_scope, list_projects,
    refresh_projects, save_scope, track_project, tracked_keys, validate_scope,
};
use taskboard_lib::db::Db;
use taskboard_lib::error::{AppError, ErrorKind};
use taskboard_lib::jira::{Limiter, TokenKind};
use taskboard_lib::secrets::SecretStore;
use taskboard_lib::store::settings::{SETTINGS_FILE, SettingsStore};
use taskboard_lib::test_support::demo::{CLOUD_ID, ME_EMAIL};
use taskboard_lib::test_support::fake_jira::{DEMO_TOKEN, FakeJira};
use taskboard_lib::test_support::{FakeSecrets, temp_config_dir};

const OTHER_EMAIL: &str = "sam.lee@example.com";
const ME_ACCOUNT_ID: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()
}

struct Rig {
    fake: FakeJira,
    dir: TempDir,
    settings: SettingsStore,
    access: JiraAccess,
    secrets: Arc<FakeSecrets>,
}

impl Rig {
    async fn new() -> Self {
        let dir = temp_config_dir();
        let secrets = Arc::new(FakeSecrets::default());
        Self {
            fake: FakeJira::start_demo().await,
            settings: SettingsStore::open(dir.path()),
            access: JiraAccess::new(secrets.clone(), Arc::new(Limiter::new(1000, 1000))),
            secrets,
            dir,
        }
    }

    fn input(&self) -> ConnectionInput {
        ConnectionInput {
            site_url: self.fake.url().to_string(),
            email: ME_EMAIL.into(),
            token_kind: TokenKind::Classic,
            token_expires: None,
            cloud_id: None,
        }
    }

    async fn save(
        &self,
        input: ConnectionInput,
        token: Option<&str>,
    ) -> Result<connection::ConnectionView, AppError> {
        connection::save(
            &self.settings,
            &self.access,
            input,
            token.map(String::from),
            today(),
        )
        .await
    }

    async fn connect(&self, token_kind: TokenKind, cloud_id: Option<&str>) {
        let input = ConnectionInput {
            token_kind,
            cloud_id: cloud_id.map(String::from),
            ..self.input()
        };
        self.save(input, Some(DEMO_TOKEN)).await.unwrap();
    }

    async fn test(&self) -> Result<ConnectionTest, AppError> {
        connection::test(&self.settings, &self.access).await
    }

    fn token_of(&self, email: &str) -> Option<String> {
        self.secrets
            .get(email)
            .unwrap()
            .map(|secret| secret.expose().to_string())
    }

    async fn fill_account_info(&self) {
        self.settings
            .edit(|s| {
                s.connection.cloud_id = Some("cloud-1".into());
                s.connection.account_id = Some("account-1".into());
                s.connection.display_name = Some("Alex Morgan".into());
                s.connection.time_zone = Some("Europe/Vienna".into());
            })
            .await
            .unwrap();
    }
}

fn invalid_field(err: AppError) -> String {
    match err {
        AppError::InvalidField { field, .. } => field,
        other => panic!("expected an invalid field, got {other:?}"),
    }
}

mod saving {
    use super::*;

    #[tokio::test]
    async fn the_token_goes_to_the_keychain_and_never_into_the_settings_file() {
        let rig = Rig::new().await;

        let view = rig.save(rig.input(), Some(DEMO_TOKEN)).await.unwrap();

        let file = fs::read_to_string(rig.dir.path().join(SETTINGS_FILE)).unwrap();
        assert!(file.contains(ME_EMAIL), "{file}");
        assert!(!file.contains(DEMO_TOKEN), "{file}");
        assert!(view.has_token);
        assert_eq!(rig.token_of(ME_EMAIL).as_deref(), Some(DEMO_TOKEN));
        assert_eq!(view.connection.email, ME_EMAIL);
        assert_eq!(view.connection.site_url, rig.fake.url().as_str());
        assert_eq!(view, connection::view(&rig.settings, &rig.access, today()));
    }

    #[tokio::test]
    async fn without_a_token_the_view_says_there_is_none() {
        let rig = Rig::new().await;

        let view = rig.save(rig.input(), None).await.unwrap();

        assert!(!view.has_token);
    }

    #[tokio::test]
    async fn a_blank_token_keeps_the_stored_one() {
        let rig = Rig::new().await;
        rig.save(rig.input(), Some(DEMO_TOKEN)).await.unwrap();

        let view = rig.save(rig.input(), Some("   ")).await.unwrap();

        assert!(view.has_token);
        assert_eq!(rig.token_of(ME_EMAIL).as_deref(), Some(DEMO_TOKEN));
    }

    #[tokio::test]
    async fn the_token_is_stored_without_surrounding_whitespace() {
        let rig = Rig::new().await;

        rig.save(rig.input(), Some("  demo-api-token\n"))
            .await
            .unwrap();

        assert_eq!(rig.token_of(ME_EMAIL).as_deref(), Some(DEMO_TOKEN));
    }

    #[tokio::test]
    async fn a_site_that_is_not_a_jira_cloud_address_names_the_site_field() {
        let rig = Rig::new().await;
        for site in ["", "https://example.com", "not a url at all"] {
            let input = ConnectionInput {
                site_url: site.into(),
                ..rig.input()
            };

            let err = rig.save(input, Some(DEMO_TOKEN)).await.unwrap_err();

            assert_eq!(invalid_field(err), "siteUrl", "{site:?}");
        }
        assert_eq!(rig.token_of(ME_EMAIL), None);
    }

    #[tokio::test]
    async fn an_email_without_an_at_sign_names_the_email_field_and_stores_nothing() {
        let rig = Rig::new().await;
        let input = ConnectionInput {
            email: "alex.morgan".into(),
            ..rig.input()
        };

        let err = rig.save(input, Some(DEMO_TOKEN)).await.unwrap_err();

        assert_eq!(invalid_field(err), "email");
        assert_eq!(rig.token_of("alex.morgan"), None);
        assert_eq!(rig.settings.get().connection.email, "");
    }

    #[tokio::test]
    async fn the_email_is_stored_trimmed() {
        let rig = Rig::new().await;
        let input = ConnectionInput {
            email: format!("  {ME_EMAIL} "),
            ..rig.input()
        };

        let view = rig.save(input, Some(DEMO_TOKEN)).await.unwrap();

        assert_eq!(view.connection.email, ME_EMAIL);
        assert_eq!(rig.token_of(ME_EMAIL).as_deref(), Some(DEMO_TOKEN));
    }

    #[tokio::test]
    async fn changing_the_site_forgets_what_jira_told_about_the_old_one() {
        let rig = Rig::new().await;
        rig.save(rig.input(), Some(DEMO_TOKEN)).await.unwrap();
        rig.fill_account_info().await;
        let other_site = FakeJira::start_demo().await;
        let input = ConnectionInput {
            site_url: other_site.url().to_string(),
            ..rig.input()
        };

        let view = rig.save(input, None).await.unwrap();

        assert_eq!(view.connection.site_url, other_site.url().as_str());
        assert_eq!(view.connection.cloud_id, None);
        assert_eq!(view.connection.account_id, None);
        assert_eq!(view.connection.display_name, None);
        assert_eq!(view.connection.time_zone, None);
    }

    #[tokio::test]
    async fn saving_the_same_account_again_keeps_what_jira_told() {
        let rig = Rig::new().await;
        rig.save(rig.input(), Some(DEMO_TOKEN)).await.unwrap();
        rig.fill_account_info().await;

        let view = rig.save(rig.input(), None).await.unwrap();

        assert_eq!(view.connection.cloud_id.as_deref(), Some("cloud-1"));
        assert_eq!(view.connection.account_id.as_deref(), Some("account-1"));
        assert_eq!(view.connection.display_name.as_deref(), Some("Alex Morgan"));
        assert_eq!(view.connection.time_zone.as_deref(), Some("Europe/Vienna"));
    }

    #[tokio::test]
    async fn changing_the_email_without_a_new_token_moves_the_token_and_forgets_the_account() {
        let rig = Rig::new().await;
        rig.save(rig.input(), Some(DEMO_TOKEN)).await.unwrap();
        rig.fill_account_info().await;
        let input = ConnectionInput {
            email: OTHER_EMAIL.into(),
            ..rig.input()
        };

        let view = rig.save(input, None).await.unwrap();

        assert!(view.has_token);
        assert_eq!(view.connection.email, OTHER_EMAIL);
        assert_eq!(rig.token_of(OTHER_EMAIL).as_deref(), Some(DEMO_TOKEN));
        assert_eq!(rig.token_of(ME_EMAIL), None);
        assert_eq!(view.connection.account_id, None);
        assert_eq!(view.connection.display_name, None);
        assert_eq!(view.connection.time_zone, None);
    }

    #[tokio::test]
    async fn changing_the_email_with_a_new_token_stores_it_under_the_new_email() {
        let rig = Rig::new().await;
        rig.save(rig.input(), Some(DEMO_TOKEN)).await.unwrap();
        let input = ConnectionInput {
            email: OTHER_EMAIL.into(),
            ..rig.input()
        };

        rig.save(input, Some("another-token")).await.unwrap();

        assert_eq!(rig.token_of(OTHER_EMAIL).as_deref(), Some("another-token"));
        assert_eq!(rig.token_of(ME_EMAIL).as_deref(), Some(DEMO_TOKEN));
    }

    #[tokio::test]
    async fn a_typed_cloud_id_is_kept_trimmed_and_an_empty_one_clears_it() {
        let rig = Rig::new().await;
        let typed = ConnectionInput {
            token_kind: TokenKind::Scoped,
            cloud_id: Some("  cloud-9 ".into()),
            ..rig.input()
        };
        let view = rig.save(typed, Some(DEMO_TOKEN)).await.unwrap();
        assert_eq!(view.connection.cloud_id.as_deref(), Some("cloud-9"));

        let untouched = ConnectionInput {
            token_kind: TokenKind::Scoped,
            cloud_id: None,
            ..rig.input()
        };
        let view = rig.save(untouched, None).await.unwrap();
        assert_eq!(view.connection.cloud_id.as_deref(), Some("cloud-9"));

        let cleared = ConnectionInput {
            token_kind: TokenKind::Scoped,
            cloud_id: Some("  ".into()),
            ..rig.input()
        };
        let view = rig.save(cleared, None).await.unwrap();
        assert_eq!(view.connection.cloud_id, None);
    }

    #[tokio::test]
    async fn the_expiry_is_counted_from_today() {
        let rig = Rig::new().await;
        for (days, expected) in [
            (None, TokenExpiry::Fine),
            (Some(30), TokenExpiry::Fine),
            (Some(3), TokenExpiry::Soon { days: 3 }),
            (Some(0), TokenExpiry::Soon { days: 0 }),
            (Some(-1), TokenExpiry::Expired),
        ] {
            let input = ConnectionInput {
                token_expires: days.map(|d| today() + chrono::Duration::days(d)),
                ..rig.input()
            };

            let view = rig.save(input, Some(DEMO_TOKEN)).await.unwrap();

            assert_eq!(view.expiry, expected, "{days:?}");
        }
    }

    #[tokio::test]
    async fn the_view_serializes_for_the_page_without_a_token() {
        let rig = Rig::new().await;
        let input = ConnectionInput {
            token_expires: Some(today() + chrono::Duration::days(3)),
            ..rig.input()
        };

        let view = rig.save(input, Some(DEMO_TOKEN)).await.unwrap();

        assert_eq!(
            serde_json::to_value(&view).unwrap(),
            json!({
                "connection": {
                    "siteUrl": rig.fake.url().as_str(),
                    "email": ME_EMAIL,
                    "tokenKind": "classic",
                    "tokenExpires": "2026-10-31",
                    "cloudId": null,
                    "accountId": null,
                    "displayName": null,
                    "timeZone": null,
                },
                "hasToken": true,
                "expiry": { "state": "soon", "days": 3 },
            })
        );
    }
}

mod testing_the_connection {
    use super::*;

    #[tokio::test]
    async fn a_classic_token_finds_the_account_and_stores_it() {
        let rig = Rig::new().await;
        rig.connect(TokenKind::Classic, None).await;

        let tested = rig.test().await.unwrap();

        assert_eq!(
            tested,
            ConnectionTest {
                display_name: "Alex Morgan".into(),
                account_id: ME_ACCOUNT_ID.into(),
                time_zone: "Europe/Vienna".into(),
                cloud_id: None,
            }
        );
        let stored = rig.settings.get().connection;
        assert_eq!(stored.account_id.as_deref(), Some(ME_ACCOUNT_ID));
        assert_eq!(stored.display_name.as_deref(), Some("Alex Morgan"));
        assert_eq!(stored.time_zone.as_deref(), Some("Europe/Vienna"));
        assert_eq!(rig.fake.paths(), ["/rest/api/3/myself"]);
    }

    #[tokio::test]
    async fn the_stored_account_survives_a_reopen() {
        let rig = Rig::new().await;
        rig.connect(TokenKind::Classic, None).await;
        rig.test().await.unwrap();

        let reopened = SettingsStore::open(rig.dir.path()).get().connection;

        assert_eq!(reopened.account_id.as_deref(), Some(ME_ACCOUNT_ID));
    }

    #[tokio::test]
    async fn a_scoped_token_without_a_cloud_id_resolves_it_through_the_gateway_path() {
        let rig = Rig::new().await;
        rig.connect(TokenKind::Scoped, None).await;

        let tested = rig.test().await.unwrap();

        assert_eq!(tested.cloud_id.as_deref(), Some(CLOUD_ID));
        assert_eq!(tested.display_name, "Alex Morgan");
        let stored = rig.settings.get().connection;
        assert_eq!(stored.cloud_id.as_deref(), Some(CLOUD_ID));
        assert_eq!(stored.account_id.as_deref(), Some(ME_ACCOUNT_ID));
        assert_eq!(
            rig.fake.paths(),
            [
                "/_edge/tenant_info".to_string(),
                format!("/ex/jira/{CLOUD_ID}/rest/api/3/myself"),
            ]
        );
    }

    #[tokio::test]
    async fn a_scoped_token_with_a_typed_cloud_id_does_not_ask_the_site_for_it() {
        let rig = Rig::new().await;
        rig.connect(TokenKind::Scoped, Some(CLOUD_ID)).await;

        let tested = rig.test().await.unwrap();

        assert_eq!(tested.cloud_id.as_deref(), Some(CLOUD_ID));
        assert_eq!(
            rig.fake.paths(),
            [format!("/ex/jira/{CLOUD_ID}/rest/api/3/myself")]
        );
    }

    #[tokio::test]
    async fn a_rejected_token_is_an_auth_error_and_stores_nothing() {
        let rig = Rig::new().await;
        rig.connect(TokenKind::Classic, None).await;
        rig.fake.fail_next(1, 401, None);

        let err = rig.test().await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Auth);
        let stored = rig.settings.get().connection;
        assert_eq!(stored.account_id, None);
        assert_eq!(stored.display_name, None);
        assert_eq!(stored.time_zone, None);
    }

    #[tokio::test]
    async fn a_scoped_site_that_does_not_tell_its_cloud_id_is_not_found() {
        let rig = Rig::new().await;
        rig.connect(TokenKind::Scoped, None).await;
        rig.fake.answer_next(404, json!({}));

        let err = rig.test().await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(rig.settings.get().connection.cloud_id, None);
        assert_eq!(rig.fake.paths(), ["/_edge/tenant_info"]);
    }

    #[tokio::test]
    async fn an_unreachable_site_is_offline() {
        for kind in [TokenKind::Classic, TokenKind::Scoped] {
            let rig = Rig::new().await;
            rig.connect(kind, None).await;
            rig.fake.go_offline().await;

            let err = rig.test().await.unwrap_err();

            assert_eq!(err.kind(), ErrorKind::Offline, "{kind:?}");
            assert_eq!(rig.settings.get().connection.account_id, None, "{kind:?}");
        }
    }

    #[tokio::test]
    async fn without_a_saved_connection_nothing_is_sent() {
        let rig = Rig::new().await;

        let err = rig.test().await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotConnected);
        assert!(rig.fake.requests().is_empty());
    }

    #[tokio::test]
    async fn without_a_token_the_connection_is_not_connected() {
        let rig = Rig::new().await;
        rig.save(rig.input(), None).await.unwrap();

        let err = rig.test().await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotConnected);
        assert!(rig.fake.requests().is_empty());
    }
}

mod projects {
    use super::*;

    type SpaceRow = (String, bool, Option<String>, Option<String>, i64);

    const NW: &str = "10001";
    const NWS: &str = "10002";

    async fn open() -> (TempDir, Db, FakeJira) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        (dir, db, FakeJira::start_demo().await)
    }

    async fn spaces(db: &Db) -> Vec<SpaceRow> {
        db.read(|conn| {
            let mut query = conn.prepare(
                "SELECT project_id, tracked, icon, color, position FROM space ORDER BY position",
            )?;
            let rows = query.query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?;
            Ok(rows.collect::<Result<Vec<SpaceRow>, _>>()?)
        })
        .await
        .unwrap()
    }

    fn space(id: &str, tracked: bool, icon: &str, color: &str, position: i64) -> SpaceRow {
        (
            id.into(),
            tracked,
            Some(icon.into()),
            Some(color.into()),
            position,
        )
    }

    #[tokio::test]
    async fn refreshing_caches_all_projects_sorted_by_name_and_untracked() {
        let (_dir, db, fake) = open().await;

        let rows = refresh_projects(&db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();

        let listed: Vec<(&str, &str, &str, bool)> = rows
            .iter()
            .map(|p| (p.id.as_str(), p.key.as_str(), p.name.as_str(), p.tracked))
            .collect();
        assert_eq!(
            listed,
            [
                ("10006", "ARC", "Archive 2024", false),
                ("10004", "FBA", "Fernweh booking API", false),
                ("10005", "INT", "Internal tools", false),
                ("10003", "LUM", "Lumen iOS app", false),
                ("10002", "NWS", "Northwind support", false),
                ("10001", "NW", "Website relaunch", false),
            ]
        );
        assert_eq!(list_projects(&db).await.unwrap(), rows);
    }

    #[tokio::test]
    async fn refreshing_drops_projects_jira_no_longer_lists_and_keeps_the_choice() {
        let (_dir, db, fake) = open().await;
        let client = fake.client(TokenKind::Classic);
        refresh_projects(&db, &client).await.unwrap();
        db.write(|tx, _| {
            tx.execute(
                "INSERT INTO cache.project (id, key, name, raw) VALUES ('99999', 'OLD', 'Old site', '{}')",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
        track_project(&db, NW.into(), true).await.unwrap();

        let rows = refresh_projects(&db, &client).await.unwrap();

        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|p| p.key != "OLD"));
        let tracked: Vec<&str> = rows
            .iter()
            .filter(|p| p.tracked)
            .map(|p| p.key.as_str())
            .collect();
        assert_eq!(tracked, ["NW"]);
    }

    #[tokio::test]
    async fn the_first_tracked_project_becomes_the_first_space() {
        let (_dir, db, fake) = open().await;
        refresh_projects(&db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();

        track_project(&db, NW.into(), true).await.unwrap();

        assert_eq!(spaces(&db).await, [space(NW, true, "W", "1", 0)]);
    }

    #[tokio::test]
    async fn further_spaces_get_the_next_position_and_color() {
        let (_dir, db, fake) = open().await;
        refresh_projects(&db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();

        track_project(&db, NW.into(), true).await.unwrap();
        track_project(&db, NWS.into(), true).await.unwrap();

        assert_eq!(
            spaces(&db).await,
            [space(NW, true, "W", "1", 0), space(NWS, true, "N", "2", 1)]
        );
    }

    #[tokio::test]
    async fn the_tile_colors_cycle_after_five_spaces() {
        let (_dir, db, fake) = open().await;
        refresh_projects(&db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();

        for id in ["10001", "10002", "10003", "10004", "10005", "10006"] {
            track_project(&db, id.into(), true).await.unwrap();
        }

        let colors: Vec<String> = spaces(&db)
            .await
            .into_iter()
            .map(|row| row.3.unwrap())
            .collect();
        assert_eq!(colors, ["1", "2", "3", "4", "5", "1"]);
    }

    #[tokio::test]
    async fn untracking_keeps_the_row_and_tracking_again_keeps_its_position() {
        let (_dir, db, fake) = open().await;
        refresh_projects(&db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();
        track_project(&db, NW.into(), true).await.unwrap();
        track_project(&db, NWS.into(), true).await.unwrap();

        track_project(&db, NW.into(), false).await.unwrap();
        assert_eq!(
            spaces(&db).await,
            [space(NW, false, "W", "1", 0), space(NWS, true, "N", "2", 1)]
        );

        track_project(&db, NW.into(), true).await.unwrap();
        assert_eq!(
            spaces(&db).await,
            [space(NW, true, "W", "1", 0), space(NWS, true, "N", "2", 1)]
        );
    }

    #[tokio::test]
    async fn tracked_keys_follow_the_space_order_and_skip_untracked_projects() {
        let (_dir, db, fake) = open().await;
        refresh_projects(&db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();
        assert!(tracked_keys(&db).await.unwrap().is_empty());

        track_project(&db, NWS.into(), true).await.unwrap();
        track_project(&db, NW.into(), true).await.unwrap();
        track_project(&db, "10003".into(), true).await.unwrap();
        track_project(&db, "10003".into(), false).await.unwrap();

        assert_eq!(tracked_keys(&db).await.unwrap(), ["NWS", "NW"]);
    }

    #[tokio::test]
    async fn the_project_list_reflects_the_tracked_flag() {
        let (_dir, db, fake) = open().await;
        refresh_projects(&db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();

        track_project(&db, NW.into(), true).await.unwrap();
        track_project(&db, NWS.into(), true).await.unwrap();
        track_project(&db, NWS.into(), false).await.unwrap();

        let flags: Vec<(String, bool)> = list_projects(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|p| (p.key, p.tracked))
            .collect();
        assert_eq!(
            flags,
            [
                ("ARC".to_string(), false),
                ("FBA".to_string(), false),
                ("INT".to_string(), false),
                ("LUM".to_string(), false),
                ("NWS".to_string(), false),
                ("NW".to_string(), true),
            ]
        );
    }

    #[tokio::test]
    async fn an_offline_site_fails_the_refresh_and_keeps_the_cache() {
        let (_dir, db, fake) = open().await;
        let client = fake.client(TokenKind::Classic);
        refresh_projects(&db, &client).await.unwrap();
        fake.go_offline().await;

        let err = refresh_projects(&db, &client).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Offline);
        assert_eq!(list_projects(&db).await.unwrap().len(), 6);
    }

    #[tokio::test]
    async fn project_rows_serialize_for_the_page() {
        let (_dir, db, fake) = open().await;
        refresh_projects(&db, &fake.client(TokenKind::Classic))
            .await
            .unwrap();

        let rows = list_projects(&db).await.unwrap();

        assert_eq!(
            serde_json::to_value(&rows[0]).unwrap(),
            json!({ "id": "10006", "key": "ARC", "name": "Archive 2024", "tracked": false })
        );
    }
}

mod scope {
    use super::*;

    fn problem(query: &str) -> String {
        format!("Error in the JQL Query: '{query}' could not be parsed.")
    }

    #[tokio::test]
    async fn a_query_jira_accepts_validates() {
        let fake = FakeJira::start_demo().await;

        let result = validate_scope(&fake.client(TokenKind::Classic), "project = NW").await;

        assert_eq!(result, Ok(()));
    }

    #[tokio::test]
    async fn a_query_jira_rejects_carries_jiras_messages() {
        let fake = FakeJira::start_demo().await;

        let err = validate_scope(&fake.client(TokenKind::Classic), "project = (")
            .await
            .unwrap_err();

        assert_eq!(err, AppError::InvalidJql(vec![problem("project = (")]));
    }

    #[tokio::test]
    async fn an_empty_query_is_rejected_without_asking_jira() {
        let fake = FakeJira::start_demo().await;
        let client = fake.client(TokenKind::Classic);

        for query in ["", "   "] {
            let err = validate_scope(&client, query).await.unwrap_err();

            assert_eq!(
                err,
                AppError::InvalidJql(vec!["Enter a JQL query".into()]),
                "{query:?}"
            );
        }
        assert!(fake.requests().is_empty());
    }

    #[tokio::test]
    async fn a_valid_override_is_saved_trimmed_and_becomes_the_scope() {
        let rig = Rig::new().await;
        let client = rig.fake.client(TokenKind::Classic);

        let sync = save_scope(
            &rig.settings,
            Some(&client),
            Some("  project = NW  ".into()),
        )
        .await
        .unwrap();

        assert_eq!(sync.scope_override.as_deref(), Some("project = NW"));
        assert_eq!(rig.settings.get().sync, sync);
        assert_eq!(
            SettingsStore::open(rig.dir.path())
                .get()
                .sync
                .scope_override
                .as_deref(),
            Some("project = NW")
        );
        let view = effective_scope(&sync, &["LUM".to_string()]);
        assert_eq!(view.jql, "project = NW");
        assert!(!view.is_default);
    }

    #[tokio::test]
    async fn an_invalid_override_is_rejected_and_the_previous_scope_stays() {
        let rig = Rig::new().await;
        let client = rig.fake.client(TokenKind::Classic);

        let first = save_scope(&rig.settings, Some(&client), Some("project = NW".into())).await;
        let err = save_scope(&rig.settings, Some(&client), Some("project = (".into()))
            .await
            .unwrap_err();

        assert!(first.is_ok());
        assert_eq!(err, AppError::InvalidJql(vec![problem("project = (")]));
        assert_eq!(
            rig.settings.get().sync.scope_override.as_deref(),
            Some("project = NW")
        );
    }

    #[tokio::test]
    async fn an_invalid_override_stores_nothing_when_there_was_none() {
        let rig = Rig::new().await;
        let client = rig.fake.client(TokenKind::Classic);

        let result = save_scope(&rig.settings, Some(&client), Some("project = (".into())).await;

        assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidJql);
        assert_eq!(rig.settings.get().sync.scope_override, None);
    }

    #[tokio::test]
    async fn saving_none_goes_back_to_the_default_without_asking_jira() {
        let rig = Rig::new().await;
        let client = rig.fake.client(TokenKind::Classic);
        save_scope(&rig.settings, Some(&client), Some("project = NW".into()))
            .await
            .unwrap();
        let asked = rig.fake.requests().len();

        let sync = save_scope(&rig.settings, None, None).await.unwrap();

        assert_eq!(sync.scope_override, None);
        assert_eq!(rig.settings.get().sync.scope_override, None);
        assert_eq!(rig.fake.requests().len(), asked);
        assert!(effective_scope(&sync, &["NW".to_string()]).is_default);
    }

    #[tokio::test]
    async fn an_override_without_a_client_is_not_connected_and_stores_nothing() {
        let rig = Rig::new().await;

        let err = save_scope(&rig.settings, None, Some("project = NW".into()))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotConnected);
        assert_eq!(rig.settings.get().sync.scope_override, None);
    }

    #[tokio::test]
    async fn saving_a_scope_leaves_the_connection_and_the_intervals_alone() {
        let rig = Rig::new().await;
        rig.save(rig.input(), Some(DEMO_TOKEN)).await.unwrap();
        let client = rig.fake.client(TokenKind::Classic);
        let before = rig.settings.get();

        save_scope(&rig.settings, Some(&client), Some("project = NW".into()))
            .await
            .unwrap();

        let after = rig.settings.get();
        assert_eq!(after.connection, before.connection);
        assert_eq!(after.sync.focused_minutes, before.sync.focused_minutes);
        assert_eq!(after.sync.keep_done_days, before.sync.keep_done_days);
    }
}
