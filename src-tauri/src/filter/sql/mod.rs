//! Compiles conditions to a parameterised SQL condition over `cache.issue i` (ADR 0024). Values only ever
//! reach SQLite as bound parameters.

use std::collections::HashMap;

use chrono::NaiveDate;
use chrono_tz::Tz;
use rusqlite::types::Value as SqlValue;

mod ops;

use super::model::{Condition, Field};
use crate::error::{AppError, AppResult};

/// Issues of tracked projects that Jira still lists.
pub const TRACKED_ISSUES: &str =
    "i.gone = 0 AND i.project_id IN (SELECT project_id FROM space WHERE tracked = 1)";
/// Issues the sidebar counts.
pub const OPEN_ISSUES: &str = "i.status_category <> 'done'";

/// What compiling needs to know beyond the conditions.
#[derive(Clone, Debug)]
pub struct Ctx {
    /// The connected account; without one `me` is nobody.
    pub me: Option<String>,
    pub today: NaiveDate,
    /// Dates in conditions are days in this zone.
    pub tz: Tz,
    /// Jira's custom fields by id, from `cache.field`.
    pub custom: HashMap<String, CustomField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomField {
    pub name: String,
    /// Jira's schema type: `user`, `option`, `string`, `number`, `date`, `datetime`, `array`, …
    pub kind: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Compiled {
    pub condition: String,
    pub params: Vec<SqlValue>,
}

impl Compiled {
    /// The condition limited to tracked, still listed issues.
    pub fn where_clause(&self) -> String {
        format!("{TRACKED_ISSUES} AND ({})", self.condition)
    }
}

/// All conditions ANDed; no condition matches every issue.
pub fn compile(conditions: &[Condition], ctx: &Ctx) -> AppResult<Compiled> {
    let mut parts = Vec::with_capacity(conditions.len());
    let mut params = Vec::new();
    for condition in conditions {
        let compiled = compile_one(condition, ctx)?;
        parts.push(format!("({})", compiled.condition));
        params.extend(compiled.params);
    }
    let condition = if parts.is_empty() {
        "1".to_string()
    } else {
        parts.join(" AND ")
    };
    Ok(Compiled { condition, params })
}

/// How a field is read from an issue row.
enum Shape {
    Scalar {
        expr: Sql,
        kind: ScalarKind,
    },
    /// A JSON array; elements are strings or objects with a `value`, `name` or `id`.
    List {
        array: Sql,
        text: bool,
    },
    Sprint,
    Date {
        expr: Sql,
        timestamp: bool,
    },
    /// Matched through the full-text index.
    FullText,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum ScalarKind {
    Id,
    User,
    Text,
    Number,
}

/// A piece of SQL with the parameters of its `?`s in order.
#[derive(Clone, Default)]
pub(super) struct Sql {
    text: String,
    params: Vec<SqlValue>,
}

impl Sql {
    pub(super) fn column(column: &str) -> Self {
        Self {
            text: column.into(),
            params: Vec::new(),
        }
    }

    fn json(id: &str, member: &str) -> Self {
        Self {
            text: "json_extract(i.raw, ?)".into(),
            params: vec![SqlValue::Text(format!("$.fields.\"{id}\"{member}"))],
        }
    }

    /// `template` with every `{}` replaced by `self`, followed by `params`.
    pub(super) fn fill(&self, template: &str, params: Vec<SqlValue>) -> Compiled {
        let uses = template.matches("{}").count();
        let mut all = Vec::with_capacity(self.params.len() * uses + params.len());
        for _ in 0..uses {
            all.extend(self.params.iter().cloned());
        }
        all.extend(params);
        Compiled {
            condition: template.replace("{}", &self.text),
            params: all,
        }
    }
}

fn shape(field: &Field, ctx: &Ctx) -> AppResult<Shape> {
    let scalar = |column: &str, kind| Shape::Scalar {
        expr: Sql::column(column),
        kind,
    };
    Ok(match field {
        Field::Status => scalar("i.status_id", ScalarKind::Id),
        Field::StatusCategory => scalar("i.status_category", ScalarKind::Id),
        Field::Type => scalar("i.issue_type", ScalarKind::Id),
        Field::Priority => scalar("i.priority", ScalarKind::Id),
        Field::Project => scalar("i.project_id", ScalarKind::Id),
        Field::Epic => scalar("i.parent_id", ScalarKind::Id),
        Field::Assignee => scalar("i.assignee_id", ScalarKind::User),
        Field::Reporter => scalar("i.reporter_id", ScalarKind::User),
        Field::Release => Shape::List {
            array: Sql::column("i.fix_versions"),
            text: false,
        },
        Field::Labels => Shape::List {
            array: Sql::column("i.labels"),
            text: true,
        },
        Field::Sprint => Shape::Sprint,
        Field::Due => Shape::Date {
            expr: Sql::column("i.due_date"),
            timestamp: false,
        },
        Field::Updated => Shape::Date {
            expr: Sql::column("i.updated_at"),
            timestamp: true,
        },
        Field::Created => Shape::Date {
            expr: Sql::column("i.created_at"),
            timestamp: true,
        },
        Field::Text => Shape::FullText,
        Field::Custom(id) => custom_shape(id, ctx)?,
    })
}

fn custom_shape(id: &str, ctx: &Ctx) -> AppResult<Shape> {
    let field = ctx
        .custom
        .get(id)
        .ok_or_else(|| AppError::InvalidInput(format!("Jira has no field “{id}”")))?;
    let scalar = |member: &str, kind| Shape::Scalar {
        expr: Sql::json(id, member),
        kind,
    };
    Ok(match field.kind.as_str() {
        "user" => scalar(".accountId", ScalarKind::User),
        "option" => scalar(".value", ScalarKind::Id),
        "string" => scalar("", ScalarKind::Text),
        "number" => scalar("", ScalarKind::Number),
        "date" => Shape::Date {
            expr: Sql::json(id, ""),
            timestamp: false,
        },
        // Jira writes the offset it was entered in; the day as written is close enough for a filter.
        "datetime" => {
            let json = Sql::json(id, "");
            Shape::Date {
                expr: Sql {
                    text: format!("substr({}, 1, 10)", json.text),
                    params: json.params,
                },
                timestamp: false,
            }
        }
        "array" => Shape::List {
            array: Sql::json(id, ""),
            text: true,
        },
        _ => {
            return Err(AppError::InvalidInput(format!(
                "Filtering on “{}” isn't supported",
                field.name
            )));
        }
    })
}

fn compile_one(condition: &Condition, ctx: &Ctx) -> AppResult<Compiled> {
    match shape(&condition.field, ctx)? {
        Shape::Scalar { expr, kind } => ops::scalar(condition, &expr, kind, ctx),
        Shape::List { array, text } => ops::list(condition, &array, text),
        Shape::Sprint => ops::sprint(condition),
        Shape::Date { expr, timestamp } => ops::date(condition, &expr, timestamp, ctx),
        Shape::FullText => ops::full_text(condition),
    }
}
