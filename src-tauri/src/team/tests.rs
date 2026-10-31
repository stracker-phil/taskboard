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

/// Website: Bea has two open issues and a gone one, Alex a finished one, Cleo reports, Finn comments,
/// Gus changed something, Eli only appears on the gone issue. Support: Dan assigned, Bea reporting.
async fn lab() -> (tempfile::TempDir, Db) {
    let dir = crate::test_support::temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    let mut sql = String::from(
        "INSERT INTO cache.space (id, key, name, raw) VALUES
           ('10001', 'NW', 'Website relaunch', '{}'), ('10002', 'NWS', 'Northwind support', '{}');
         INSERT INTO cache.user (account_id, name) VALUES
           ('acc-alex', 'alex Stone'), ('acc-bea', 'Bea Carter'), ('acc-cleo', 'Cleo Dunn'),
           ('acc-dan', 'Dan Evers'), ('acc-eli', 'Eli Ford'), ('acc-finn', 'Finn Gray'),
           ('acc-gus', 'Gus Hart');",
    );
    for issue in [
        issue_sql(1, WEB, "indeterminate", BEA, CLEO, 0),
        issue_sql(2, WEB, "done", ALEX, CLEO, 0),
        issue_sql(3, WEB, "indeterminate", BEA, ELI, 1),
        issue_sql(4, SUPPORT, "new", DAN, BEA, 0),
        issue_sql(5, WEB, "new", BEA, "", 0),
    ] {
        sql.push_str(&issue);
    }
    sql.push_str(
        "INSERT INTO cache.comment (id, issue_id, author_id, created_at, updated_at)
           VALUES ('c1', '1', 'acc-finn', '2026-01-02T00:00:00Z', '2026-01-02T00:00:00Z');
         INSERT INTO cache.issue_history (issue_id, position, field, author_id, changed_at)
           VALUES ('2', 0, 'status', 'acc-gus', '2026-01-02T00:00:00Z');",
    );
    db.write(move |tx, _| {
        tx.execute_batch(&sql)?;
        Ok(())
    })
    .await
    .unwrap();
    (dir, db)
}

fn ids(members: &[TeamMember]) -> Vec<&str> {
    members
        .iter()
        .map(|m| m.person.account_id.as_str())
        .collect()
}

fn active_of(members: &[TeamMember], account_id: &str) -> bool {
    members
        .iter()
        .find(|m| m.person.account_id == account_id)
        .unwrap()
        .active
}

async fn active_people(db: &Db, projects: &[&str], me: Option<&str>) -> Vec<String> {
    let projects: Vec<String> = projects.iter().map(|p| p.to_string()).collect();
    let me = me.map(str::to_string);
    db.read(move |conn| active(conn, &projects, me.as_deref()))
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.account_id)
        .collect()
}

async fn set(db: &Db, project: &str, account: &str, on: bool) -> AppResult<Vec<TeamMember>> {
    space_set_member(db, project.into(), account.into(), on, None).await
}

#[tokio::test]
async fn the_team_lists_everyone_with_activity_once_sorted_by_name_ignoring_case() {
    let (_dir, db) = lab().await;
    let members = space_team(&db, WEB.into(), None).await.unwrap();
    assert_eq!(ids(&members), [ALEX, BEA, CLEO, FINN, GUS]);
}

#[tokio::test]
async fn people_of_another_space_or_only_on_gone_issues_are_not_in_the_team() {
    let (_dir, db) = lab().await;
    let web = space_team(&db, WEB.into(), None).await.unwrap();
    assert!(!ids(&web).contains(&DAN));
    assert!(!ids(&web).contains(&ELI));
    let support = space_team(&db, SUPPORT.into(), None).await.unwrap();
    assert_eq!(ids(&support), [BEA, DAN]);
}

#[tokio::test]
async fn assigned_counts_only_open_issues_of_the_space_that_are_not_gone() {
    let (_dir, db) = lab().await;
    let members = space_team(&db, WEB.into(), None).await.unwrap();
    let counts: Vec<(&str, u32)> = members
        .iter()
        .map(|m| (m.person.account_id.as_str(), m.assigned))
        .collect();
    assert_eq!(
        counts,
        [(ALEX, 0), (BEA, 2), (CLEO, 0), (FINN, 0), (GUS, 0)]
    );
    let support = space_team(&db, SUPPORT.into(), None).await.unwrap();
    assert_eq!(
        support
            .iter()
            .find(|m| m.person.account_id == BEA)
            .unwrap()
            .assigned,
        0
    );
}

