//! Which issues a filter list holds (ADR 0024): typed conditions, compiled to SQL and put into words.

mod describe;
mod model;
mod sql;

pub use describe::{Names, Parts, describe, parts};
pub use model::{Condition, Field, Filter, ME, Op, Value};
pub use sql::{Compiled, Ctx, CustomField, OPEN_ISSUES, RELEVANT_FIELD, TRACKED_ISSUES, compile};
