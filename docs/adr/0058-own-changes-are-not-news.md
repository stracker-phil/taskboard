# 0058. The owner's own changes aren't news

## Status

Accepted (2026-11-10). Owner's request after several weeks of use. Refines
[0023](0023-one-sync-task-full-sync-per-project-changes-by-diffing.md) (changes by diffing) and
[0044](0044-writes-show-at-once-and-settle-after-jira.md) (writes settle after Jira).

## Context

The sync records changes by comparing versions, without knowing who made them. Issues the owner created,
comments they wrote and statuses they changed in Jira showed as unread; so did what Jira changed along with a
write from Taskboard. A fresh cache (a reset, memory mode after a restart) compared with nothing and marked
everything read, so news from while the app was closed was lost.

## Decision

- After a write is accepted, the refetched issue is seen up to its `updated` (`changes::seen_until_updated`).
- The sync knows the connected account: no `created` for an issue it reported, no `comment_added` or `mention`
  for comments that are all its own.
- Field changes get their author from Jira's changelog (`sync::own`), fetched after each delta and full sync for
  at most 50 issues, newest first; the account's own changes are dropped. The changelog is kept as the history,
  so the Issue view needn't fetch it again.
- A project's first sync after the owner read some of its issues records `updated` for an issue updated after
  `issue_seen.seen_at` and `created` for one never seen. A new Space still starts all read.

## Consequences

- One changelog request per changed issue after a sync, capped at 50; issues past the cap keep their changes.
- In a fresh cache, any update after the last look counts, including fields the diff doesn't track.
- Without Test Connection nothing is dropped.
