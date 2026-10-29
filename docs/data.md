# Data

Where Taskboard keeps what, and the rules for changing it. Decision:
[ADR 0019](adr/0019-two-sqlite-files-through-rusqlite.md).

## Files

All in the config dir (`~/Library/Application Support/com.philippstracker.taskboard/`, debug `….debug/`),
0600 in a 0700 folder:

| File | Holds | Reset cache |
|---|---|---|
| `local.sqlite` | The owner's setup: Spaces, filter lists, views, pins, status style overrides | kept |
| `cache.sqlite` | The tracker mirror (Jira today): spaces, issues, comments, people, versions, fields, change log, seen marks, sync state, search index | deleted and recreated empty |
| `settings.json`, `keybindings.json`, `ui_state.json`, `places.json` | Settings, key overrides, window layout, each view's remembered place (`store/`) | kept |
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

**cache** (the tracker's data, plus our derived columns). `space`, `issue`, `comment`, `remote_link` and `issue_history` carry `source` (`jira`
today; migration `cache/0003`, [ADR 0029](adr/0029-issues-come-from-a-source-jira-is-the-first.md)); ids are
unique across sources (Jira's numeric ids; another source prefixes its own):

| Table | Notes |
|---|---|
| `space` | the tracker's projects (renamed from `project` in `cache/0003`): `id, key, name, avatar_url, raw, source`; the owner's Space settings are in `local.space`, so queries name this one `cache.space` |
| `status` | `id, name, category` (`new`, `indeterminate`, `done`) |
| `issue` | keyed by the numeric Jira id (as text), never the key. Normalised columns for everything the UI filters, groups or sorts on, `raw` for the rest. Ours, derived during sync: `status_category`, `type_is_subtask`, `is_epic`, `sprint_id`, `sprint_state`, `comment_count`, `comments_synced_at`, `links_synced_at`, `history_synced_at`, `synced_at`, `gone` |
| `issue_change` | what the sync saw change (status, assignee, due date, priority, comments, created) |
| `issue_seen` | when the owner last looked at an issue: changes after it are unseen (`changes::UNSEEN`) |
| `comment` | `body_html` from Jira, `body_text` (ours) for search |
| `remote_link` | an issue's web links (migration `cache/0004`): `issue_id` (cascades), `id`, `position`, `url` (untrusted), `title`, `application`, `relationship`, `source`; replaced as a whole when the issue is opened, at most every 5 minutes: `issue.links_synced_at` (migration `cache/0005`) holds the last fetch (`issues::issue_sync_web_links`, `issues::fetched::FRESH_FOR`) |
| `issue_history` | an issue's history, Jira's changelog (migration `cache/0007`, [ADR 0031](adr/0031-issue-history-comes-from-the-changelog-on-demand.md)): `issue_id` (cascades), `position` (oldest first), `field` (Jira's name: `status`, `Fix Version`, `Sprint`), `field_id`, `from_value`/`to_value` (ids: status, account, version), `from_text`/`to_text`, `author_id`, `changed_at` (UTC, sorts with `issue_change`), `source`; replaced as a whole when the issue is opened, at most every 5 minutes: `issue.history_synced_at` holds the last fetch (`issues::issue_sync_history`). Never read by `changes::UNSEEN` |
| `version`, `user`, `field` | releases, people, custom field names (`field.shown` is ours; `field.used`, migration `cache/0002`: a tracked project's recent issues fill it, so the Columns popover and the filter editor offer it, `filter::RELEVANT_FIELD`) |
| `sync_state` | one row per sync scope, or per project for the full sync's cursor |
| `issue_fts` | FTS5 over key, summary, description and comments; rowid = the issue id; keeps its text for snippets; `remove_diacritics 2`. Queried by the palette and the `text` filter field, always through `search::match_expression` (quoted prefix terms, so typed FTS5 syntax stays literal) |

**local** (the owner's):

| Table | Notes |
|---|---|
| `space` | a tracked project as the sidebar shows it: name, icon, colour (`1`–`12`), client, position, `default_columns` for new views; its Space settings (`local/0007`): `layouts` (JSON array of the layouts its views offer; null = all), `default_layout` and `default_group` for new views |
| `filter_list` | WHICH issues: the sidebar tree (`kind` everything, space, list, recent, pinned); a JSON filter ANDed with its ancestors |
| `view` | HOW to show a list: one tab; layout, columns, grouping, sort; never a filter |
| `pin` | pinned issues, comments and lists, with an optional reason and expiry; active while not unpinned and not expired (`pins::active`); ended pins stay in Recently unpinned for 7 days |
| `space_status_style` | overrides the detected style of a status in one Space (`local/0007`; the old app-wide `status_style` rows were copied to every Space) |
| `space_field` | how a custom field shows in one Space: `hiddenInList`, `hiddenInDetail` or `hidden`; no row = visible (`space_config`) |

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

Built in (migration `local/0002`): Everything (one "All issues" view grouped by Space), and under Home Unread
(kind `recent`) and Bookmarks (kind `pinned`; renamed by `local/0008`). Tracking a project (`lists::ensure_space`) adds its Space node with three views (All issues,
Board, Table; grouped by status; the Space's default columns, else Assignee, Epic, Release, Priority) and the
filter lists Current sprint, Epics, Release *next unreleased version* (when the project has one), Open bugs, each
with one List view. Positions among siblings stay 0, 1, 2, … (`order.rs`). Deleting a filter list deletes its
views and pins; a level keeps at least one view.

### Group and sort keys

`view_rows(listId, viewId, narrowing)` returns the view's issues as `ViewData { groupBy, groups, rows,
columns }`: the level's filter, plus the kind's own rule (Unread: issues with unseen changes or marked
unread, open or done, plus those read since the owner came to this place; Bookmarks: the actively bookmarked
issues), plus the narrowing (`me`, `text` in the key or summary ignoring
case and accents, `epic`).

| `groupBy` | Groups, in order |
|---|---|
| `status` | by style (In progress, In review, Blocked, To do, Backlog, Done), then name; Done starts collapsed |
| `statusType` | one group per style, in the same order |
| `project` | the Spaces in sidebar order |
| `client` | client labels A–Z, then Internal |
| `assignee` | me, the others A–Z, then Unassigned |
| `epic` | epics A–Z, then No epic |
| `release` | the first fix version, by release date, then No release |
| `priority` | Highest to Lowest, then No priority |
| `due` | Overdue, Today, This week (to Sunday), Later, No due date |
| `none` | one group, no header |

`sort.by`: `updated`, `created`, `priority` (highest first when descending), `key` (by number within a
project), `due`, `summary`, `status`; empty priorities and due dates sort last either way; ties keep the
issue id order. Columns are standard ids (`statusType`, `assignee`, `epic`, `release`, `priority`, `labels`, `sprint`,
`due`) or a Jira custom field id (`columns_catalogue` lists them; Jira's internal `any` fields and the sprint
field are left out). `view.widths` (migration `local/0003`) holds the Table layout's column widths by column id; `view.wrap_titles` (`local/0004`, default on) whether the List wraps long titles; `view.inline_labels` (`local/0011`, default on) whether labels show as tags after the title while the view has no Labels column; `view.open_with` (`local/0009`, `preview`, `full` or null for Settings → General) what a click on an issue opens; `filter_list.icon` (`local/0006`) and `filter_list.color`, `view.icon`, `view.color` (`local/0010`) the tile look (`look.rs`: a letter, digit or emoji, or a line icon name, on tile color 1–12; no icon shows the funnel or the layout's tile, no color the plain icon); `view.collapsed` (`local/0005`) a Board's collapsed or expanded columns by group key (a status id when grouped by status; the rest keep their default, Done collapsed). `view_board` returns one column per group of the view's `group_by`: by status (also for `none`) in the Board order (Backlog, To do, In progress, Blocked, In review, Done, then Jira's status order), by status type in that style order, otherwise the List's groups (`views::groups::group`) in their order. `read_view` and `list_views` fit a view to its Space (`space_config`): a layout the Space switched off shows as its default layout, `View.layouts` lists the offered ones, and columns of fields the Space hides from lists are left out; `columns_catalogue(list_id)` leaves them out too (outside the Spaces only fields every Space hides).
`space_status_style` holds the owner's style per Space and status (`status_style_set`); `status_style::Styles::of` resolves an issue's style with its own Space's choice.

## Rules

- Timestamps are UTC ISO 8601; the UI shows local time.
- Migrations are numbered SQL files in `src-tauri/migrations/<file>/`, listed in `db/migrate.rs`. Never edit
  a shipped one; add the next number. A file from a newer Taskboard is refused.
- Filters are evaluated in SQL on the cache, never sent to Jira as JQL; Jira is only queried by the sync.
