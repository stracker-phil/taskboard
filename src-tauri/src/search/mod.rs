//! Full-text search over the cached issues and the places: the command palette's query and the `text` filter
//! field.

pub mod commands;
mod fts;
mod palette;
mod places;

pub use fts::{KeyHint, key_hint, match_expression};
pub use palette::{
    IssueHit, MatchedIn, PaletteResults, PaletteScope, TextPart, palette_query, palette_search,
    query,
};
pub use places::{ListHit, ViewHit};

/// Issues (alias `i`) whose key, summary, description or comments hold every word of a `match_expression`.
pub const TEXT_MATCH: &str =
    "i.id IN (SELECT CAST(rowid AS TEXT) FROM cache.issue_fts WHERE issue_fts MATCH ?)";
