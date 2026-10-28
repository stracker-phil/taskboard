//! Each operator as SQL, per way a field is read (`Shape`).

use chrono::{Days, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::types::Value as SqlValue;

use super::{Compiled, Ctx, ScalarKind, Sql};
use crate::error::{AppError, AppResult};
use crate::filter::model::{Condition, ME, Op, Value};
use crate::sync::map::format_utc;

const SPRINT_STATES: [&str; 3] = ["active", "future", "closed"];

pub(super) fn scalar(
    c: &Condition,
    expr: &Sql,
    kind: ScalarKind,
    ctx: &Ctx,
) -> AppResult<Compiled> {
    let param = |value: &str| -> AppResult<SqlValue> {
        match kind {
            // Without an account `me` is nobody: no account id is empty, so `is` matches nothing and `is not`
            // everything.
            ScalarKind::User if value == ME => {
                Ok(SqlValue::Text(ctx.me.clone().unwrap_or_default()))
            }
            ScalarKind::Number => value
                .trim()
                .parse::<f64>()
                .map(SqlValue::Real)
                .map_err(|_| AppError::InvalidInput(format!("“{value}” isn't a number"))),
            _ => Ok(SqlValue::Text(value.to_string())),
        }
    };
    Ok(match c.op {
        Op::Is => expr.fill("{} = ?", vec![param(one(c)?)?]),
        Op::IsNot => expr.fill("{} IS NOT ?", vec![param(one(c)?)?]),
        Op::In | Op::NotIn => {
            let values = many(c)?
                .iter()
                .map(|v| param(v))
                .collect::<AppResult<Vec<_>>>()?;
            if values.is_empty() {
                return Ok(constant(c.op == Op::NotIn));
            }
            let marks = marks(values.len());
            let template = if c.op == Op::In {
                format!("{{}} IN ({marks})")
            } else {
                format!("{{}} IS NULL OR {{}} NOT IN ({marks})")
            };
            expr.fill(&template, values)
        }
        Op::IsEmpty => expr.fill("COALESCE({}, '') = ''", vec![]),
        Op::IsNotEmpty => expr.fill("COALESCE({}, '') <> ''", vec![]),
        Op::Contains if kind == ScalarKind::Text => {
            expr.fill("{} LIKE ? ESCAPE '\\'", vec![like(one(c)?)])
        }
        _ => return Err(not_applicable(c)),
    })
}

pub(super) fn list(c: &Condition, array: &Sql, text: bool) -> AppResult<Compiled> {
    const ELEMENT: &str = "CASE WHEN j.type = 'object' THEN COALESCE(json_extract(j.value, '$.value'), \
        json_extract(j.value, '$.name'), json_extract(j.value, '$.id')) ELSE j.value END";
    let any = |test: &str, params: Vec<SqlValue>| {
        array.fill(
            &format!("EXISTS (SELECT 1 FROM json_each({{}}) j WHERE {ELEMENT} {test})"),
            params,
        )
    };
    let negate = |compiled: Compiled| Compiled {
        condition: format!("NOT {}", compiled.condition),
        params: compiled.params,
    };
    Ok(match c.op {
        Op::Is => any("= ?", vec![text_param(one(c)?)]),
        Op::IsNot => negate(any("= ?", vec![text_param(one(c)?)])),
        Op::In | Op::NotIn => {
            let values: Vec<SqlValue> = many(c)?.iter().map(|v| text_param(v)).collect();
            if values.is_empty() {
                return Ok(constant(c.op == Op::NotIn));
            }
            let matched = any(&format!("IN ({})", marks(values.len())), values);
            if c.op == Op::In {
                matched
            } else {
                negate(matched)
            }
        }
        Op::IsEmpty => array.fill("NOT EXISTS (SELECT 1 FROM json_each({}))", vec![]),
        Op::IsNotEmpty => array.fill("EXISTS (SELECT 1 FROM json_each({}))", vec![]),
        Op::Contains if text => any("LIKE ? ESCAPE '\\'", vec![like(one(c)?)]),
        _ => return Err(not_applicable(c)),
    })
}

/// The issue's sprint is the active one, else its latest (`sync::map`); a state matches that sprint's.
pub(super) fn sprint(c: &Condition) -> AppResult<Compiled> {
    Ok(match c.op {
        Op::Is | Op::IsNot => {
            let value = one(c)?;
            let column = if is_sprint_state(value) {
                "i.sprint_state"
            } else {
                "i.sprint_id"
            };
            let template = if c.op == Op::Is {
                "{} = ?"
            } else {
                "{} IS NOT ?"
            };
            Sql::column(column).fill(template, vec![text_param(value)])
        }
        Op::In | Op::NotIn => {
            let (states, ids): (Vec<&String>, Vec<&String>) =
                many(c)?.iter().partition(|v| is_sprint_state(v));
            let mut parts = Vec::new();
            let mut params = Vec::new();
            for (name, values) in [("i.sprint_state", states), ("i.sprint_id", ids)] {
                if !values.is_empty() {
                    parts.push(format!("COALESCE({name} IN ({}), 0)", marks(values.len())));
                    params.extend(values.into_iter().map(|v| text_param(v)));
                }
            }
            if parts.is_empty() {
                return Ok(constant(c.op == Op::NotIn));
            }
            let matched = parts.join(" OR ");
            Compiled {
                condition: if c.op == Op::In {
                    matched
                } else {
                    format!("NOT ({matched})")
                },
                params,
            }
        }
        Op::IsEmpty => Sql::column("i.sprint_id").fill("{} IS NULL", vec![]),
        Op::IsNotEmpty => Sql::column("i.sprint_id").fill("{} IS NOT NULL", vec![]),
        _ => return Err(not_applicable(c)),
    })
}

fn is_sprint_state(value: &str) -> bool {
    SPRINT_STATES.contains(&value)
}

/// Days are whole days in `ctx.tz`; "within the last n days" starts n days before today.
pub(super) fn date(c: &Condition, expr: &Sql, timestamp: bool, ctx: &Ctx) -> AppResult<Compiled> {
    let bound = |day: NaiveDate| -> SqlValue {
        if timestamp {
            SqlValue::Text(format_utc(start_of(day, ctx.tz)))
        } else {
            SqlValue::Text(day.format("%Y-%m-%d").to_string())
        }
    };
    let between = |from: NaiveDate, until: NaiveDate| {
        let after = expr.fill("{} >= ?", vec![bound(from)]);
        let before = expr.fill("{} < ?", vec![bound(until)]);
        Compiled {
            condition: format!("{} AND {}", after.condition, before.condition),
            params: [after.params, before.params].concat(),
        }
    };
    Ok(match c.op {
        Op::Before => expr.fill("{} < ?", vec![bound(day(c)?)]),
        Op::After => expr.fill("{} >= ?", vec![bound(days_after(c, day(c)?, 1)?)]),
        Op::WithinLastDays => {
            let from = ctx
                .today
                .checked_sub_days(Days::new(days(c)?.into()))
                .ok_or_else(|| rejected(c, "is out of range"))?;
            between(from, days_after(c, ctx.today, 1)?)
        }
        Op::WithinNextDays => {
            let until = days_after(c, ctx.today, u64::from(days(c)?) + 1)?;
            between(ctx.today, until)
        }
        Op::IsEmpty => expr.fill("{} IS NULL", vec![]),
        Op::IsNotEmpty => expr.fill("{} IS NOT NULL", vec![]),
        _ => return Err(not_applicable(c)),
    })
}

fn start_of(day: NaiveDate, tz: Tz) -> chrono::DateTime<Utc> {
    let midnight = day.and_time(NaiveTime::MIN);
    tz.from_local_datetime(&midnight)
        .earliest()
        // Midnight can fall in a daylight saving gap; the day then starts an hour later.
        .or_else(|| {
            tz.from_local_datetime(&(midnight + chrono::Duration::hours(1)))
                .earliest()
        })
        .map(|at| at.with_timezone(&Utc))
        .unwrap_or_else(|| midnight.and_utc())
}

fn days_after(c: &Condition, day: NaiveDate, days: u64) -> AppResult<NaiveDate> {
    day.checked_add_days(Days::new(days))
        .ok_or_else(|| rejected(c, "is out of range"))
}

fn constant(matches: bool) -> Compiled {
    Compiled {
        condition: if matches { "1" } else { "0" }.into(),
        params: Vec::new(),
    }
}

fn marks(count: usize) -> String {
    vec!["?"; count].join(", ")
}

fn text_param(value: &str) -> SqlValue {
    SqlValue::Text(value.to_string())
}

fn like(value: &str) -> SqlValue {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    SqlValue::Text(format!("%{escaped}%"))
}

fn one(c: &Condition) -> AppResult<&str> {
    match &c.value {
        Some(Value::One(value)) => Ok(value),
        _ => Err(needs(c, "one value")),
    }
}

fn many(c: &Condition) -> AppResult<&[String]> {
    match &c.value {
        Some(Value::Many(values)) => Ok(values),
        Some(Value::One(value)) => Ok(std::slice::from_ref(value)),
        _ => Err(needs(c, "a list of values")),
    }
}

fn days(c: &Condition) -> AppResult<u32> {
    match c.value {
        Some(Value::Days(days)) => Ok(days),
        _ => Err(needs(c, "a number of days")),
    }
}

fn day(c: &Condition) -> AppResult<NaiveDate> {
    let value = one(c)?;
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::InvalidInput(format!("“{value}” isn't a date (YYYY-MM-DD)")))
}

/// `<field> <op> <what>`, e.g. "due before needs one value".
fn rejected(c: &Condition, what: &str) -> AppError {
    AppError::InvalidInput(format!("{} {} {what}", c.field.name(), c.op.name()))
}

fn needs(c: &Condition, what: &str) -> AppError {
    rejected(c, &format!("needs {what}"))
}

fn not_applicable(c: &Condition) -> AppError {
    rejected(c, "doesn't apply")
}
