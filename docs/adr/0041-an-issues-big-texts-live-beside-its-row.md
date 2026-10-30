# 0041. An issue's big texts live beside its row

## Status

Accepted (2026-10-30). Performance cleanup. Refines [0019](0019-two-sqlite-files-through-rusqlite.md) (what
`cache.issue` holds) and [0023](0023-one-sync-task-full-sync-per-project-changes-by-diffing.md) (how an unchanged
issue is recognised).

## Context

`cache.issue` kept the description (HTML and ADF) and `raw`, Jira's whole issue JSON with its rendered fields and
comments, in the middle of every row. Every view, count and child count reads every issue row, so each read
walked the overflow pages of rows tens of KB long. Custom columns and custom-field filters ran
`json_extract(raw, …)` on that JSON per row. `raw` also repeated what the description columns and the `comment`
table hold, and Jira renders dates in `renderedFields` relative to now ("2 hours ago"), so `raw` could differ on
every fetch and defeat the sync's unchanged-issue skip.

## Decision

- **`issue_body` holds the big texts**, one row per issue (migration `cache/0011`, cascades with the issue):
  `description_html`, `description_adf` and `raw`. Only the Issue view, attachments, links and the sync's
  comparison read it, one issue at a time.
- **`issue.custom` holds the custom fields' values**: a JSON object of the non-null `customfield_*` values. Views
  and filters read it with `json_extract(i.custom, '$."customfield_…"')`.
- **`raw` keeps only what nothing else holds** (`sync::raw::kept_raw`): id, key, the fields without the
  description, the custom fields and the comments, each comment's id and edit time (so an edited comment still
  changes it), and the rendered custom fields (the Issue view's fallback for text fields without ADF).
- **An issue is unchanged** when its kept `raw` and `custom` equal the cached ones and it isn't gone.
- **The key is indexed** (`issue_key`, case-insensitive) and so is the number after its dash (`issue_key_number`),
  for the palette, links and `taskboard open`.
- Rejected: keeping `raw` whole in a side table (still the duplicates and the relative dates); dropping `raw`
  (attachments, links and rendered custom fields need it); a hash of the issue for the comparison (another column,
  and a collision would hide a change).

## Consequences

- List reads touch only small rows; a cache shrinks by the duplicated descriptions, comments and rendered fields
  (the file itself shrinks only when SQLite reuses the freed pages).
- The migration trims existing rows in SQL without the comment versions, so each issue is written once more on
  its next fetch; no change is recorded for that, as the tracked fields are equal.
- Code that needs more of Jira's issue JSON adds it to `kept_raw` and gets it on the next full sync.
