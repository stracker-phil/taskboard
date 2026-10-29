# 0031. Issue history comes from the changelog, on demand

## Status

Accepted (2026-10-29). Owner's request: the Activity showed almost only comments.

## Context

The Activity's field changes came only from `issue_change`, which the sync writes when it sees a tracked field
differ between two versions of an issue. Nothing that happened before Taskboard first saw an issue showed, the
changes carried no author, and only four fields were tracked. Jira keeps every issue's full changelog
(`GET issue/{id}/changelog`): who changed which field, when, from what to what.

## Decision

- **The changelog is fetched when an issue is opened**, like its web links: `issues::issue_sync_history`
  reads every page and replaces the issue's rows in `cache.issue_history` (migration `cache/0007`). It asks
  at most once per 5 minutes per issue (`issue.history_synced_at`, `issues::fetched::FRESH_FOR`), so moving
  through issues doesn't ask Jira for each again. The gateway allows `GET issue/{idOrKey}/changelog` with
  `startAt` and `maxResults` only.
- **The Activity merges both.** History entries show with their author and every field; long text
  (description, environment, values over 120 characters) only names the field. Once an issue has history,
  recorded changes at or before its newest entry are dropped as duplicates; newer ones still show until the
  next fetch, and `created` stays.
- **Unread stays with the sync.** Only `issue_change` decides whether an issue has unseen changes; fetching
  history never makes an issue unread, it only marks fresh items in the Activity.
- Rejected: fetching changelogs during the sync (`expand=changelog` on the search, or one request per changed
  issue): it multiplies the sync's traffic for issues the owner never opens, and Jira caps the expanded
  changelog at 100 entries. Rejected: replacing `issue_change` by the changelog: unread must work for issues
  the owner hasn't opened.

## Consequences

- An issue never opened shows only recorded changes; one opened shows its whole past.
- Two stores of field changes with a time-based dedupe: a recorded change stamped later than the change itself
  (another edit followed before the sync saw it) can show twice.
- Another source implements `Source::changelog` in Jira's shape ([ADR 0029](0029-issues-come-from-a-source-jira-is-the-first.md)).
