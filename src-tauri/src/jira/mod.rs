//! Jira Cloud REST API v3: the site, the client, its limits and the answers it reads (`docs/jira.md`).

mod api;
mod client;
pub mod dto;
pub mod jql;
pub mod limiter;
pub mod paging;
pub mod retry;
pub mod site;

pub use api::BULK_FETCH_MAX;
pub use client::JiraClient;
pub use limiter::Limiter;
pub use paging::{SearchPages, SearchRequest};
pub use site::{Site, TokenKind};
