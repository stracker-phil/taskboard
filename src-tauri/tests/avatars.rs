//! Profile pictures: what `user_avatar` hands the page, and when it asks the avatar host again, on a temp
//! database with made-up people and pictures.

use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;
use tempfile::TempDir;

use taskboard_lib::avatars::{FRESH_FOR, RETRY_AFTER, user_avatar};
use taskboard_lib::db::Db;
use taskboard_lib::jira::gateway::{Image, ImageKind};
use taskboard_lib::net::NetError;
use taskboard_lib::test_support::temp_config_dir;

const PERSON: &str = "712020:0a1b2c3d-0000-4000-8000-00000000aaaa";
const URL: &str =
    "https://avatar-management--avatars.us-west-2.prod.public.atl-paas.net/712020:aaaa/0f1e2d3c/48";
const NEW_URL: &str =
    "https://avatar-management--avatars.us-west-2.prod.public.atl-paas.net/712020:aaaa/9e8d7c6b/48";
const FIRST: &[u8] = b"\x89PNG\r\n\x1a\nfirst";
const SECOND: &[u8] = b"\x89PNG\r\n\x1a\nsecond";

struct Fixture {
    db: Db,
    _dir: TempDir,
}

fn now() -> DateTime<Utc> {
    "2026-10-20T09:00:00Z".parse().unwrap()
}

fn png(bytes: &[u8]) -> Image {
    Image {
        kind: ImageKind::Png,
        bytes: bytes.to_vec(),
    }
}

fn shown(bytes: &[u8]) -> Option<String> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

async fn fixture(avatar_url: Option<&str>) -> Fixture {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    let fixture = Fixture { db, _dir: dir };
    set_url(&fixture, avatar_url).await;
    fixture
}

async fn set_url(fixture: &Fixture, avatar_url: Option<&str>) {
    let url = avatar_url.map(str::to_owned);
    fixture
        .db
        .write(move |tx, _| {
            tx.execute(
                "INSERT INTO cache.user (account_id, name, avatar_url) VALUES (?1, 'Robin Vale', ?2)
                 ON CONFLICT (account_id) DO UPDATE SET avatar_url = excluded.avatar_url",
                params![PERSON, url],
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn ask(fixture: &Fixture, at: DateTime<Utc>, answer: &[u8]) -> Option<String> {
    let image = png(answer);
    user_avatar(&fixture.db, PERSON.into(), at, |_| async move { Ok(image) })
        .await
        .unwrap()
}

async fn ask_failing(fixture: &Fixture, at: DateTime<Utc>) -> Option<String> {
    user_avatar(&fixture.db, PERSON.into(), at, |_| async {
        Err(NetError::Offline("no connection".into()))
    })
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn the_first_ask_fetches_the_picture_and_returns_it_as_a_data_url() {
    let fixture = fixture(Some(URL)).await;

    assert_eq!(ask(&fixture, now(), FIRST).await, shown(FIRST));
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn the_fetch_asks_for_the_stored_address() {
    let fixture = fixture(Some(URL)).await;

    let asked = user_avatar(&fixture.db, PERSON.into(), now(), |url| async move {
        assert_eq!(url.as_str(), URL);
        Ok(png(FIRST))
    })
    .await
    .unwrap();

    assert_eq!(asked, shown(FIRST));
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn an_unknown_person_or_one_without_an_address_shows_initials() {
    let unknown = fixture(Some(URL)).await;
    let image = png(FIRST);
    let answer = user_avatar(&unknown.db, "712020:nobody".into(), now(), |_| async move {
        Ok(image)
    })
    .await
    .unwrap();
    assert_eq!(answer, None);

    let without = fixture(None).await;
    assert_eq!(ask(&without, now(), FIRST).await, None);
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn an_address_off_the_avatar_host_is_never_fetched() {
    for url in [
        "https://secure.gravatar.com/avatar/0f1e2d3c?d=mm",
        "http://127.0.0.1:47470/avatar/48.png",
        "https://avatar-management--avatars.us-west-2.prod.public.atl-paas.net.evil.example.com/48",
    ] {
        let fixture = fixture(Some(url)).await;
        assert_eq!(ask(&fixture, now(), FIRST).await, None, "{url}");
    }
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn a_picture_is_kept_for_a_week_then_fetched_again() {
    let fixture = fixture(Some(URL)).await;
    ask(&fixture, now(), FIRST).await;

    let within = now() + FRESH_FOR - Duration::minutes(1);
    assert_eq!(ask(&fixture, within, SECOND).await, shown(FIRST));

    let after = now() + FRESH_FOR;
    assert_eq!(ask(&fixture, after, SECOND).await, shown(SECOND));
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn a_new_address_is_fetched_at_once() {
    let fixture = fixture(Some(URL)).await;
    ask(&fixture, now(), FIRST).await;

    set_url(&fixture, Some(NEW_URL)).await;

    let soon = now() + Duration::minutes(1);
    assert_eq!(ask(&fixture, soon, SECOND).await, shown(SECOND));
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn a_failed_fetch_shows_initials_and_is_not_repeated_for_a_day() {
    let fixture = fixture(Some(URL)).await;
    assert_eq!(ask_failing(&fixture, now()).await, None);

    let within = now() + RETRY_AFTER - Duration::minutes(1);
    assert_eq!(ask(&fixture, within, FIRST).await, None);

    let after = now() + RETRY_AFTER;
    assert_eq!(ask(&fixture, after, FIRST).await, shown(FIRST));
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn a_failed_refetch_keeps_the_last_picture() {
    let fixture = fixture(Some(URL)).await;
    ask(&fixture, now(), FIRST).await;

    let stale = now() + FRESH_FOR;
    assert_eq!(ask_failing(&fixture, stale).await, shown(FIRST));

    let soon = stale + Duration::minutes(1);
    assert_eq!(ask(&fixture, soon, SECOND).await, shown(FIRST));
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn a_failure_for_a_new_address_keeps_the_old_picture_until_the_new_one_arrives() {
    let fixture = fixture(Some(URL)).await;
    ask(&fixture, now(), FIRST).await;
    set_url(&fixture, Some(NEW_URL)).await;

    assert_eq!(ask_failing(&fixture, now()).await, shown(FIRST));

    let after = now() + RETRY_AFTER;
    assert_eq!(ask(&fixture, after, SECOND).await, shown(SECOND));
}

#[tokio::test]
#[ignore = "uses a temp database"]
async fn a_person_whose_address_is_gone_shows_initials_again() {
    let fixture = fixture(Some(URL)).await;
    ask(&fixture, now(), FIRST).await;

    set_url(&fixture, None).await;

    assert_eq!(ask(&fixture, now(), SECOND).await, None);
}
