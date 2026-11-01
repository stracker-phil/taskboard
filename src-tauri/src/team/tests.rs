use serde_json::json;

use super::*;
use crate::error::ErrorKind;

const WEB: &str = "10001";
const SUPPORT: &str = "10002";

const ALEX: &str = "acc-alex";
const BEA: &str = "acc-bea";
const CLEO: &str = "acc-cleo";
const DAN: &str = "acc-dan";
const ELI: &str = "acc-eli";
const FINN: &str = "acc-finn";
const GUS: &str = "acc-gus";
const HAL: &str = "acc-hal";
const IVY: &str = "acc-ivy";

const ASSIGNEE_ON: TeamCell = TeamCell {
    active: true,
    assignee: true,
    involved: true,
};
const ASSIGNEE_OFF: TeamCell = TeamCell {
    active: false,
    assignee: true,
    involved: true,
};
const INVOLVED_OFF: TeamCell = TeamCell {
    active: false,
    assignee: false,
    involved: true,
};
const INVOLVED_ON: TeamCell = TeamCell {
    active: true,
    assignee: false,
    involved: true,
};
const NO_ACTIVITY: TeamCell = TeamCell {
    active: false,
    assignee: false,
    involved: false,
};
const SWITCHED_ON_WITHOUT_ACTIVITY: TeamCell = TeamCell {
    active: true,
    assignee: false,
    involved: false,
};

fn issue_sql(
    id: u32,
    project: &str,
    category: &str,
    assignee: &str,
    reporter: &str,
    gone: u8,
) -> String {
    let quote = |v: &str| {
        if v.is_empty() {
            "NULL".to_string()
        } else {
            format!("'{v}'")
        }
    };
    format!(
        "INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, status_category,
                                 assignee_id, reporter_id, created_at, updated_at, synced_at, gone)
         VALUES ('{id}', 'K-{id}', '{project}', 'Issue {id}', 'Task', 's1', '{category}',
                 {}, {}, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', {gone});",
        quote(assignee),
        quote(reporter),
    )
}

async fn exec(db: &Db, sql: &'static str) {
    db.write(move |tx, _| {
        tx.execute_batch(sql)?;
        Ok(())
    })
    .await
    .unwrap();
}

/// Website: Bea has two open issues and a gone one, Alex a finished one, Hal was assigned one in a recorded change,
/// Ivy in an issue's history, Cleo reports, Finn comments, Gus changed something, Eli only appears on the gone issue.
/// Dan is only named in a status change. Support: Dan assigned, Bea reporting.
async fn lab() -> (tempfile::TempDir, Db) {
    let dir = crate::test_support::temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    let mut sql = String::from(
        "INSERT INTO cache.space (id, key, name, raw) VALUES
           ('10001', 'NW', 'Website relaunch', '{}'), ('10002', 'NWS', 'Northwind support', '{}');
         INSERT INTO cache.user (account_id, name) VALUES
           ('acc-alex', 'alex Stone'), ('acc-bea', 'Bea Carter'), ('acc-cleo', 'Cleo Dunn'),
           ('acc-dan', 'Dan Evers'), ('acc-eli', 'Eli Ford'), ('acc-finn', 'Finn Gray'),
           ('acc-gus', 'Gus Hart'), ('acc-hal', 'Hal Imre'), ('acc-ivy', 'ivy Quill');",
    );
    for issue in [
        issue_sql(1, WEB, "indeterminate", BEA, CLEO, 0),
        issue_sql(2, WEB, "done", ALEX, CLEO, 0),
        issue_sql(3, WEB, "indeterminate", BEA, ELI, 1),
        issue_sql(4, SUPPORT, "new", DAN, BEA, 0),
        issue_sql(5, WEB, "new", BEA, "", 0),
        issue_sql(6, WEB, "new", "", "", 0),
        issue_sql(7, WEB, "new", "", "", 0),
    ] {
        sql.push_str(&issue);
    }
    sql.push_str(
        "INSERT INTO cache.comment (id, issue_id, author_id, created_at, updated_at)
           VALUES ('c1', '1', 'acc-finn', '2026-01-02T00:00:00Z', '2026-01-02T00:00:00Z');
         INSERT INTO cache.issue_history (issue_id, position, field, field_id, to_value, author_id, changed_at)
           VALUES ('2', 0, 'status', 'status', NULL, 'acc-gus', '2026-01-02T00:00:00Z'),
                  ('7', 0, 'assignee', 'assignee', 'acc-ivy', 'acc-gus', '2026-01-02T00:00:00Z');
         INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
           VALUES ('6', 'assignee', NULL, 'acc-hal', '2026-01-02T00:00:00Z'),
                  ('1', 'status', NULL, 'acc-dan', '2026-01-02T00:00:00Z');",
    );
    db.write(move |tx, _| {
        tx.execute_batch(&sql)?;
        Ok(())
    })
    .await
    .unwrap();
    (dir, db)
}

