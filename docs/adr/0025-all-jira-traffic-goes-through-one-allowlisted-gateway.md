# 0025. All Jira traffic goes through one allowlisted gateway

## Status

Accepted (2026-10-28). Builds on [0013](0013-network-only-to-listed-hosts.md) and
[0020](0020-jira-site-and-atlassian-gateway-are-the-hosts.md): they fix the hosts, this one fixes the requests.

## Context

Taskboard reads Jira, but the owner's API token may carry more rights than the app needs: a classic token acts
with the account's full rights, and a scoped one may include `write:jira-work`. The host list kept requests on
Jira, but any module could send any method to any path, values were spliced into paths with `format!`, and
redirects were followed. The owner asked for a single module, an allowlist of the actions Taskboard may take,
and tests wide enough that nothing thrown at it has unexpected side effects.

## Decision

- `jira::gateway` is the only code that sends a request. `Gateway::send` takes an `Endpoint`, a closed enum with
  one variant per allowed operation; there is no method that takes a path, a method or a body.
- Values that name a Jira object or carry text are validated newtypes (`ProjectKey`, `IssueRef`, `Jql`,
  `FieldIds`, …). Path segments are pushed one by one (percent-encoded); JQL travels only in JSON bodies.
- `guard::check` compares each planned request with a fixed table of methods, paths and query keys right
  before the wire, apart from the code that planned it. Write rows are refused while `WRITES_ENABLED` is
  `false`; milestone 4 adds each write as its own variant and row.
- The HTTP client (`gateway/http.rs`) follows no redirects, accepts only JSON on success and caps answers at
  32 MB. A clippy fence (`src-tauri/clippy.toml`) refuses `reqwest` types outside that file.
- Rejected: checking only hosts (a token's extra rights stay reachable); a path allowlist alone without typed
  endpoints (a stringly API invites new call sites); a separate proxy process (more moving parts for one
  user).

## Consequences

- What Taskboard may ask of Jira is one table (`guard.rs` `RULES`) and one enum (`endpoint.rs`), both listed in
  `docs/jira.md`. A new endpoint needs a variant, a guard row and tests.
- Unit and property tests cover planning, parameters and the guard; integration tests run the client against
  the fake Jira and check every recorded request against the guard.
