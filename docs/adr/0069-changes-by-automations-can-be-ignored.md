# 0069. Changes by automations can be ignored

## Status

Accepted (2026-11-12). Owner's decisions. Builds on [0068](0068-notification-settings-decide-what-counts.md) and
[0058](0058-own-changes-are-not-news.md).

## Context

A QA run (a test management app and a CI integration) changes and comments on hundreds of issues. Notification
settings can leave out issue types, but an automation also touches the issues the owner cares about. To leave out
its changes, each recorded change needs its author, and the author's kind. Until now only the 50 newest changed
issues got their changelog after a sync, and only to drop the owner's own changes.

## Decision

- **An automation is an app account**: Jira's `accountType` `app` (Xray, CI integrations, Jira Automation).
  `cache.user.account_type` keeps it; an answer without it keeps the known one.
- **The author per row**: `issue_change.author_id`, recorded with the change where the sync can tell (the reporter
  for `created`, the comments' author for `comment_added` and `mention`, the changelog for field rows).
- **The changelog comes with the change sync's search** (`expand=changelog`), used only when it is complete
  (`startAt + histories >= total`). Rejected: a changelog request per changed issue (a 300-issue QA run would
  need 300–900 changelog requests per delta at 5 requests per second, a minute or more of a sync that should take
  seconds), and the experimental `changelog/bulkfetch` (not stable, and still requests of its own). The 50-issue
  post-pass stays for what the search can't tell (a partial changelog, the full sync and reconciliation, which
  don't ask for it).
- **The expand never breaks a sync**: a page that fails with a server error, an unreadable answer or a `400`
  naming the expand is asked again once without the changelog, and the delta goes on without it.
- **Unknown counts**: a change without a known author, or by an account of unknown type, is not an automation's.
- **About you still gets through**: a mention, an assignment or being added to a people field by an app counts.
- **On by default**: "Ignore changes made by automations" under Which issues; an old settings file reads it on.
  An ignored change stays in the activity; it never makes an issue unread or notifies.

## Consequences

- Migration `cache/0017` adds `issue_change.author_id` and `user.account_type`; account types are ids, not text a
  person wrote, so the cache stays within [0066](0066-jira-data-lives-in-memory-only.md).
- The change sync's answers grow by each issue's recent history; no new token scope (`read:jira-work`).
- Accepted gap: changes recorded before the first change sync after a launch (the full sync), of issues past the
  50 the post-pass checks, or whose changelog entry is missing keep an unknown author and count.
- The owner's own field changes are dropped when the page is stored, where the changelog is complete, instead of
  only after the sync.