async fn track(db: &Db, projects: &[&str]) {
    for project in projects {
        crate::connection::track_project(db, project.to_string(), true)
            .await
            .unwrap();
    }
}

async fn matrix(db: &Db) -> TeamMatrix {
    team_matrix_of(db, None).await.unwrap()
}

fn ids(matrix: &TeamMatrix) -> Vec<&str> {
    matrix
        .people
        .iter()
        .map(|row| row.person.account_id.as_str())
        .collect()
}

fn cell(matrix: &TeamMatrix, account_id: &str, project: &str) -> TeamCell {
    let column = matrix.spaces.iter().position(|p| p == project).unwrap();
    matrix
        .people
        .iter()
        .find(|row| row.person.account_id == account_id)
        .unwrap_or_else(|| panic!("{account_id} is not listed"))
        .cells[column]
}

async fn set(db: &Db, project: &str, account: &str, on: bool) -> AppResult<TeamMatrix> {
    space_set_member(db, project.into(), account.into(), on, None).await
}

const ZOE: &str = "acc-zoe";

fn me_of(account_id: &str) -> Me {
    Me {
        account_id: account_id.to_string(),
        name: "Zoe Park".to_string(),
    }
}

async fn active_people(db: &Db, projects: &[&str], me: Option<&str>) -> Vec<String> {
    let projects: Vec<String> = projects.iter().map(|p| p.to_string()).collect();
    let me = me.map(me_of);
    db.read(move |conn| active(conn, &projects, me.as_ref()))
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.account_id)
        .collect()
}

#[tokio::test]
async fn people_are_listed_once_sorted_by_name_ignoring_case() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;

    assert_eq!(
        ids(&matrix(&db).await),
        [ALEX, BEA, CLEO, FINN, GUS, HAL, IVY]
    );
}

#[tokio::test]
async fn only_activity_on_non_gone_issues_of_tracked_spaces_lists_a_person() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;
    let web = matrix(&db).await;

    assert!(!ids(&web).contains(&DAN));
    assert!(!ids(&web).contains(&ELI));

    track(&db, &[SUPPORT]).await;
    let both = matrix(&db).await;
    assert!(ids(&both).contains(&DAN));
    assert!(!ids(&both).contains(&ELI));
}

#[tokio::test]
async fn anyone_ever_assigned_a_non_gone_issue_is_on_the_team_by_default() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;
    let web = matrix(&db).await;

    for (account, why) in [
        (BEA, "current assignee"),
        (ALEX, "current assignee of a finished issue"),
        (HAL, "assignee in a recorded change"),
        (IVY, "assignee in an issue's history"),
    ] {
        assert_eq!(cell(&web, account, WEB), ASSIGNEE_ON, "{account} {why}");
    }
}

#[tokio::test]
async fn reporters_commenters_and_history_authors_are_listed_but_off_by_default() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;
    let web = matrix(&db).await;

    for (account, role) in [
        (CLEO, "reporter"),
        (FINN, "commenter"),
        (GUS, "history author"),
    ] {
        assert_eq!(cell(&web, account, WEB), INVOLVED_OFF, "{account} {role}");
    }
}

#[tokio::test]
async fn a_person_is_on_in_the_space_they_are_assigned_in_and_off_in_the_one_they_only_report_in() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB, SUPPORT]).await;
    let both = matrix(&db).await;

    assert_eq!(cell(&both, BEA, WEB), ASSIGNEE_ON);
    assert_eq!(cell(&both, BEA, SUPPORT), INVOLVED_OFF);
    assert_eq!(cell(&both, DAN, SUPPORT), ASSIGNEE_ON);
    assert_eq!(cell(&both, DAN, WEB), NO_ACTIVITY);
}

#[tokio::test]
async fn columns_are_the_tracked_spaces_in_sidebar_order() {
    let (_dir, db) = lab().await;
    track(&db, &[SUPPORT, WEB]).await;

    let both = matrix(&db).await;

    assert_eq!(both.spaces, [SUPPORT, WEB]);
    assert!(both.people.iter().all(|row| row.cells.len() == 2));
}

