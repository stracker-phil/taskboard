# Notifications

Settings → Notifications decides which recorded changes count, per channel ([ADR
0068](adr/0068-notification-settings-decide-what-counts.md)): **In app** makes an issue unread (the inbox and the
bell), **macOS** shows a notification. `src-tauri/src/notify/` holds both: `rules.rs` judges a change, `notices`
announces.

## Settings (`notifications`, `store/settings/notifications.rs`)

| Key | Holds | Default |
|---|---|---|
| `scope` | `allIssues`, or `involved`: only issues the owner is involved in | `allIssues` |
| `involvement` | what makes the owner involved: `assigned`, `reported`, `peopleFields` (named in a people field) | all on |
| `excludedTypes` | issue types (by name) whose changes count nowhere; a type added later counts | none |
| `inApp` | the events that make an issue unread | every event |
| `mac` | the events a macOS notification announces | `mentionsMe`, `assignsMe`, `addsMeToPeopleField` |

The scope, involvement and types are shared by both channels. Saved only through `notifications_save`, which
tidies the types like `sync.excludedTypes` and keeps the events in the page's order (`Notifications::tidied`).
`notification_issue_types` offers the cached issues' types plus the excluded ones; `notification_people_fields`
lists the people fields (`PeopleField { id, name }`).

An old file's `behaviour.notifyAbout` is read once (`old_notify_about`, only while the file has no
`notifications`) and never written again (`Notifications::from_notify_about`): in app stays as it was; `nothing`
leaves `mac` empty, `mentionsAndAssignments` and `myIssues` give the default `mac` events.

## Events (`notify::rules::event_of`)

Each `cache.issue_change` row is one `NotifyEvent`, or none:

| Row | Event |
|---|---|
| `mention` of the connected account | `mentionsMe` (a mention of someone else: none) |
| `assignee` to / from the connected account | `assignsMe` / `unassignsMe` |
| `people` naming the connected account | `addsMeToPeopleField` |
| `comment_added` | `newComment` |
| `status` into Done from another category | `movedToDone` |
| `status` into To Do from another category | `reopened` |
| any other `status`, `updated` and fields not listed | `otherChange` |
| `due_date`, `priority` | `dueOrPriority` |
| `created` | `created` |
| `gone` | none |

`counts(settings, channel, event, subject)`: the event must be ticked for the channel. The four "about you" events
(`NotifyEvent::is_about_me`) then always count; the others skip an excluded type and, with `involved`, an issue
the owner isn't involved in (`Subject`: assigned to or reported by the connected account, or named in a people
field). A change that has no event never counts.

## People fields (`notify::people`)

The opted-in custom fields (`field_choice`) of a user type: `cache.field.type` `user`, or `array` with `items`
`user` (single and multi user pickers). `named_in` reads an issue's `custom` values for those fields only, so only
`accountId` values of user fields match.

## In app (`notify::rules::settle`)

A change that doesn't count in app is stored `quiet` (`issue_change.quiet = 1`); `changes::UNSEEN`,
`unseen_changes`, `inbox::count` and the inbox's snapshot read only changes with `COALESCE(quiet, 0) = 0`. A quiet
change stays in the issue's activity and its Since-yesterday banner; it just never makes the issue unread. An
issue the owner marked unread (`issue_seen.unread`) stays unread regardless.

- **When changes are recorded**: the delta, full and reconciliation syncs call `settle(…, again = false)` right
  after `store_page`, in the same transaction, deciding the rows still `NULL`. A row not settled yet counts.
- **Again** (`resettle`, every row): on `notifications_save`, and when the connected account changes
  (`connection_save`, `connection_test`, the full sync's `/myself`). An issue whose unread state flips is
  announced as changed, so the bell and lists follow at once.

## When

- After a **change sync** (delta) finishes, while the main window **doesn't have the focus** (in front, the
  bell's badge already shows them). A full sync, a reconciliation or a launch announces nothing; their changes are passed by.
- `Notifier` keeps the newest `cache.issue_change` row it has looked at (`rowid`) in memory; it starts after the
  rows cached at launch.
- The owner's own changes never notify. Taskboard's writes are seen up to Jira's answer ([writes.md](writes.md)). The
  sync records no `created` for an issue the owner reported, no `comment_added` or `mention` for the owner's own
  comments, and drops field changes Jira's changelog says the owner made ([sync.md](sync.md#own-changes)), all
  before the notifier looks.

## macOS (`notify::notices`)

The new rows whose event counts for `mac` (`rules::counts` with `Channel::Mac`); an empty `mac` list announces
nothing. One notification per live issue: the title is the key and summary, the body the Since-yesterday sentence
of its counting rows (`changes::sentence`, "Mentioned you, 1 new comment.", "Added you as Developer."). More than
`MAX_SINGLE` (3) issues in one sync make one notification "5 issues changed" listing the keys. Nothing is
announced without a connected account (Test Connection).

## Buttons (`notify::show`)

An issue's notification has **Open** (`issue:open:<KEY>`: shows the window with the issue in the full Issue
view, `remote::open_issue`); the summary has **Show Notifications** (`inbox:show`: shows the window with the inbox open, [ux.md](ux.md#inbox)). Both are bus commands
(`notify/commands.rs`). macOS asks once whether Taskboard may notify; the debug build asks for itself.
