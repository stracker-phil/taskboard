//! Storing a page of searched issues: the issue, its body, people, comments, mentions, search row and news.

use rusqlite::{Connection, OptionalExtension, params};

use super::authors::{Entry, entries_of};
use super::diff::Snapshot;
use super::map::{FieldIds, IssueRow, Mapped, map_issue};
use super::mentions::record_mentions;
use super::news::{Arrival, Pass, new_comment_authors, record_news};
use super::users::upsert_user;
use crate::db::Changes;
use crate::error::AppResult;
use crate::jira::dto::Issue;
use crate::notify::people::people_field_ids;
use crate::writes;

/// Upserts a page of issues and returns the newest `updated` on it. Unchanged issues are skipped, so they
/// announce nothing. Changes by `me` (the connected account) that the page shows aren't news.
pub fn store_page(
    conn: &Connection,
    changes: &mut Changes,
    issues: &[Issue],
    ids: &FieldIds,
    pass: Pass,
    me: Option<&str>,
    now: &str,
) -> AppResult<Option<String>> {
    let mut newest: Option<String> = None;
    let people = people_field_ids(conn)?;
    let page = PageContext {
        pass,
        me,
        people: &people,
        now,
    };
    for issue in issues {
        let Some(mapped) = map_issue(issue, ids) else {
            log::warn!(
                "Skipped issue {} without project, status, dates or a numeric id",
                issue.id
            );
            continue;
        };
        if newest.as_deref() < Some(mapped.issue.updated_at.as_str()) {
            newest = Some(mapped.issue.updated_at.clone());
        }
        let history = issue
            .changelog
            .as_ref()
            .filter(|changelog| changelog.is_complete())
            .map(|changelog| entries_of(&changelog.histories));
        if store_issue(conn, changes, &mapped, history.as_deref(), &page)? {
            changes.issue_changed(&mapped.issue.id);
        }
    }
    Ok(newest)
}

/// What every issue of a page is stored with.
struct PageContext<'a> {
    pass: Pass,
    /// The connected account.
    me: Option<&'a str>,
    /// The people fields, by id.
    people: &'a [String],
    now: &'a str,
}

struct CachedIssue {
    snapshot: Snapshot,
    raw: Option<String>,
    gone: bool,
}

fn cached_issue(conn: &Connection, id: &str) -> AppResult<Option<CachedIssue>> {
    Ok(conn
        .query_row(
            "SELECT i.status_id, i.assignee_id, i.due_date, i.priority, i.comment_count, i.custom, b.raw, i.gone,
               i.updated_at
             FROM cache.issue i LEFT JOIN cache.issue_body b ON b.issue_id = i.id WHERE i.id = ?1",
            [id],
            |r| {
                Ok(CachedIssue {
                    snapshot: Snapshot {
                        status_id: r.get(0)?,
                        assignee_id: r.get(1)?,
                        due_date: r.get(2)?,
                        priority: r.get(3)?,
                        comment_count: r.get(4)?,
                        custom: r.get(5)?,
                        updated_at: r.get(8)?,
                    },
                    raw: r.get(6)?,
                    gone: r.get(7)?,
                })
            },
        )
        .optional()?)
}

/// `false` when the cache already holds this version or a newer one.
fn store_issue(
    conn: &Connection,
    changes: &mut Changes,
    mapped: &Mapped,
    history: Option<&[Entry]>,
    page: &PageContext,
) -> AppResult<bool> {
    let &PageContext {
        pass,
        me,
        people,
        now,
    } = page;
    let cached = cached_issue(conn, &mapped.issue.id)?;
    if cached.as_ref().is_some_and(|c| {
        let same = c.snapshot.custom == mapped.issue.custom
            && c.raw.as_deref() == Some(mapped.issue.raw.as_str());
        !c.gone && (same || c.snapshot.updated_at > mapped.issue.updated_at)
    }) {
        return Ok(false);
    }
    let mut row = mapped.issue.clone();
    writes::overlay(conn, &mut row)?;
    // Before the page's comments are stored, which would make them all known.
    let new_comments = new_comment_authors(conn, mapped)?;
    upsert_issue(conn, &row, now)?;
    upsert_body(conn, &row)?;
    store_users(conn, mapped)?;
    if pass.records_news() {
        record_mentions(conn, mapped, me)?;
    }
    store_comments(conn, mapped, now)?;
    replace_search_doc(conn, mapped)?;
    record_news(
        conn,
        changes,
        Arrival {
            row: &row,
            previous: cached.as_ref().map(|c| &c.snapshot),
            new_comments: &new_comments,
            history,
            pass,
            me,
            people,
            now,
        },
    )?;
    Ok(true)
}