#[tokio::test]
async fn an_untracked_space_is_not_a_column_and_its_choices_are_not_shown() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB, SUPPORT]).await;
    set(&db, SUPPORT, CLEO, true).await.unwrap();

    crate::connection::track_project(&db, SUPPORT.into(), false)
        .await
        .unwrap();
    let web = matrix(&db).await;

    assert_eq!(web.spaces, [WEB]);
    assert!(web.people.iter().all(|row| row.cells.len() == 1));
    assert_eq!(cell(&web, CLEO, WEB), INVOLVED_OFF);
    assert!(!ids(&web).contains(&DAN));
}

#[tokio::test]
async fn nothing_tracked_gives_an_empty_grid() {
    let (_dir, db) = lab().await;

    let empty = matrix(&db).await;

    assert!(empty.spaces.is_empty());
    assert!(empty.people.is_empty());
}

#[tokio::test]
async fn the_person_marked_as_me_gets_the_first_color() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;

    let grid = team_matrix_of(&db, Some(CLEO.into())).await.unwrap();

    for row in &grid.people {
        let is_me = row.person.account_id == CLEO;
        assert_eq!(row.person.color == 1, is_me, "{}", row.person.account_id);
    }
}

#[tokio::test]
async fn switching_someone_on_makes_them_active_in_that_space_only() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB, SUPPORT]).await;
    set(&db, SUPPORT, CLEO, true).await.unwrap();

    let grid = matrix(&db).await;

    assert_eq!(cell(&grid, CLEO, SUPPORT), SWITCHED_ON_WITHOUT_ACTIVITY);
    assert_eq!(cell(&grid, CLEO, WEB), INVOLVED_OFF);
}

#[tokio::test]
async fn switching_an_involved_person_on_keeps_their_activity_flags() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;

    let grid = set(&db, WEB, CLEO, true).await.unwrap();

    assert_eq!(cell(&grid, CLEO, WEB), INVOLVED_ON);
}

#[tokio::test]
async fn switching_an_assignee_off_affects_only_that_space() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB, SUPPORT]).await;
    set(&db, SUPPORT, BEA, true).await.unwrap();

    let grid = set(&db, WEB, BEA, false).await.unwrap();

    assert_eq!(cell(&grid, BEA, WEB), ASSIGNEE_OFF);
    assert_eq!(
        cell(&grid, BEA, SUPPORT),
        TeamCell {
            active: true,
            assignee: false,
            involved: true
        }
    );
    assert_eq!(cell(&grid, ALEX, WEB), ASSIGNEE_ON);
}

#[tokio::test]
async fn switching_off_twice_keeps_the_person_off() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;
    set(&db, WEB, BEA, false).await.unwrap();

    let grid = set(&db, WEB, BEA, false).await.unwrap();

    assert_eq!(cell(&grid, BEA, WEB), ASSIGNEE_OFF);
}

#[tokio::test]
async fn switching_back_to_the_default_restores_it() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;
    let before = matrix(&db).await;

    set(&db, WEB, BEA, false).await.unwrap();
    set(&db, WEB, CLEO, true).await.unwrap();
    set(&db, WEB, BEA, true).await.unwrap();
    let after = set(&db, WEB, CLEO, false).await.unwrap();

    assert_eq!(after, before);
}

#[tokio::test]
async fn an_assignee_switched_off_and_back_on_stops_being_on_once_unassigned() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;
    set(&db, WEB, ALEX, false).await.unwrap();
    set(&db, WEB, ALEX, true).await.unwrap();

    exec(
        &db,
        "UPDATE cache.issue SET assignee_id = NULL WHERE id = '2'",
    )
    .await;

    assert!(
        !active_people(&db, &[WEB], None)
            .await
            .contains(&ALEX.into())
    );
}

#[tokio::test]
async fn a_person_switched_on_and_back_off_follows_a_later_assignment() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;
    set(&db, WEB, CLEO, true).await.unwrap();
    set(&db, WEB, CLEO, false).await.unwrap();

    exec(
        &db,
        "UPDATE cache.issue SET assignee_id = 'acc-cleo' WHERE id = '6'",
    )
    .await;

    assert_eq!(cell(&matrix(&db).await, CLEO, WEB), ASSIGNEE_ON);
}

