# 0057. Jira data can live in memory only

## Status

Accepted (2026-11-10). Owner's request after several weeks of use, first as an option. Refines
[0019](0019-two-sqlite-files-through-rusqlite.md): the cache is a file or an in-memory database.

## Context

The cache file holds company data (issues, comments, people) between sessions. Clearing it on quit
([`sync.clearCacheOnQuit`](../data.md#settings-settingsjson)) still writes it to disk while the app runs, and a
crash leaves it there until the next launch. Every read path joins `local` and `cache` tables in one SQL
statement, so replacing SQLite with Rust structs would mean rewriting all of them.

## Decision

- `sync.memoryOnly` (off by default; Settings → Sync → Cache) attaches the cache as an in-memory SQLite database
  instead of `cache.sqlite`: `ATTACH 'file:/taskboard-cache-<uuid>?vfs=memdb' AS cache` on every connection
  (`db::connection::Cache`). The memdb VFS shares one database by name between the connections of a process, so
  readers and the writer see the same data. A keeper connection holds it open; replacing the cache drops the
  keeper and frees it. The name is new for every cache, so two `Db`s (tests) never share one.
- Not `mode=memory&cache=shared`: shared-cache mode answers `SQLITE_LOCKED`, which the busy timeout doesn't wait
  for. memdb answers `SQLITE_BUSY`, which it does.
- Turning the setting on or off works like Reset Cache (`Db::switch_cache`): detach everywhere, delete
  `cache.sqlite` and its `-wal`/`-shm`, attach an empty cache of the other kind, announce
  `issues_changed { all }`, queue a full sync. A launch in memory mode also deletes a leftover file.
- The file-only upkeep does nothing in memory mode: no WAL, no compaction, no cache size, no clear on quit.
- Everything the owner sets up stays in `local.sqlite`, as before.

## Consequences

- No Jira data is written to disk; each launch syncs again and shows empty views until the first sync.
- The memdb uses a rollback journal in memory: a write blocks readers for its transaction, which the busy
  timeout covers. The sync writes in short page transactions.
- Memory use grows with the cache, as the file did on disk.
- The disk cache goes in a later step once the owner has tried this mode.
