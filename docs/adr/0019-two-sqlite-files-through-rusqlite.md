# 0019. Two SQLite files through rusqlite: the owner's setup and a disposable Jira cache

## Status

Accepted (2026-10-28), confirmed by the owner. The spec's single database file is split in two.

## Context

Taskboard mirrors Jira into SQLite and reads only from it. The spec's "Reset cache deletes the file" would
also delete what the owner built by hand: filter lists, views, pins, Space settings and status overrides.
TimeTracker uses sqlx with a pool; Taskboard's spec picks rusqlite, and one user needs no pool.

## Decision

- **Two files** in the config dir: `local.sqlite` (the owner's setup) and `cache.sqlite` (the Jira mirror, its
  change log, seen marks and search index). Every connection opens the local file and `ATTACH`es the cache as
  `cache`, so one query can join both. Reset cache deletes `cache.sqlite` only; the next full sync refills it.
  The site and the connection live in `settings.json`, never in either file; the token lives in the Keychain.
- **rusqlite** with the bundled SQLite (FTS5 included; the app refuses to start without it). WAL,
  `synchronous=NORMAL`, foreign keys on, a 5 s busy timeout. One writer connection and two readers; every call
  runs on a blocking thread.
- **Migrations** are numbered SQL files per file (`migrations/local/`, `migrations/cache/`), embedded in the
  binary and counted by `PRAGMA user_version`. A shipped migration is never edited. A file from a newer
  Taskboard is refused, never touched.
- **One write path:** `Db::write` runs a closure in a transaction and, after the commit, announces what the
  closure recorded (`issues_changed`, `local_changed`).
- **Search keeps its text:** `issue_fts` is a normal FTS5 table, so search can show snippets.
- **Extra columns** the UI filters on (sprint state, comment count, status category, …) are stored on `issue`.
- The config dir is the bundle identifier's (`com.philippstracker.taskboard`), so the debug app has its own.

## Consequences

- No foreign keys across the two files; code removes local rows a vanished project leaves behind.
- Files are 0600 in the 0700 config dir; FileVault encrypts them at rest.
- `docs/data.md` lists every table and which columns are Jira's and which are the owner's.