#[tokio::test]
async fn switching_on_someone_without_activity_adds_them_to_that_column() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB, SUPPORT]).await;
    assert_eq!(cell(&matrix(&db).await, DAN, WEB), NO_ACTIVITY);

    set(&db, WEB, ELI, true).await.unwrap();
    let grid = set(&db, WEB, DAN, true).await.unwrap();

    assert_eq!(cell(&grid, DAN, WEB), SWITCHED_ON_WITHOUT_ACTIVITY);
    assert_eq!(cell(&grid, DAN, SUPPORT), ASSIGNEE_ON);
    assert_eq!(cell(&grid, ELI, WEB), SWITCHED_ON_WITHOUT_ACTIVITY);
    assert_eq!(cell(&grid, ELI, SUPPORT), NO_ACTIVITY);
}

#[tokio::test]
async fn someone_without_activity_who_is_switched_on_only_appears_while_the_choice_stands() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;

    assert!(!ids(&matrix(&db).await).contains(&ELI));
    assert!(ids(&set(&db, WEB, ELI, true).await.unwrap()).contains(&ELI));
    assert!(!ids(&set(&db, WEB, ELI, false).await.unwrap()).contains(&ELI));
}

#[tokio::test]
async fn setting_a_member_returns_the_same_grid_as_reading_it() {
    let (_dir, db) = lab().await;
    track(&db, &[SUPPORT, WEB]).await;

    let returned = set(&db, WEB, CLEO, true).await.unwrap();

    assert_eq!(returned, matrix(&db).await);
}

#[tokio::test]
async fn an_untracked_space_or_an_unknown_account_is_not_found() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB]).await;

    for (project, account) in [(SUPPORT, DAN), ("99999", BEA), (WEB, "acc-nobody")] {
        let err = set(&db, project, account, true).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NotFound, "{project} {account}");
    }
}

#[tokio::test]
async fn active_lists_the_default_and_chosen_team_of_the_projects_sorted_by_name() {
    let (_dir, db) = lab().await;
    assert_eq!(
        active_people(&db, &[WEB, SUPPORT], None).await,
        [ALEX, BEA, DAN, HAL, IVY]
    );
    assert_eq!(active_people(&db, &[SUPPORT], None).await, [DAN]);

    track(&db, &[WEB]).await;
    set(&db, WEB, CLEO, true).await.unwrap();
    assert_eq!(
        active_people(&db, &[WEB], None).await,
        [ALEX, BEA, CLEO, HAL, IVY]
    );
}

#[tokio::test]
async fn active_puts_me_first_and_once_when_among_the_people() {
    let (_dir, db) = lab().await;
    assert_eq!(
        active_people(&db, &[WEB, SUPPORT], Some(IVY)).await,
        [IVY, ALEX, BEA, DAN, HAL]
    );
}

#[tokio::test]
async fn active_lists_me_first_even_when_not_on_the_team() {
    let (_dir, db) = lab().await;
    assert_eq!(
        active_people(&db, &[WEB, SUPPORT], Some(GUS)).await,
        [GUS, ALEX, BEA, DAN, HAL, IVY]
    );
    assert_eq!(
        active_people(&db, &[SUPPORT], Some(CLEO)).await,
        [CLEO, DAN]
    );
}

#[tokio::test]
async fn active_names_me_from_the_cache_or_else_from_the_connection() {
    let (_dir, db) = lab().await;
    let names = |me: Me| db.read(move |conn| active(conn, &[WEB.to_string()], Some(&me)));

    let cached = names(me_of(GUS)).await.unwrap();
    let unsynced = names(me_of(ZOE)).await.unwrap();

    assert_eq!(
        (cached[0].account_id.as_str(), cached[0].name.as_str()),
        (GUS, "Gus Hart")
    );
    assert_eq!(
        (unsynced[0].account_id.as_str(), unsynced[0].name.as_str()),
        (ZOE, "Zoe Park")
    );
}

#[tokio::test]
async fn active_without_me_lists_only_the_teammates() {
    let (_dir, db) = lab().await;
    assert_eq!(
        active_people(&db, &[WEB], None).await,
        [ALEX, BEA, HAL, IVY]
    );
}

