//! What compiling and describing filters needs from the settings and the cache.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rusqlite::Connection;

use crate::error::AppResult;
use crate::filter::{Ctx, CustomField, Names};
use crate::store::settings::Connection as JiraConnection;

/// Who is looking and when: `me` and the day boundaries of date conditions.
#[derive(Clone, Debug, PartialEq)]
pub struct Viewer {
    pub me: Option<String>,
    /// IANA name; UTC when unknown.
    pub time_zone: Option<String>,
    pub now: DateTime<Utc>,
}

impl Viewer {
    pub fn of(connection: &JiraConnection, now: DateTime<Utc>) -> Self {
        Self {
            me: connection.account_id.clone(),
            time_zone: connection.time_zone.clone(),
            now,
        }
    }

    /// The owner's time zone; UTC when unknown.
    pub fn tz(&self) -> Tz {
        self.time_zone
            .as_deref()
            .and_then(|name| name.parse().ok())
            .unwrap_or(Tz::UTC)
    }

    pub fn ctx(&self, conn: &Connection) -> AppResult<Ctx> {
        let tz = self.tz();
        Ok(Ctx {
            me: self.me.clone(),
            today: self.now.with_timezone(&tz).date_naive(),
            tz,
            custom: custom_fields(conn)?,
        })
    }
}

fn custom_fields(conn: &Connection) -> AppResult<HashMap<String, CustomField>> {
    let mut query = conn.prepare(
        "SELECT id, name, COALESCE(type, '') FROM cache.field WHERE id LIKE 'customfield\\_%' ESCAPE '\\'",
    )?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            CustomField {
                name: row.get(1)?,
                kind: row.get(2)?,
            },
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The display names `describe` needs.
pub fn names(conn: &Connection) -> AppResult<Names> {
    let map = |sql: &str| -> AppResult<HashMap<String, String>> {
        let mut query = conn.prepare(sql)?;
        let rows = query.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    };
    Ok(Names {
        statuses: map("SELECT id, name FROM cache.status")?,
        users: map("SELECT account_id, name FROM cache.user")?,
        versions: map("SELECT id, name FROM cache.version")?,
        epics: map("SELECT id, summary FROM cache.issue WHERE is_epic = 1")?,
        projects: map("SELECT id, key FROM cache.space")?,
        sprints: map(
            "SELECT sprint_id, MAX(sprint) FROM cache.issue WHERE sprint_id IS NOT NULL GROUP BY sprint_id",
        )?,
        fields: map("SELECT id, name FROM cache.field")?,
    })
}
