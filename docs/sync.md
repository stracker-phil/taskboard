# Sync

`src-tauri/src/sync/` mirrors the sync scope from Jira into `cache.sqlite`
([ADR 0023](adr/0023-one-sync-task-full-sync-per-project-changes-by-diffing.md)). The UI never waits for it: it
reads the cache and listens to the events.

## The engine

`sync::SyncEngine` (`engine.rs`) is the one task that talks to Jira. `app.rs` starts it with a `SyncContext`
(clones of the settings store, `connection::JiraAccess` and `Db`) and an `Emit` callback that turns its events
into Tauri events. Jobs arrive on a channel (`queue`), run one at a time, and a job already waiting isn't
queued twice. A cache that was never synced gets a full sync at start.

| `SyncState` | When | What runs next |
|---|---|---|
| `idle { lastOk }` | after a successful job, or without a connection or token | the next job |
| `running { job }` | while a job runs | – |
| `error { kind, message }` | the job failed (a page, the database) | the next job |
| `offline { since }` | Jira couldn't be reached | the next job |
| `paused { message }` | Jira answered 401/403 | nothing until `connection_changed` (Save on the Connection page) |

`run_job` is the pure part: it checks the connection, runs the job and returns the state after it. Tests call it
directly.

## Jobs

**Full** (`full.rs`):

1. Metadata: `/field` (the site's Sprint and Epic Color fields are found by their custom type), projects,
   statuses, versions of every tracked project, `/myself` (account id, name, time zone).
2. Per tracked project, in Space order: Jira's approximate count of `(<scope>) AND project = KEY`, then that
   query `ORDER BY updated ASC` with `expand=renderedFields` and `SEARCH_FIELDS` plus the sprint, epic colour
   and `shown` custom fields. Every page is stored in one transaction and announced as `sync_progress`.
3. A `sync_state` row `full:<project id>` after each project; the `scope` row (`last_full_at`, and
   `last_delta_at` = the newest `updated` stored) after all of them. A failed page ends the job before its
   cursors move; the next run repeats it.

Delta, reconciliation and detail syncs arrive in step 14. Until then `taskboard sync` runs a full sync.

## Storing a page

`map.rs` turns an issue's JSON into rows (pure): the issue with every column `docs/data.md` lists, its comments
(Jira's HTML plus plain text for search), the people it names, and the search document. Timestamps are stored
as UTC with milliseconds, so they sort as text. The sprint is the active one, else the one with the highest id;
an epic is hierarchy level 1 or type "Epic".

`store.rs` writes them. An issue whose JSON equals the cached one is skipped, so a sync without changes
announces nothing. Otherwise `diff.rs` compares the tracked fields with the cached row and writes
`issue_change` rows (`status`, `assignee`, `due_date`, `priority`, `comment_added`, `created`), stamped with the
new `updated`. A project's first sync records no changes and marks every issue seen. When the search returned
all of an issue's comments, they replace the cached ones.

## Events

| Event | Payload | When |
|---|---|---|
| `sync_state` | `SyncState` | the state changed |
| `sync_progress` | `SyncProgress { job, projectId, projectKey, pagesDone, fetched, approxTotal }` | before the first page and after every page |
| `issues_changed` | `IssuesChanged { ids, all }` | after a page stored changes (`Db::write`) |

## Where to see it

- Settings → Sync → Cache: the state, Sync Now, and per tracked project the cached and open issues and the last
  full sync (`sync_overview`, `sync_run`).
- `taskboard status [--json]`: the same overview; `taskboard sync [--full]` queues a sync and returns.
- The main window shows "Loading your Jira issues" with per-project progress while the first sync runs.
