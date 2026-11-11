//! Helpers shared by unit tests and integration tests through the `test-util` feature. The demo site and
//! the fake Jira also ship in debug builds, which sync from them (ADR 0022).

pub mod demo;
pub mod fake_jira;
#[cfg(any(test, feature = "test-util"))]
mod fixtures;
mod pull_requests;
mod secrets;

#[cfg(any(test, feature = "test-util"))]
pub use fixtures::*;
pub use pull_requests::FakePullRequests;
pub use secrets::FakeSecrets;
