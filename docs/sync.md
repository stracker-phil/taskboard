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
| `error { kind, message, since, retryAt, lastOk }` | the job failed (a page, the database) | a delta at `retryAt` |
| `offline { since, retryAt, lastOk }` | Jira couldn't be reached | a delta at `retryAt` |
| `paused { message }` | Jira answered 401/403 | nothing until `connection_changed` (Save on the Connection page) |

`run_job` is the pure part: it checks the connection, runs the job and returns the state after it. Tests call it
directly. The engine passes each result through `backoff.rs` `Failures`: `since` is the first failure in a row,
`retryAt` is a minute later while offline and 1, 2, 4, 8, then 15 minutes later after each failed job in a row;
`lastOk` is the last job that worked. A job that works ends the streak.

## Jobs

**Full** (`full.rs`):

1. Metadata: `/field` (the site's Sprint and Epic Color fields are found by their custom type), projects,
   statuses, versions of every tracked project, `/myself` (account id, name, time zone).
2. Per tracked project, in Space order: Jira's approximate count of `(<scope>) AND project = KEY`, then that
   query `ORDER BY updated ASC` with `expand=renderedFields` and `SEARCH_FIELDS` plus the sprint, epic color
   and the custom fields opted in (`field_choice`) or used by a view's column or a filter (`fetched_fields`); no
   other custom field is downloaded ([ADR 0056](adr/0056-custom-fields-are-opted-in-app-wide-a-space-opts-out.md)),
   and unticking one forgets its cached values. Every page is stored in one transaction and announced as `sync_progress`.
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
comments, changes and seen rows. An issue Jira returns that is outside the scope (done for longer than the scope
keeps, moved to an untracked project) stays cached with `issue.left_scope_at` and is asked for again only after 7
days (`RECHECK_LEFT_SCOPE`), so one deleted since is still noticed; the mark goes when the scope lists it again.
Seen changes older than `sync.keepHistoryDays` (Settings → Sync, 90 days by default) are pruned ([data.md](data.md#tables)). Runs after every full sync and hourly.

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
`background_minutes` otherwise (15), and right away after the Mac woke (a tick more than 2 minutes late). While
a sync is failing, only a delta runs, at the failure's `retryAt` or after a wake. ⌘R (`sync:refresh`, also the
status bar's sync state and `taskboard sync`) queues a delta, ⌘⇧R (`sync:full`, `taskboard sync --full`) a full sync.

## Own changes

The owner's own changes aren't news (`sync::own`, `sync::store`), once Test Connection knows the account:

- A page records no `created` for an issue the owner reported, and no `comment_added` or `mention` for new
  comments that are all the owner's; the issue is then seen up to its `updated` (`changes::catch_up`) unless other
  changes are unseen.
- After a delta or full sync, the 50 most recently updated issues with a recorded status, assignee, priority, due
  date or `updated` change get Jira's changelog (`Source::changelog`), kept as their history. A field change whose
  last changelog entry is the owner's goes; an `updated` row goes when every entry since `seen_at` is the owner's.
  A failure keeps the changes as recorded.

A project's first sync (`store::Pass`): with no issue read yet (a new Space) every issue counts as seen. Once the
owner read some (a fresh cache: a reset, memory mode after a restart), an issue updated after `issue_seen.seen_at`
records `updated` ("Updated" in the inbox) and one never seen records `created`.

## Storing a page

`map/` turns an issue's JSON into rows (pure): the issue with every column `docs/data.md` lists and the search
document (`issue.rs`), its comments (`comments.rs`: Jira's HTML plus plain text for search, `text::html_to_text`),
and the people it names (`users.rs`). Timestamps are stored as UTC with milliseconds, so they sort as text
(`time::utc`, `time::format_utc`). The sprint is the active one, else the one with the highest id;
an epic is hierarchy level 1 or type "Epic".

`store.rs` writes them, skipping a page older than the cached issue (`updated`) and keeping the value of a
write on its way (`writes::overlay`, [writes.md](writes.md)): the issue row, its `issue_body` and its custom fields' values (`raw.rs`,
[ADR 0041](adr/0041-an-issues-big-texts-live-beside-its-row.md)). An issue whose kept JSON and custom values equal
the cached ones is skipped, so a sync without changes announces nothing; the kept JSON leaves out Jira's
relative rendered dates and keeps each comment's edit time, so an edited comment still counts. Otherwise `diff.rs` compares the tracked fields with the cached row and writes
`issue_change` rows (`status`, `assignee`, `due_date`, `priority`, `comment_added`, `created`), stamped with the
new `updated`, and a `mention` row for each account an ADF `mention` node of a comment the cache hadn't seen
names (`sync::mentions`, `adf::mentions`). A project's first sync compares with nothing ([Own changes](#own-changes)). When the search returned
all of an issue's comments, they replace the cached ones.

## Events

| Event | Payload | When |
|---|---|---|
| `sync_state` | `SyncState` | the state changed |
| `sync_progress` | `SyncProgress { job, projectId, projectKey, pagesDone, fetched, approxTotal }` | before the first page and after every page |
| `issues_changed` | `IssuesChanged { ids, all }` | after a page stored changes (`Db::write`), merged to at most one per 500 ms |
| `issue_details_changed` | `IssuesChanged { ids }` | an opened issue's history or web links changed |

## Where to see it

- Settings → Sync → Synced: the state, Sync Now, and per tracked project the cached and open issues and the last
  full sync (`sync_overview`, `sync_run`).
- Settings → Privacy & Data → Jira data: "Keep Jira data in memory only" (`sync.memoryOnly`) empties the cache,
  keeps it in memory and syncs again. The cache's size (`SyncOverview.cache_bytes`) with Reset Cache, or Refetch
  from Jira in memory mode; in file mode also how long read history stays and the switch that clears the cache at
  every quit (`sync.clearCacheOnQuit`; also at launch, for a session that didn't quit cleanly). Read marks stay.
  Show Data Folder opens the folder, so the owner can see that no `cache.sqlite` is there.
- `taskboard status [--json]`: the same overview; `taskboard sync [--full]` queues a sync and returns;
  `taskboard reset-cache --yes` empties the cache and queues a full sync.
- The main window shows "Loading your Jira issues" with per-project progress while the first sync runs.
