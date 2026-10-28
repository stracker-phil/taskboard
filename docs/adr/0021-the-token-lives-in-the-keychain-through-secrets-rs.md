# 0021. The API token lives in the Keychain through `secrets.rs`

## Status

Accepted (2026-10-28), confirmed by the owner. The spec names the `keyring` crate; Taskboard ports Daybook's
`secrets.rs` instead.

## Context

The Jira API token grants the owner's access to every project. The spec requires it to live only in the macOS
Keychain (service `Taskboard`, account = the email), never in SQLite, config files, logs or crash reports.
Daybook already keeps a token this way, with a test double.

## Decision

- `secrets.rs`: a `SecretStore` trait, `Keychain` on `security-framework` generic passwords, and `Secret`, whose
  `Debug` hides the value and whose memory is wiped on drop. `FakeSecrets` (test support) stands in for tests.
- Service = `APP_NAME` (`Taskboard`; the debug build's `Taskboard (Debug)` keeps its own token), account = the
  email.
- The token never leaves Rust: no command returns it. `JiraClient` reads it per request to build the
  `Authorization` header, which is marked sensitive and never logged; `net::redact` strips `Authorization`
  values, `Basic`/`Bearer` tokens and the email from every log line it writes.
- Rejected: the `keyring` crate (a second, cross-platform abstraction over the same API, without the fake).

## Consequences

- A test never touches the real Keychain.
- macOS may ask once to allow access after the app's signature changes; the local signing certificate
  ([ADR 0015](0015-signed-with-a-local-certificate.md)) keeps that stable.
