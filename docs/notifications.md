# Notifications

`src-tauri/src/notify/` shows a macOS notification when a change sync brings changes that concern the owner.

## When

- After a **change sync** (delta) finishes, while the main window **doesn't have the focus** (in front, the
  unread rows already show them). A full sync, a reconciliation or a launch announces nothing; their changes are passed by.
- `Notifier` keeps the newest `cache.issue_change` row it has looked at (`rowid`) in memory; it starts after the
  rows cached at launch.
- The owner's own changes never notify. Taskboard's writes are seen up to Jira's answer ([writes.md](writes.md)). The
  sync records no `created` for an issue the owner reported, no `comment_added` or `mention` for the owner's own
  comments, and drops field changes Jira's changelog says the owner made ([sync.md](sync.md#own-changes)), all
  before the notifier looks.

## What (`notify::notices`)

Settings → Behavior → **Notify me about** (`behaviour.notifyAbout`):

| Value | Announces |
|---|---|
| `nothing` | nothing |
| `mentionsAndAssignments` (default) | issues with a `mention` of the connected account or an `assignee` change to it |
| `myIssues` | also any recorded change of an issue assigned to or reported by the connected account |

`created` and `gone` rows never notify. One notification per issue: the title is the key and summary, the body
the Since-yesterday sentence of the new rows (`changes::sentence`, "Mentioned you, 1 new comment."). More than
`MAX_SINGLE` (3) issues in one sync make one notification "5 issues changed" listing the keys. Nothing is
announced without a connected account (Test Connection).

## Buttons (`notify::show`)

An issue's notification has **Open** (`issue:open:<KEY>`: shows the window with the issue in the full Issue
view, `remote::open_issue`); the summary has **Show Unread** (`unread:show`: turns Unread only on and goes to Everything). Both are bus commands
(`notify/commands.rs`). macOS asks once whether Taskboard may notify; the debug build asks for itself.
