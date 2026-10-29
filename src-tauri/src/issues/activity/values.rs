//! What the activity needs to show a status or a person: the cached names and styles.

use std::collections::HashMap;

use rusqlite::Connection;

use super::{ChangeValue, status_names, user_names};
use crate::error::AppResult;
use crate::status_style::Styles;
use crate::views::rows::{self, Person};

pub struct Values {
    /// Status id to its name and category.
    statuses: HashMap<String, (String, String)>,
    styles: Styles,
    project_id: String,
    people: HashMap<String, String>,
    me: Option<String>,
}

impl Values {
    pub fn load(conn: &Connection, id: &str, me: Option<&str>) -> AppResult<Self> {
        let project_id = conn.query_row(
            "SELECT project_id FROM cache.issue WHERE id = ?1",
            [id],
            |r| r.get(0),
        )?;
        Ok(Self {
            statuses: status_names(conn)?,
            styles: Styles::load(conn)?,
            project_id,
            people: user_names(conn)?,
            me: me.map(str::to_owned),
        })
    }

    /// A status by its id, else by the name the tracker gave; the name falls back to the id.
    pub fn status(&self, id: Option<&str>, name: Option<&str>) -> Option<ChangeValue> {
        let id = id.filter(|id| !id.is_empty());
        let (name, category) = match id.and_then(|id| self.statuses.get(id)) {
            Some(known) => known.clone(),
            None => (name.or(id)?.to_owned(), String::new()),
        };
        let style = self
            .styles
            .of(&self.project_id, id.unwrap_or_default(), &category, &name);
        Some(ChangeValue::Status { name, style })
    }

    /// A person by account id: the cached name, else `name`, else the id.
    pub fn person(&self, account_id: &str, name: Option<&str>) -> Person {
        let name = self
            .people
            .get(account_id)
            .map(String::as_str)
            .or(name)
            .unwrap_or(account_id);
        rows::person(account_id, name, self.me.as_deref())
    }

    /// A person by account id; only a name (no id) shows as text.
    pub fn person_value(
        &self,
        account_id: Option<&str>,
        name: Option<&str>,
    ) -> Option<ChangeValue> {
        match account_id.filter(|id| !id.is_empty()) {
            Some(id) => Some(ChangeValue::Person(self.person(id, name))),
            None => super::text(name.map(str::to_owned)),
        }
    }
}