#[tokio::test]
async fn the_person_marked_as_me_gets_the_first_color() {
    let (_dir, db) = lab().await;
    let members = space_team(&db, WEB.into(), Some(CLEO.into()))
        .await
        .unwrap();
    let cleo = members
        .iter()
        .find(|m| m.person.account_id == CLEO)
        .unwrap();
    assert_eq!(cleo.person.color, 1);
    assert!(
        members
            .iter()
            .filter(|m| m.person.account_id != CLEO)
            .all(|m| m.person.color != 1)
    );
}

#[tokio::test]
async fn everyone_starts_active() {
    let (_dir, db) = lab().await;
    let members = space_team(&db, WEB.into(), None).await.unwrap();
    assert!(members.iter().all(|m| m.active));
}

#[tokio::test]
async fn switching_a_person_off_affects_only_that_space() {
    let (_dir, db) = lab().await;
    let web = set(&db, WEB, BEA, false).await.unwrap();
    assert!(!active_of(&web, BEA));
    assert!(
        web.iter()
            .filter(|m| m.person.account_id != BEA)
            .all(|m| m.active)
    );
    let support = space_team(&db, SUPPORT.into(), None).await.unwrap();
    assert!(active_of(&support, BEA));
}

#[tokio::test]
async fn switching_a_person_back_on_makes_them_active_again() {
    let (_dir, db) = lab().await;
    set(&db, WEB, BEA, false).await.unwrap();
    let members = set(&db, WEB, BEA, true).await.unwrap();
    assert!(active_of(&members, BEA));
    assert!(
        space_team(&db, WEB.into(), None)
            .await
            .unwrap()
            .iter()
            .all(|m| m.active)
    );
}

#[tokio::test]
async fn switching_off_twice_keeps_the_person_inactive() {
    let (_dir, db) = lab().await;
    set(&db, WEB, BEA, false).await.unwrap();
    let members = set(&db, WEB, BEA, false).await.unwrap();
    assert!(!active_of(&members, BEA));
}

#[tokio::test]
async fn an_account_without_activity_in_the_space_is_not_found() {
    let (_dir, db) = lab().await;
    for (project, account) in [(WEB, DAN), (WEB, ELI), (WEB, "acc-nobody")] {
        let err = set(&db, project, account, false).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NotFound, "{project} {account}");
    }
}

#[tokio::test]
async fn active_lists_people_of_the_projects_sorted_by_name() {
    let (_dir, db) = lab().await;
    assert_eq!(
        active_people(&db, &[WEB, SUPPORT], None).await,
        [ALEX, BEA, CLEO, DAN, FINN, GUS]
    );
    assert_eq!(active_people(&db, &[SUPPORT], None).await, [BEA, DAN]);
}

#[tokio::test]
async fn active_puts_me_first_when_among_the_people() {
    let (_dir, db) = lab().await;
    assert_eq!(
        active_people(&db, &[WEB, SUPPORT], Some(GUS)).await,
        [GUS, ALEX, BEA, CLEO, DAN, FINN]
    );
    assert_eq!(active_people(&db, &[SUPPORT], Some(GUS)).await, [BEA, DAN]);
}

#[tokio::test]
async fn a_person_off_in_one_project_but_active_in_another_still_counts() {
    let (_dir, db) = lab().await;
    set(&db, WEB, BEA, false).await.unwrap();
    assert!(
        active_people(&db, &[WEB, SUPPORT], None)
            .await
            .contains(&BEA.to_string())
    );
}

#[tokio::test]
async fn a_person_off_in_every_given_project_is_left_out() {
    let (_dir, db) = lab().await;
    set(&db, WEB, BEA, false).await.unwrap();
    set(&db, SUPPORT, BEA, false).await.unwrap();
    assert_eq!(
        active_people(&db, &[WEB, SUPPORT], None).await,
        [ALEX, CLEO, DAN, FINN, GUS]
    );
    assert_eq!(
        active_people(&db, &[WEB], None).await,
        [ALEX, CLEO, FINN, GUS]
    );
}

#[tokio::test]
async fn the_team_serializes_with_camel_case_fields() {
    let (_dir, db) = lab().await;
    let members = space_team(&db, SUPPORT.into(), None).await.unwrap();
    let json = serde_json::to_value(&members[1]).unwrap();
    assert_eq!(json["active"], true);
    assert_eq!(json["assigned"], 1);
    assert_eq!(json["person"]["accountId"], DAN);
}
