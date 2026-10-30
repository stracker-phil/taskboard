//! Full-text search over the cached issues: the command palette's query and the `text` filter field.

pub mod commands;
mod fts;
mod palette;

pub use fts::{KeyHint, key_hint, match_expression};
pub use palette::{
    IssueHit, ListHit, MatchedIn, PaletteResults, PaletteScope, TextPart, palette_query,
    palette_search, query,
};

/// Issues (alias `i`) whose key, summary, description or comments hold every word of a `match_expression`.
pub const TEXT_MATCH: &str =
    "i.id IN (SELECT CAST(rowid AS TEXT) FROM cache.issue_fts WHERE issue_fts MATCH ?)";
