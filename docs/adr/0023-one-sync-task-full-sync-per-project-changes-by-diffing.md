# 0023. One sync task; the full sync runs per tracked project; changes are recorded by diffing

## Status

Accepted (2026-10-28).

## Context

The spec asks for one background task that owns all network work, a full sync of the sync scope, and a record
of what changed (status, assignee, due date, priority, new comments, new issues) to drive the change dots and
the activity. Jira's search pages strictly one after another and gives no total; the first-sync screen
shows progress per Space and should make finished Spaces usable while others still load.

## Decision

- **One task** (`sync::SyncEngine`) receives jobs on a channel, runs one at a time and coalesces duplicates. It
  owns the state (`idle`, `running`, `error`, `offline`, `paused`) and announces it as `sync_state`. A 401/403
  pauses it until the connection is saved again; offline skips the run.
- **The full sync runs per tracked project:** metadata first (fields, projects, statuses, versions, the user),
  then for each tracked project `(<scope>) AND project = KEY ORDER BY updated ASC`, with Jira's approximate
  count for the progress bar (`sync_progress` after every page). Every page is stored in one transaction.
- **Changes are recorded by diffing** the fetched issue against the cached row (`sync::diff`), never from
  Jira's changelog. A project's first sync records nothing and marks every issue seen. Unchanged issues are
  skipped, so a sync that finds nothing announces nothing.
- **Cursors move only on success:** a project's `sync_state` row after its last page, the scope's row (and the
  first `last_delta_at`, the newest `updated` stored) after every project.
- The engine runs on clones of the settings, the Keychain access and the database, so tests drive it without
  Tauri.
- Rejected: one search over the whole scope (no per-Space progress, nothing usable until the end); Jira's
  changelog (`expand=changelog`, heavy, and it misses comment counts); parallel pages (Jira forbids it).

## Consequences

- One search per project costs a few extra requests; with the 5/s limit that is negligible.
- A change made between two syncs is recorded once, stamped with Jira's `updated`, not the change's own time.
