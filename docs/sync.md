# Sync

`src-tauri/src/sync/` mirrors the sync scope from Jira into `cache.sqlite`
([ADR 0023](adr/0023-one-sync-task-full-sync-per-project-changes-by-diffing.md)). The UI never waits for it: it
reads the cache and listens to the events.

## The engine

`sync::SyncEngine` (`engine.rs`) is the one task that talks to Jira. `app.rs` starts it with a `SyncContext`
(clones of the settings store, `connection::JiraAccess` and `Db`) and an `Emit` callback that turns its events
into Tauri events. Jobs arrive on a channel (`queue`), run one at a time, and a job already waiting isn't
queued twice. Every start syncs right away: a full sync for a cache that was never synced, a delta otherwise.

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
   statuses, versions of every tracked project, `/myself` (account id, name, time zone). Then field usage: per
   tracked project, `used_custom_fields` samples its 100 newest issues and `field.used` marks the custom fields
   any of them fills, so this run already fetches them. A failed sample keeps the last marks and doesn't fail
   the sync.
2. Per tracked project, in Space order: Jira's approximate count of `(<scope>) AND project = KEY`, then that
   query `ORDER BY updated ASC` with `expand=renderedFields` and `SEARCH_FIELDS` plus the sprint, epic color
   and the custom fields that are `shown`, a view's column or filled by the tracked projects (`fetched_fields`). Every page is stored in one transaction and announced as `sync_progress`.
3. A `sync_state` row `full:<project id>` after each project; the `scope` row (`last_full_at`, and
   `last_delta_at` = the newest `updated` stored) after all of them. A failed page ends the job before its
   cursors move; the next run repeats it.

**Delta** (`delta.rs`): `(<scope>) AND updated >= "<cursor − 2 min>" ORDER BY updated ASC`, the date literal in
the Jira user's time zone from `/myself` (Jira reads JQL dates there, at minute precision). Pages are stored
like the full sync's (changes recorded); the cursor (`last_delta_at`) moves to the newest `updated` only after
the whole job succeeded, so a failed page is fetched again next time. Without a cursor the delta runs a full
sync. Afterwards the comments of the 20 most recently updated issues are fetched when they may be behind.
Progress counts changed issues (`fetched`).

**Reconciliation** (`reconcile.rs`): the scope's ids only (`fields=updated`); cached live issues missing from it
are asked for with `issue/bulkfetch` in batches of 100. What comes back is stored as Jira has it (a moved issue
gets its new key and project and drops out of its old Space); what doesn't is marked `gone` (an `issue_change`
row `gone`), which takes it out of every list at once. Issues gone for more than 7 days are deleted with their
comments, changes and seen rows. Runs after every full sync and hourly.

**Detail**: opening an issue fetches its comments when the cached ones are older than the issue
(`issues::issue_sync_comments`), and its web links unless fetched in the last 5 minutes (`issues::issue_sync_web_links`,
`Source::remote_links`): they replace the cached ones; a failure is logged and the cached links stay.
Its history (`issues::issue_sync_history`, `Source::changelog`, [ADR 0031](adr/0031-issue-history-comes-from-the-changelog-on-demand.md))
is fetched the same way: Jira's whole changelog, at most every 5 minutes per issue (`issues::fetched::FRESH_FOR`,
`issue.history_synced_at`), replacing the cached `issue_history` rows; `issue_details_changed` is emitted only when
they changed (web links alike), so only the issue's detail refetches. Comments emit `issues_changed` only when
they or their authors changed. History never makes an issue unread: unseen changes stay those the sync recorded in `issue_change`.
The activity shows the history with its authors and drops the recorded changes (except `created`) at or before
its newest entry, which the history already has.

## Schedule

`schedule.rs` ticks every 30 s and queues at most one job (`due`, pure): a full sync weekly, a reconciliation
hourly, a delta every `focused_minutes` while the main window has focus (5 by default) and every
`background_minutes` otherwise (15), right away after the Mac woke (a tick more than 2 minutes late), and every
minute while offline, so the cache catches up soon after the network returns. ⌘R (`sync:refresh`, also the
sync chip and `taskboard sync`) queues a delta, ⌘⇧R (`sync:full`, `taskboard sync --full`) a full sync.

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
| `issues_changed` | `IssuesChanged { ids, all }` | after a page stored changes (`Db::write`), merged to at most one per 500 ms |
| `issue_details_changed` | `IssuesChanged { ids }` | an opened issue's history or web links changed |

## Where to see it

- Settings → Sync → Cache: the state, Sync Now, and per tracked project the cached and open issues and the last
  full sync (`sync_overview`, `sync_run`).
- `taskboard status [--json]`: the same overview; `taskboard sync [--full]` queues a sync and returns;
  `taskboard reset-cache --yes` empties the cache and queues a full sync.
- The main window shows "Loading your Jira issues" with per-project progress while the first sync runs.
