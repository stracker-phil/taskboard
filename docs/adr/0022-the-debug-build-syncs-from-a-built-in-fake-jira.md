# 0022. The debug build syncs from a built-in fake Jira

## Status

Accepted (2026-10-28).

## Context

The debug build runs on demo data ([ADR 0011](0011-debug-build-is-a-separate-app-with-demo-data.md)).
Seeding SQLite directly would skip the code that matters most: the Jira client, paging, mapping, delta sync,
reconciliation and change tracking. The tests need the same Jira with failures on demand (429, 503, 401,
offline, deleted and moved issues).

## Decision

- `test_support::demo` builds a made-up Jira site (people, projects, statuses, epics, sprints, versions,
  issues with comments) as Jira-shaped JSON, matching the mockup.
- `test_support::fake_jira` serves it on `127.0.0.1` over plain HTTP: every endpoint Taskboard uses, the JQL
  Taskboard writes, `nextPageToken` paging. Knobs set the page size, canned failures, latency, offline, and
  edit, move, delete or revoke issues. It records every request.
- The tests use it now; the debug build points its connection at it when it syncs (step 6), so every screen
  in the debug app runs through the real sync code.
- Rejected: a seeded database (a second code path that drifts from the sync); recorded HTTP fixtures (can't
  change between runs).

## Consequences

- The fake's JQL reader understands only Taskboard's own queries; any other query matches every issue.
- Release builds can't reach it: `http` to loopback is allowed only in debug builds and tests (ADR 0020).
