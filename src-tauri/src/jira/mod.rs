//! Jira Cloud REST API v3: the site, the client, its limits and the answers it reads (`docs/jira.md`).
//! Every request goes through `gateway` (ADR 0025).

mod api;
pub mod capability;
mod client;
pub mod dto;
pub mod gateway;
pub mod in_flight;
pub mod jql;
pub mod limiter;
pub mod paging;
pub mod parallel;
pub mod retry;
pub mod site;

pub use api::is_filled;
pub use client::JiraClient;
pub use gateway::BULK_FETCH_MAX;
pub use in_flight::{InFlight, JiraActivity};
pub use limiter::Limiter;
pub use paging::{SearchPages, SearchRequest};
pub use parallel::{ask_all, run_all};
pub use site::{Site, TokenKind};
