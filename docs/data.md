# Data

Where Taskboard keeps what, and the rules for changing it. Decision:
[ADR 0019](adr/0019-two-sqlite-files-through-rusqlite.md).

## Files

All in the config dir (`~/Library/Application Support/com.philippstracker.taskboard/`, debug `….debug/`),
0600 in a 0700 folder:

| File | Holds | Reset cache |
|---|---|---|
| `local.sqlite` | The owner's setup: Spaces, filter lists, views, pins, status style overrides | kept |
| `cache.sqlite` | The Jira mirror: projects, issues, comments, people, versions, fields, change log, seen marks, sync state, search index | deleted and recreated empty |
| `settings.json`, `keybindings.json`, `ui_state.json` | Settings, key overrides, window layout (`store/`) | kept |

The Jira token is in the Keychain only, never in a file or a log.

## Access (`src-tauri/src/db/`)

- `Db::open` creates or migrates both files, then opens one writer and two readers. Each connection opens
  `local.sqlite` and attaches `cache.sqlite` as `cache`, so cache tables are written `cache.issue`.
- `Db::read(|conn| …)` runs a query on a reader; `Db::write(|tx, changes| …)` is the only way to change data.
  The closure records what it changed (`changes.issue_changed(id)`, `changes.local_changed(LocalPart::Views)`);
  after the commit the app emits `issues_changed { ids, all }` and `local_changed { parts }`. A failed write
  rolls back and emits nothing.
- `Db::reset_cache` detaches the cache everywhere, deletes it, creates an empty one and emits
  `issues_changed { all: true }`.

## Tables

**cache** (Jira's data, plus our derived columns):

| Table | Notes |
|---|---|
| `project` | Jira fields only (`id, key, name, avatar_url, raw`); the owner's Space settings are in `local.space` |
| `status` | `id, name, category` (`new`, `indeterminate`, `done`) |
| `issue` | keyed by the numeric Jira id (as text), never the key. Normalised columns for everything the UI filters, groups or sorts on, `raw` for the rest. Ours, derived during sync: `status_category`, `type_is_subtask`, `is_epic`, `sprint_id`, `sprint_state`, `comment_count`, `comments_synced_at`, `synced_at`, `gone` |
| `issue_change` | what the sync saw change (status, assignee, due date, priority, comments, created) |
| `issue_seen` | when the owner last looked at an issue |
| `comment` | `body_html` from Jira, `body_text` (ours) for search |
| `version`, `user`, `field` | releases, people, custom field names (`field.shown` is ours) |
| `sync_state` | one row per sync scope, or per project for the full sync's cursor |
| `issue_fts` | FTS5 over key, summary, description and comments; rowid = the issue id; keeps its text for snippets; `remove_diacritics 2` |

**local** (the owner's):

| Table | Notes |
|---|---|
| `space` | a tracked project as the sidebar shows it: name, icon, colour, client, position |
| `filter_list` | WHICH issues: the sidebar tree; a JSON filter ANDed with its ancestors |
| `view` | HOW to show a list: one tab; layout, columns, grouping, sort; never a filter |
| `pin` | pinned issues, comments and lists, with an optional reason and expiry |
| `status_style` | overrides the detected style of a status |

Project ids in `local` point into `cache`; SQLite can't enforce that across files, so code cleans up.

## Rules

- Timestamps are UTC ISO 8601; the UI shows local time.
- Migrations are numbered SQL files in `src-tauri/migrations/<file>/`, listed in `db/migrate.rs`. Never edit
  a shipped one; add the next number. A file from a newer Taskboard is refused.
- Filters are evaluated in SQL on the cache, never sent to Jira as JQL; Jira is only queried by the sync.
