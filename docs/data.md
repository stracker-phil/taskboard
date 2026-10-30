# Data

Where Taskboard keeps what, and the rules for changing it. Decision:
[ADR 0019](adr/0019-two-sqlite-files-through-rusqlite.md).

## Files

All in the config dir (`~/Library/Application Support/com.philippstracker.taskboard/`, debug `….debug/`),
0600 in a 0700 folder:

| File | Holds | Reset cache |
|---|---|---|
| `local.sqlite` | The owner's setup: Spaces, filter lists, views, bookmarks (`pin`), status style overrides | kept |
| `cache.sqlite` | The tracker mirror (Jira today): spaces, issues, comments, people, versions, fields, change log, seen marks, sync state, search index | deleted and recreated empty |
| `settings.json`, `keybindings.json`, `ui_state.json`, `places.json` | Settings, key overrides, window layout (the hidden sidebar, the activity column's width), each view's remembered place (`store/`; no longer the Me chip or the text filter, [ADR 0038](adr/0038-filter-changes-are-drafts-until-the-view-is-saved.md): an old `onlyMine` seeds `view.only_mine` once at startup, `text` is ignored) | kept |
| `nav.json` | The main window's place: the open level and view, each level's last view, the expanded Spaces (`nav/`). A place that no longer exists falls back to the level's default view, then to Everything | kept |

Attachments shown in Quick Look are downloaded to `~/Library/Caches/<bundle identifier>/attachments/<id>/<name>`
(`issues::preview`, [ADR 0035](adr/0035-attachments-preview-in-quick-look.md)); the folder is removed when the app
starts and quits, and a file already there with the size Jira lists is reused.

The Jira token is in the Keychain only, never in a file or a log.

## Settings (`settings.json`)

`store/settings/`; missing keys read as defaults, values out of bounds are clamped on load.

| Section | Holds | Changed by |
|---|---|---|
| `connection` | site URL (normalized), email, token kind, token expiry date, cloud id, and what Test Connection found: account id, display name, time zone. Never the token (Keychain, service `APP_NAME`, account = email) | `connection_save`, `connection_test` |
| `sync` | scope override (JQL or none), delta interval in front (1–60 min) and in the background (5–120 min), days to keep done issues (7–365) | `scope_save`, `sync_save` |

`update_settings` replaces only what the page owns; `connection` and `sync` change through their own
commands, which validate them. Tracking a project writes `local.space`, not the settings.

## Access (`src-tauri/src/db/`)

- `Db::open` creates or migrates both files, then opens one writer and two readers. Each connection opens
  `local.sqlite` and attaches `cache.sqlite` as `cache`, so cache tables are written `cache.issue`.
- `Db::read(|conn| …)` runs a query on a reader; `Db::write(|tx, changes| …)` is the only way to change data.
  The closure records what it changed (`changes.issue_changed(id)`, `changes.detail_changed(id)` when only the
  issue's detail changed, `changes.local_changed(LocalPart::Views)`); after the commit the app emits
  `issues_changed { ids, all }`, `issue_details_changed { ids }` and `local_changed { parts }`. A failed write
  rolls back and emits nothing. Writes in a burst (a sync's pages) are merged (`db::coalesce`): the first is
  announced at once, the rest at most every `ANNOUNCE_EVERY` (500 ms), the last one always.
- The page refetches by event (`src/queries/useEvents.ts`): `issues_changed` for named issues refreshes the
  rows, the sidebar, the bookmarks, the counts and those issues' detail; with `all` everything an issue shows
  up in; `issue_details_changed` only those issues' detail.
- Every connection keeps a 16 MB page cache and memory-maps up to 256 MB per file (`connection::tune`).
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
| `issue` | keyed by the numeric Jira id (as text), never the key. Normalized columns for everything the UI filters, groups or sorts on, `raw` for the rest (attachments are read from `raw`; migration `cache/0009` clears the delta cursor so the next sync is a full one that fills them). Ours, derived during sync: `status_category`, `type_is_subtask`, `is_epic`, `sprint_id`, `sprint_state`, `comment_count`, `comments_synced_at`, `links_synced_at`, `history_synced_at`, `synced_at`, `gone` |
| `issue_change` | what the sync saw change (status, assignee, due date, priority, comments, created) |
| `issue_seen` | when the owner last looked at an issue: changes after it are unseen (`changes::UNSEEN`); `unread` (migration `cache/0006`): marked unread by the owner, unread until read again |
| `comment` | `body_html` from Jira, `body_text` (ours) for search, `body_adf` (migration `cache/0010`, [ADR 0036](adr/0036-rich-text-renders-from-adf-not-renderedfields.md)): the body as ADF, which the page is shown from (`adf::html_or`); null on rows from before the migration until the next sync, which then show `body_html` |
| `remote_link` | an issue's web links (migration `cache/0004`): `issue_id` (cascades), `id`, `position`, `url` (untrusted), `title`, `application`, `relationship`, `source`; replaced as a whole when the issue is opened, at most every 5 minutes: `issue.links_synced_at` (migration `cache/0005`) holds the last fetch (`issues::issue_sync_web_links`, `issues::fetched::FRESH_FOR`) |
| `issue_history` | an issue's history, Jira's changelog (migration `cache/0007`, [ADR 0031](adr/0031-issue-history-comes-from-the-changelog-on-demand.md)): `issue_id` (cascades), `position` (oldest first), `field` (Jira's name: `status`, `Fix Version`, `Sprint`), `field_id`, `from_value`/`to_value` (ids: status, account, version), `from_text`/`to_text`, `author_id`, `changed_at` (UTC, sorts with `issue_change`), `source`; replaced as a whole when the issue is opened, at most every 5 minutes: `issue.history_synced_at` holds the last fetch (`issues::issue_sync_history`). Never read by `changes::UNSEEN` |
| `user` | people: `account_id`, `name`, `avatar_url`: the 48 px picture's address from Jira's user objects (assignee, reporter, comment and history authors), stored only when it is on Atlassian's avatar host (`net::AvatarUrl`, [ADR 0032](adr/0032-avatars-come-from-atlassians-avatar-host-through-rust.md)) |
| `avatar` | people's profile pictures (migration `cache/0008`, `avatars/`): `account_id`, `url` (the address last asked for), `mime`, `bytes` (the last picture that arrived, kept while a newer one fails), `fetched_at`, `failed_at`. Asked again after `avatars::FRESH_FOR` (7 days) or when `user.avatar_url` changed; not within `avatars::RETRY_AFTER` (1 day) of a failure |
| `version`, `field` | releases, custom field names (`field.shown` is ours; `field.used`, migration `cache/0002`: a tracked project's recent issues fill it, so the Columns popover and the filter editor offer it, `filter::RELEVANT_FIELD`) |
| `sync_state` | one row per sync scope, or per project for the full sync's cursor |
| `issue_fts` | FTS5 over key, summary, description and comments; rowid = the issue id; keeps its text for snippets; `remove_diacritics 2`. Queried by the palette and the `text` filter field, always through `search::match_expression` (quoted prefix terms, so typed FTS5 syntax stays literal) |

**local** (the owner's):

| Table | Notes |
|---|---|
| `space` | a tracked project as the sidebar shows it: name, icon, color (`1`–`12`), client, position, `default_columns` for new views; its Space settings (`local/0007`): `layouts` (JSON array of the layouts its views offer; null = all), `default_layout` and `default_group` for new views |
| `filter_list` | WHICH issues: the sidebar tree (`kind` everything, space, list, recent, pinned); a JSON filter ANDed with its ancestors |
| `view` | HOW to show a list: one tab; layout, columns, grouping, sort; `filter` (`local/0012`): its own conditions, ANDed with its list's ([ADR 0033](adr/0033-views-can-hold-their-own-filter.md)); `subtasks` (`local/0013`): how subtasks show ([ADR 0037](adr/0037-views-choose-how-subtasks-show.md)); `only_mine` (`local/0014`): the saved Me chip ([ADR 0038](adr/0038-filter-changes-are-drafts-until-the-view-is-saved.md)) |
| `pin` | bookmarked issues, comments and lists (Bookmarks in the UI), with an optional reason and expiry; active while not removed and not expired (`pins::active`); ended bookmarks stay in Recently removed for 7 days |
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

A view holds the same JSON in `view.filter` (migration `local/0012`, [ADR 0033](adr/0033-views-can-hold-their-own-filter.md)):
its own conditions, ANDed with its level's chain, so two views of one list can show different issues. Only the
view's issues are narrowed (`view_rows`, `view_board`, their groups and Board columns, and Mark all as read from
that view); the sidebar counts stay with the level. A view filter that can't be read or compiled is an error of that view only.
`view_set_filter(id, filter)` compiles it under the level first and marks its custom fields `shown`;
`list_conditions(id, viewId)` returns the view's conditions next to the level's (`ListConditions.view`,
`viewError`), and `View.ownConditions` counts them for the Filter badge. Duplicating a view or a filter list
copies the view filters and `view.only_mine` (`View.onlyMine`, set by `view_set_only_mine`).

The page edits both layers and the Me chip as drafts ([ADR 0038](adr/0038-filter-changes-are-drafts-until-the-view-is-saved.md)):
`Narrowing.listDraft` and `Narrowing.viewDraft` stand in for the filter list's own and the view's stored conditions in
`view_rows` and `view_board` (a list draft only on a filter list), compiled under the same chain as the setters check
them, and nothing is written until Save View calls `list_set_filter`, `view_set_filter` and `view_set_only_mine`.
`list_match_count`, the popover's live count, counts every matching issue, like the sidebar, and with `me` only
the connected account's issues, like a view's `Narrowing.me`.

Built in (migration `local/0002`): Everything (one "All issues" view grouped by Space), and under Home Unread
(kind `recent`) and Bookmarks (kind `pinned`; renamed by `local/0008`). Tracking a project (`lists::ensure_space`) adds its Space node with three views (All issues,
Board, Table; grouped by status; the Space's default columns, else Assignee, Epic, Release, Priority) and the
filter lists Current sprint, Epics, Release *next unreleased version* (when the project has one), Open bugs, each
with one List view. Positions among siblings stay 0, 1, 2, … (`order.rs`). Deleting a filter list deletes its
views and bookmarks; a level keeps at least one view.

### Group and sort keys

`view_rows(listId, viewId, narrowing)` returns the view's issues as `ViewData { groupBy, subtasks, groups,
rows, columns }`: the level's filter, plus the view's own conditions, plus the kind's own rule (Unread: issues with unseen changes or marked
unread, open or done, plus those read since the owner came to this place; Bookmarks: the actively bookmarked
issues), plus the narrowing (`me`: the view's Me chip as drafted or saved, `text` in the key or summary ignoring
case and accents, `epic`, and the draft conditions `listDraft` and `viewDraft`).

A view's subtasks are the descendants of its results through `cache.issue.parent_id`, of any type
([ADR 0040](adr/0040-subtasks-are-the-children-of-a-views-results.md); `type_is_subtask` plays no part), down to
`views::subtasks::MAX_DEPTH` (5) levels, gone and untracked issues left out, fetched in one recursive CTE per
load (`subtasks::descendants`). They follow `view.subtasks` (migration `local/0013`, `view_set_subtasks`; null
reads `hidden`; duplicating copies it): `hidden` shows the results only; `list` adds every descendant as a row of
its own (`Row.isSubtask` when it isn't a result itself), grouped and sorted on its own values, with `me` and
`text` narrowing every row; `hierarchy` (`subtasks::nest`, `subtasks::expand`) groups the results that have no
ancestor among the results and puts each one's subtree right after it in depth order (`Row.depth`), unfiltered,
siblings sorted by the view's sort, with the group's count counting the roots and `me` and `text` narrowing the
roots only. Every row carries `childCount` (its direct children in the cache) and its `parentId`. `view_board`
always shows the results only. The sidebar counts the results. `places.json` keeps a view's collapsed rows
(`ViewPlace.collapsedParents`). `list_mark_all_seen(listId, viewId, narrowing)` marks the unseen issues among
exactly the rows the view shows (`views::unseen_ids` with a `ViewAt`); without a view, the level's.

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