fn upsert_issue(conn: &Connection, row: &IssueRow, now: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.issue (id, key, project_id, parent_id, fix_versions, epic_color, summary,
           issue_type, type_is_subtask, is_epic, status_id, status_category, priority, assignee_id,
           reporter_id, sprint, sprint_id, sprint_state, labels, due_date, created_at, updated_at,
           resolved_at, comment_count, custom, synced_at, gone)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19,
           ?20, ?21, ?22, ?23, ?24, ?25, ?26, 0)
         ON CONFLICT (id) DO UPDATE SET key = excluded.key, project_id = excluded.project_id,
           parent_id = excluded.parent_id, fix_versions = excluded.fix_versions,
           epic_color = excluded.epic_color, summary = excluded.summary, issue_type = excluded.issue_type,
           type_is_subtask = excluded.type_is_subtask, is_epic = excluded.is_epic,
           status_id = excluded.status_id, status_category = excluded.status_category,
           priority = excluded.priority, assignee_id = excluded.assignee_id,
           reporter_id = excluded.reporter_id, sprint = excluded.sprint, sprint_id = excluded.sprint_id,
           sprint_state = excluded.sprint_state, labels = excluded.labels, due_date = excluded.due_date,
           created_at = excluded.created_at, updated_at = excluded.updated_at,
           resolved_at = excluded.resolved_at, comment_count = excluded.comment_count,
           custom = excluded.custom, synced_at = excluded.synced_at, gone = 0",
        params![
            row.id,
            row.key,
            row.project_id,
            row.parent_id,
            row.fix_versions,
            row.epic_color,
            row.summary,
            row.issue_type,
            row.type_is_subtask,
            row.is_epic,
            row.status_id,
            row.status_category,
            row.priority,
            row.assignee_id,
            row.reporter_id,
            row.sprint,
            row.sprint_id,
            row.sprint_state,
            row.labels,
            row.due_date,
            row.created_at,
            row.updated_at,
            row.resolved_at,
            row.comment_count,
            row.custom,
            now
        ],
    )?;
    Ok(())
}

fn upsert_body(conn: &Connection, row: &IssueRow) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.issue_body (issue_id, description_html, description_adf, raw)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (issue_id) DO UPDATE SET description_html = excluded.description_html,
           description_adf = excluded.description_adf, raw = excluded.raw",
        params![row.id, row.description_html, row.description_adf, row.raw],
    )?;
    Ok(())
}

fn store_users(conn: &Connection, mapped: &Mapped) -> AppResult<()> {
    for user in &mapped.users {
        upsert_user(conn, user)?;
    }
    Ok(())
}

/// The search's comments; when they are all of the issue's, they replace the cached ones.
fn store_comments(conn: &Connection, mapped: &Mapped, now: &str) -> AppResult<()> {
    let issue_id = &mapped.issue.id;
    if mapped.issue.comments_complete {
        conn.execute(
            "DELETE FROM cache.comment WHERE issue_id = ?1 AND id NOT LIKE 'pending:%'",
            [issue_id],
        )?;
        conn.execute(
            "UPDATE cache.issue SET comments_synced_at = ?2 WHERE id = ?1",
            params![issue_id, now],
        )?;
    }
    for comment in &mapped.comments {
        conn.execute(
            "INSERT INTO cache.comment (id, issue_id, author_id, body_html, body_text, created_at, updated_at, body_adf)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (id) DO UPDATE SET issue_id = excluded.issue_id, author_id = excluded.author_id,
               body_html = excluded.body_html, body_text = excluded.body_text,
               created_at = excluded.created_at, updated_at = excluded.updated_at, body_adf = excluded.body_adf",
            params![
                comment.id,
                comment.issue_id,
                comment.author_id,
                comment.body_html,
                comment.body_text,
                comment.created_at,
                comment.updated_at,
                comment.body_adf
            ],
        )?;
    }
    Ok(())
}

fn replace_search_doc(conn: &Connection, mapped: &Mapped) -> AppResult<()> {
    let doc = &mapped.fts;
    conn.execute("DELETE FROM cache.issue_fts WHERE rowid = ?1", [doc.rowid])?;
    conn.execute(
        "INSERT INTO cache.issue_fts (rowid, key, summary, description, comments)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            doc.rowid,
            doc.key,
            doc.summary,
            doc.description,
            doc.comments
        ],
    )?;
    Ok(())
}
