# 0029. Issues come from a source; Jira is the first

## Status

Accepted (2026-10-29). Owner's request: prepare for GitHub issues without building them. An exception to
[ADR 0014](0014-small-feature-modules.md)'s "ports only where a swap is real".

## Context

Taskboard mirrors Jira only. The owner wants to add GitHub issues later and asked to prepare the data and the
code now, so that a second source is a new module rather than a rewrite.

## Decision

- **The cache knows each row's source.** `cache.issue`, `cache.space` and `cache.comment` have a
  `source` column (`jira` today, e.g. `github` later; migration `cache/0003`). Jira's rows take the column's
  default; another source writes its own `SourceKind`. The cache's project table is
  renamed `space`, the app's word for a tracked project or repository. Ids stay text and unique across
  sources: Jira keeps its numeric ids, any other source prefixes its own (`github:…`).
- **The sync talks to a `Source`, not to Jira.** `sync/source.rs` defines the trait with what the sync asks
  of a tracker: its field list, statuses, spaces, a space's versions, who the user is, counting and paging
  through a query, fetching issues by id, a space's comments and which custom fields it fills.
  `sync/jira_source.rs` is the only implementation and the only place in `sync/` that names the Jira client.
  The engine gets its source from `SyncContext::source`.
- **The wire shapes stay Jira's for now.** The trait passes the Jira DTOs (`jira::dto`), which `sync/map.rs`
  turns into rows. A second source maps its answers into these shapes, or, when that gets awkward, the map
  step moves behind the trait.
- Connection setup (Test Connection, the project list, checking a JQL query) stays Jira's own: a source
  brings its own settings page.

## Consequences

- Adding GitHub is a `GithubSource` plus its settings, a `source = 'github'` on its rows and a choice in
  `SyncContext::source`; queries that must tell sources apart filter on `source`.
- One trait with a single implementation until then: kept narrow, and tested through the Jira source and the
  fake Jira.
