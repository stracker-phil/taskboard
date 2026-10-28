//! Jira Cloud REST API v3: the site, the client, its limits and the answers it reads (`docs/jira.md`).
//! Every request goes through `gateway` (ADR 0025).

mod api;
mod client;
pub mod dto;
pub mod gateway;
pub mod jql;
pub mod limiter;
pub mod paging;
pub mod retry;
pub mod site;

pub use client::JiraClient;
pub use gateway::BULK_FETCH_MAX;
pub use limiter::Limiter;
pub use paging::{SearchPages, SearchRequest};
pub use site::{Site, TokenKind};