#[test]
fn me_is_known_only_with_an_account_id() {
    let mut connection = JiraConnection::default();
    assert_eq!(Me::of(&connection), None);

    connection.account_id = Some(ALEX.to_string());
    assert_eq!(
        Me::of(&connection),
        Some(Me {
            account_id: ALEX.to_string(),
            name: String::new()
        })
    );

    connection.display_name = Some("Alex Morgan".to_string());
    assert_eq!(
        Me::of(&connection),
        Some(Me {
            account_id: ALEX.to_string(),
            name: "Alex Morgan".to_string()
        })
    );
}

#[tokio::test]
async fn a_person_on_in_one_project_and_off_in_another_still_counts() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB, SUPPORT]).await;
    set(&db, WEB, DAN, true).await.unwrap();
    set(&db, SUPPORT, DAN, false).await.unwrap();

    assert!(
        active_people(&db, &[WEB, SUPPORT], None)
            .await
            .contains(&DAN.to_string())
    );
    assert!(
        !active_people(&db, &[SUPPORT], None)
            .await
            .contains(&DAN.to_string())
    );
}

#[tokio::test]
async fn a_person_off_in_every_given_project_is_left_out() {
    let (_dir, db) = lab().await;
    track(&db, &[WEB, SUPPORT]).await;
    set(&db, WEB, BEA, false).await.unwrap();

    assert_eq!(
        active_people(&db, &[WEB, SUPPORT], None).await,
        [ALEX, DAN, HAL, IVY]
    );
}

async fn named(db: &Db, accounts: &[&str], me: Option<&str>) -> Vec<(String, String)> {
    let accounts: HashSet<String> = accounts.iter().map(|a| a.to_string()).collect();
    let me = me.map(me_of);
    db.read(move |conn| with_me_first(conn, &accounts, me.as_ref()))
        .await
        .unwrap()
        .into_iter()
        .map(|p| (p.account_id, p.name))
        .collect()
}

fn pair(account: &str, name: &str) -> (String, String) {
    (account.to_string(), name.to_string())
}

#[tokio::test]
async fn accounts_are_listed_by_name_ignoring_case() {
    let (_dir, db) = lab().await;

    assert_eq!(
        named(&db, &[IVY, BEA, ALEX, DAN], None).await,
        [
            pair(ALEX, "alex Stone"),
            pair(BEA, "Bea Carter"),
            pair(DAN, "Dan Evers"),
            pair(IVY, "ivy Quill"),
        ]
    );
}

#[tokio::test]
async fn an_account_the_cache_does_not_know_is_left_out() {
    let (_dir, db) = lab().await;

    assert_eq!(
        named(&db, &[BEA, "acc-nobody"], None).await,
        [pair(BEA, "Bea Carter")]
    );
}

#[tokio::test]
async fn me_comes_first_and_only_once_when_among_the_accounts() {
    let (_dir, db) = lab().await;

    assert_eq!(
        named(&db, &[ALEX, BEA, IVY], Some(BEA)).await,
        [
            pair(BEA, "Bea Carter"),
            pair(ALEX, "alex Stone"),
            pair(IVY, "ivy Quill"),
        ]
    );
}

#[tokio::test]
async fn me_comes_first_even_with_no_accounts() {
    let (_dir, db) = lab().await;

    assert_eq!(
        named(&db, &[], Some(ALEX)).await,
        [pair(ALEX, "alex Stone")]
    );
    assert_eq!(
        named(&db, &[DAN], Some(ALEX)).await,
        [pair(ALEX, "alex Stone"), pair(DAN, "Dan Evers")]
    );
}

#[tokio::test]
async fn me_unknown_to_the_cache_is_listed_under_the_connections_name() {
    let (_dir, db) = lab().await;

    assert_eq!(
        named(&db, &[DAN], Some(ZOE)).await,
        [pair(ZOE, "Zoe Park"), pair(DAN, "Dan Evers")]
    );
}

#[tokio::test]
async fn the_matrix_serializes_with_camel_case_fields() {
    let (_dir, db) = lab().await;
    track(&db, &[SUPPORT]).await;

    let json = serde_json::to_value(matrix(&db).await).unwrap();

    assert_eq!(json["spaces"], json!([SUPPORT]));
    assert_eq!(json["people"][0]["person"]["accountId"], BEA);
    assert_eq!(
        json["people"][0]["cells"],
        json!([{ "active": false, "assignee": false, "involved": true }])
    );
    assert_eq!(json["people"][1]["person"]["accountId"], DAN);
    assert_eq!(
        json["people"][1]["cells"],
        json!([{ "active": true, "assignee": true, "involved": true }])
    );
}
