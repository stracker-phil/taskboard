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
| `ignoreAutomations` | changes by app accounts (Xray, CI, Jira Automation) count only when they are about the owner ([ADR 0069](adr/0069-changes-by-automations-can-be-ignored.md)) | on (an old file without it reads on) |

The scope, involvement, types and the automation switch are shared by both channels. Saved only through `notifications_save`, which
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

`counts(settings, channel, event, subject, by_automation)`: the event must be ticked for the channel. The four
"about you" events (`NotifyEvent::is_about_me`) then always count, also when an app made them; the others skip,
with `ignoreAutomations`, a change an app account made (`Judged.by_automation`: the row's `author_id` is a
`cache.user` with `account_type` `app`; an unknown author or type is not an automation), an excluded type and,
with `involved`, an issue the owner isn't involved in (`Subject`: assigned to or reported by the connected
account, or named in a people field). A change that has no event never counts.

## Authors

Each `issue_change` row keeps who made it in `author_id` (NULL: unknown, which counts):

| Row | Author |
|---|---|
| `created` | the reporter |
| `comment_added` | the one account that wrote every new comment the search brought, when they cover all that were added; else unknown |
| `mention` | the comment's author |
| `status`, `assignee`, `priority`, `due_date`, `people` | the newest changelog entry for that field (a people row by the field id in `old_value`) at or before the change |
| `updated` | the one account behind every changelog entry since `seen_at`; else unknown |

The field rows read the changelog the change sync's search embeds (`expand=changelog`, `sync::authors`), but only
when it is complete (`EmbeddedChangelog::is_complete`). Otherwise, and for the full sync and reconciliation, which
don't ask for it, the post-pass after the sync (`sync::attribute`) fetches the changelog of the 50 most recently
updated issues with an unattributed field row, names the authors, and settles those rows again in the same
transaction. People seen as changelog authors are stored like any other (`sync::users::upsert_user` keeps a
known `account_type` when an answer leaves it out). Rows recorded before the launch's first change sync, rows of
issues past the 50, and rows whose changelog entry is missing stay unknown, so they count: the accepted
after-launch gap.

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
  after `store_page`, in the same transaction, deciding the rows still `NULL`. A row not settled yet counts. The
  authors' post-pass resets `quiet` of the rows it attributes and settles them again.
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
  comments, and drops field changes Jira's changelog says the owner made ([sync.md](sync.md#who-made-a-change)),
  all before the notifier looks. The post-pass runs inside the sync job, so the notifier judges the final authors.

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
