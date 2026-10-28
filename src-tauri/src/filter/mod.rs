//! Which issues a filter list holds (ADR 0024): typed conditions, compiled to SQL and put into words.

mod describe;
mod model;
mod sql;

pub use describe::{Names, describe};
pub use model::{Condition, Field, Filter, ME, Op, Value};
pub use sql::{Compiled, Ctx, CustomField, OPEN_ISSUES, TRACKED_ISSUES, compile};
