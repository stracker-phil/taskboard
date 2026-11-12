# 0068. Notification settings decide what counts

## Status

Accepted (2026-11-12). Owner's decisions. Supersedes `behaviour.notifyAbout` (Settings → Behavior → Notify me
about) and, in part, [0059](0059-the-inbox-replaces-unread-mode.md): not every recorded change makes an issue
unread any more.

## Context

A QA run (test management issues changed by an integration) turned hundreds of issues unread. Every recorded
change counted in the inbox, and macOS notifications knew only three choices. The owner wants to choose what to
watch without a page that overwhelms.

## Decision

- **A matrix of events × channels** (Settings → Notifications): ten events (Mentions me, Assigns me, Unassigns me,
  Adds me to a people field, New comment, Moved to Done, Reopened, Other status change, Due date or priority,
  Issue created), each with an "In app" and a "macOS" box. Statuses go by category, not one row per status.
  Rejected: involvement and issue types as more matrix dimensions; per-status rows.
- **Which issues** are filters above the matrix, shared by both channels: all issues or only those the owner is
  involved in (assigned, reported, named in a people field), and the issue types left out (stored as excluded, so
  a new type counts).
- **"About you" always gets through**: the four events about the owner ignore the scope and type filters.
- **In app = unread**: a change that doesn't count is stored `quiet` when it is recorded, in the same
  transaction, and every unread read (`changes::UNSEEN`, `unseen_changes`, `inbox::count`) skips quiet changes.
  Saving the settings or another connected account settles every change again, so a change applies at once, also
  to what is already unread. Rejected: judging at read time in SQL (the event needs status categories, the
  custom values and the settings in every unread query). A quiet change stays in the activity; an issue marked
  unread by the owner stays unread.
- **People fields**: opted-in custom fields of a user type. Being added to one is recorded as a `people` change
  (field id and account id only).
- **Upgrade**: an old `notifyAbout` is read once. In-app stays as it was (all issues, every event); `nothing` turns
  the macOS events off, the other two give the "about you" events. `myIssues`' narrower scope isn't carried
  over, since the scope now applies to both channels.

## Consequences

- `issue_change` has a `quiet` column (`cache/0016`); `cache.field` keeps `items` to tell multi user pickers.
- macOS notifications follow the same events and filters; "Issue created" can notify when ticked.
- Ignoring changes made by automations needs the author of each change and comes later.
