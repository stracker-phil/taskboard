//! An issue as Markdown for the clipboard: read from the cache here, composed by `compose`.

mod compose;

pub use compose::{Content, MarkdownComment, MarkdownIssue, browse_url, compose};

use rusqlite::Connection;
use serde_json::Value;

use super::detail::{CustomField, IssueDetail, read_detail};
use super::web_links::issue_web_links;
use crate::db::Db;
use crate::error::AppResult;
use crate::lists::Viewer;
use crate::views::rows::Cell;

/// The issue as Markdown, its `source` on `site_url` (none when empty).
pub async fn issue_markdown(
    db: &Db,
    viewer: Viewer,
    site_url: String,
    id: String,
) -> AppResult<String> {
    let links = issue_web_links(db, id.clone()).await?;
    let tz = viewer.tz();
    let mut issue = db.read(move |conn| read(conn, &viewer, &id)).await?;
    issue.web_links = links.into_iter().map(|l| (l.title, l.url)).collect();
    issue.source = issue.source.and_then(|key| browse_url(&site_url, &key));
    Ok(compose(&issue, tz))
}

/// The issue without web links; `source` holds its key.
fn read(conn: &Connection, viewer: &Viewer, id: &str) -> AppResult<MarkdownIssue> {
    let IssueDetail {
        key,
        summary,
        status_name,
        assignees,
        priority,
        custom: fields,
        ..
    } = read_detail(conn, viewer, id)?;
    let body = read_body(conn, id)?;
    Ok(MarkdownIssue {
        source: Some(key),
        summary,
        status: status_name,
        assignees: assignees.into_iter().map(|p| p.name).collect(),
        priority: priority.map(|p| p.name),
        description: stored_rich(body.description_adf, body.description_html),
        fields: fields
            .into_iter()
            .filter_map(|field| field_content(field, &body.custom))
            .collect(),
        web_links: Vec::new(),
        comments: comments(conn, id)?,
    })
}

struct Body {
    description_adf: Option<String>,
    description_html: Option<String>,
    /// The custom fields' raw values by field id.
    custom: Value,
}

fn read_body(conn: &Connection, id: &str) -> AppResult<Body> {
    let (description_adf, description_html, custom): (_, _, Option<String>) = conn.query_row(
        "SELECT b.description_adf, b.description_html, i.custom FROM cache.issue i
             LEFT JOIN cache.issue_body b ON b.issue_id = i.id WHERE i.id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(Body {
        description_adf,
        description_html,
        custom: custom
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default(),
    })
}

/// The stored ADF when it is a document, else Jira's HTML.
fn stored_rich(adf: Option<String>, html: Option<String>) -> Option<Content> {
    match adf
        .and_then(|a| serde_json::from_str(&a).ok())
        .and_then(adf_document)
    {
        Some(doc) => Some(Content::Adf(doc)),
        None => html.map(Content::Html),
    }
}

fn adf_document(value: Value) -> Option<Value> {
    (value["type"] == "doc" && value["content"].is_array()).then_some(value)
}

/// A field the Issue view shows with content, as rich text or its value's text.
fn field_content(field: CustomField, custom: &Value) -> Option<(String, Content)> {
    let content = match (field.html, field.value) {
        (Some(html), _) => adf_document(custom[&field.id].clone())
            .map(Content::Adf)
            .unwrap_or(Content::Html(html)),
        (None, Some(Cell::Text(text))) => Content::Plain(text),
        (None, Some(Cell::Person(person))) => Content::Plain(person.name),
        (None, None) => return None,
    };
    Some((field.name, content))
}

fn comments(conn: &Connection, id: &str) -> AppResult<Vec<MarkdownComment>> {
    let mut query = conn.prepare(
        "SELECT u.name, c.created_at, c.body_adf, c.body_html FROM cache.comment c
         LEFT JOIN cache.user u ON u.account_id = c.author_id
         WHERE c.issue_id = ?1 ORDER BY c.created_at",
    )?;
    let rows = query.query_map([id], |r| {
        Ok(MarkdownComment {
            author: r.get(0)?,
            at: r.get(1)?,
            body: stored_rich(r.get(2)?, r.get(3)?),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}
