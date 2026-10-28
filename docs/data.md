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
| `nav.json` | The main window's place: the open level and view, each level's last view, the expanded Spaces (`nav/`). A place that no longer exists falls back to the level's default view, then to Everything | kept |

The Jira token is in the Keychain only, never in a file or a log.

## Settings (`settings.json`)

`store/settings/`; missing keys read as defaults, values out of bounds are clamped on load.

| Section | Holds | Changed by |
|---|---|---|
| `connection` | site URL (normalised), email, token kind, token expiry date, cloud id, and what Test Connection found: account id, display name, time zone. Never the token (Keychain, service `APP_NAME`, account = email) | `connection_save`, `connection_test` |
| `sync` | scope override (JQL or none), delta interval in front (1–60 min) and in the background (5–120 min), days to keep done issues (7–365) | `scope_save`, `sync_save` |

`update_settings` replaces only what the page owns; `connection` and `sync` change through their own
commands, which validate them. Tracking a project writes `local.space`, not the settings.

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
| `space` | a tracked project as the sidebar shows it: name, icon, colour, client, position, `default_columns` for new views |
| `filter_list` | WHICH issues: the sidebar tree (`kind` everything, space, list, recent, pinned); a JSON filter ANDed with its ancestors |
| `view` | HOW to show a list: one tab; layout, columns, grouping, sort; never a filter |
| `pin` | pinned issues, comments and lists, with an optional reason and expiry |
| `status_style` | overrides the detected style of a status |

Project ids in `local` point into `cache`; SQLite can't enforce that across files, so code cleans up.

## Filter lists and views

A filter (`filter::Filter`, [ADR 0024](adr/0024-filters-run-locally-in-sql-views-never-filter.md)) is stored as
JSON; no condition is stored as null:

```json
{ "conditions": [
  { "field": "status", "op": "is", "value": "10003" },
  { "field": "customfield_10071", "op": "is", "value": "me" },
  { "field": "updated", "op": "withinLastDays", "value": 7 }
] }
```

- `field`: `status` (id), `statusCategory` (`new`, `indeterminate`, `done`), `type` (name), `assignee`,
  `reporter` (account id or `me`), `priority` (name), `epic` (the parent's issue id), `release` (fix version id),
  `sprint` (`active`, `future`, `closed` or a sprint id), `labels`, `due`, `updated`, `created`, `project` (id),
  or a custom field id (Jira schema types user, option, string, number, date, datetime, array).
- `op`: `is`, `isNot`, `in`, `notIn`, `isEmpty`, `isNotEmpty`, `contains` (labels and text fields), `before`,
  `after` (`YYYY-MM-DD`), `withinLastDays`, `withinNextDays` (a number of days; dates are days in the account's
  time zone).
- `value`: a string, a list of strings or a number of days, as the operator needs.

Built in (migration `local/0002`): Everything (one "All issues" view grouped by Space), and under Home Recently
changed and Pinned. Tracking a project (`lists::ensure_space`) adds its Space node with three views (All issues,
Board, Table; grouped by status; the Space's default columns, else Assignee, Epic, Release, Priority) and the
filter lists Current sprint, Epics, Release *next unreleased version* (when the project has one), Open bugs, each
with one List view. Positions among siblings stay 0, 1, 2, … (`order.rs`). Deleting a filter list deletes its
views and pins; a level keeps at least one view.

## Rules

- Timestamps are UTC ISO 8601; the UI shows local time.
- Migrations are numbered SQL files in `src-tauri/migrations/<file>/`, listed in `db/migrate.rs`. Never edit
  a shipped one; add the next number. A file from a newer Taskboard is refused.
- Filters are evaluated in SQL on the cache, never sent to Jira as JQL; Jira is only queried by the sync.
