# 0066. Jira data lives in memory only; no issue text on disk

## Status

Accepted (2026-11-12). Owner's decision after using memory mode. Supersedes
[0057](0057-jira-data-can-live-in-memory-only.md) and the cache file of [0019](0019-two-sqlite-files-through-rusqlite.md).

## Context

Since 0057 the Jira mirror could live in memory, as an opt-in beside `cache.sqlite`. The owner kept it on and
wants it to be the only way: company data should never sit on the Mac's disk.

## Decision

- The mirror is always an in-memory database attached as `cache` (`db::connection::Cache`), empty at launch and
  gone at quit. A `cache.sqlite` an earlier version left is deleted at launch. Reset Cache (Refetch from Jira)
  replaces it with an empty one and syncs again.
- Gone: `CacheKind`, `sync.memoryOnly`, `sync.clearCacheOnQuit`, `sync.keepHistoryDays` (a fixed 90 days), Compact,
  the cache file upkeep, the status bar's memory chip, and the one-time migrations that read the cache file.
- Standing rule (`.claude/rules/no-issue-data-on-disk.md`): nothing a person wrote in Jira is written to disk;
  files hold ids, keys, timestamps and the owner's own setup. Logs name issues by key. Exceptions: the record of
  changes sent to Jira keeps picked values in full and typed text as its first 10 characters and length
  (`writes::sent::typed`); Quick Look previews download an attachment to the Caches folder, deleted at launch
  and quit.

## Consequences

- Every launch runs a full sync before lists fill; the owner accepted that.
- Read marks, bookmarks and the setup survive in `local.sqlite`; changes recorded before a quit are gone, so the
  inbox starts from what the first sync finds (`sync::news`).
