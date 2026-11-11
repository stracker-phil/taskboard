# Writes

`src-tauri/src/writes/` changes Jira: it assigns issues (`assign.rs`), moves them along their workflow
(`transition.rs`), sets their priority (`priority.rs`), adds comments (`comment.rs`) and web links (`web_link.rs`) ([ADR 0044](adr/0044-writes-show-at-once-and-settle-after-jira.md)).
Every write is an allowlisted gateway endpoint with `Access::Write` ([jira.md](jira.md#gateway-and-allowlist)),
sent once and never retried.

## Turning it on

- The build: `jira/gateway/guard.rs` `WRITES_ENABLED`.
- The user: Settings → Connection → Changes → **Allow changes to Jira** (`behaviour.allowJiraChanges`, off by
  default; the debug build turns it on when it first connects to the fake Jira), then each kind of change on its own
  (`behaviour.disabledChanges`: Change the assignee, Change the status, Change the priority, Change the title, Add
  comments, Add web links). Off, a write fails as `invalid_input` ("Changes to Jira are off…" or "“Add comments” is switched off…"),
  the page hides that change's buttons, menu items and keys, and a drag onto groups it would change says why.
- The list is `jira::capability`: every gateway endpoint names its `Capability` (`Endpoint::capability`, an
  exhaustive match), so a new request can't exist without one, and `writes::ensure_allowed` checks the change's
  capability before anything is sent. `jira_capabilities` gives the page the list (the reads as information, the
  changes as switches); `jira_capability_set` switches a change.
- A scoped token needs `write:jira-work`. A `403` on a write is `invalid_input` naming that scope; it never
  pauses the sync. A `400` shows Jira's reason.
- Jira's own permissions: while changes are on, Settings → Connection asks `GET mypermissions` once per tracked
  project (`connection_change_permissions`, `connection::permissions`) for the project permission each change needs
  (`Capability::jira_permission`: Assign issues, Transition issues, Edit issues (priority and title), Add comments,
  Link issues; each asked once) and
  names, under a change, the projects where Jira doesn't allow it. Jira answers for the user, not the token, so a
  scoped token without `write:jira-work` still shows only when a change is refused.

## Flow (`writes::apply`)

1. In one `Db::write`: the issue row shows the new value and `cache.pending_write` records it with the value
   the row had (`pending::begin`); `issues_changed` refreshes the page at once. A second write to the same
   field while one is on its way is refused.
2. The write goes to Jira through `Source` (`assign`, `transition`, `set_priority`).
3. Refused: the row gets `base` back (`pending::roll_back`) and the error reaches the page, which toasts it.
4. Accepted: `sync::refetch_issue` fetches the issue with the sync's fields and stores it like a sync page;
   the pending row goes in the same transaction (`pending::settle`), and the issue counts as seen up to its new
   `updated` (`changes::seen_until_updated`; never backwards, not while marked unread). If that fetch fails the
   row goes anyway.

While a row is pending, `sync::store` keeps its value over what a sync page brings (`writes::overlay`) and
stores Jira's value as `base`. A page whose `updated` is older than the cached issue's is skipped. Own changes
record no `issue_change` (a pending comment counts in `comment_count`, also for the detail comment sync), and what
Jira changed along with them is seen, so they never show as unread. At launch `writes::forget_all` drops rows a quit cut off.

## Outcome

`WriteOutcome { done, reverse }`: `done` is the toast ("Assigned NW-1042 to you", "Moved NW-1042 to Done");
`reverse` is the opposite change and its label ("Change back to Jamie Doe", "Change back to Unassigned"),
which the toast offers as its action (`src/writes/runWrite.ts`). It is a second change in Jira, never an undo.
Assigning to the current assignee, or moving to the current status, sends nothing and offers no reverse.

## Status changes

Jira's workflow decides which statuses an issue can move to, so `issue_transitions(id)` asks Jira
(`GET issue/{id}/transitions`) through [Offers](#offers) and lists the moves by target status
(`TransitionOption { id, statusName, style, opensJira }`, in Jira's order). A move with `hasScreen` needs fields
Taskboard doesn't show: the menu marks it "Opens Jira" and opens the issue in the browser, and `apply` refuses it.
`apply` checks the move against the same offers before it writes, so a move Jira doesn't offer fails with a
reason, and one Jira dropped since it was asked is refused by Jira. The row
shows the target status (and its category) at once; the pending row holds the status id. The reverse is the move
back to the old status when Jira offers one without a screen ("Change back to In Progress"); otherwise none.

A priority is set by name to one the issue's edit screen offers (`GET issue/{id}/editmeta` through
[Offers](#offers), also checked before each write; no priority field there means it can't be changed). The pending row holds the name; the reverse sets
the old one ("Change back to Medium"), none when the issue had no priority.
## Offers

`writes::Offers` (in `AppState`, cleared when the connection is saved) keeps what Jira offers, so menus, drags and
the check before a write don't ask again:

- **Moves** per issue and the status it had when asked, for a minute: a workflow's conditions may look at the
  issue (its assignee, say). Another status asks again; so does any change Taskboard makes to the issue other than
  a move, and a refused one. A move asks for the moves from the new status for its reverse and keeps them.
- **Priorities** per kind of issue (project, issue type and status, from `cache.issue`), for ten minutes: issues of
  one kind share an edit screen. An issue the cache doesn't know is asked for itself, uncached.

## Drag and drop

A List row or a Board card dragged onto a group changes the field the view groups by (`writes/drop.rs`,
`src/writes/useIssueDrag.ts`). Rust decides what a group means:

| Grouped by | Places it may go | A drop |
|---|---|---|
| Status (a Board grouped by nothing shows status columns) | the statuses Jira's moves lead to, without moves that need a screen | the move into that status |
| Status type | the types of those statuses | the one move into a status of that type; several → a menu of those moves at the drop point |
| Assignee | me, the Space's active team (`team::active`) and Unassigned | assigns to that person, or unassigns |
| Priority | the priorities the issue's edit screen offers | sets that priority |
| anything else | none, with a note why | refused |

When a drag starts, `issue_drop_targets(id, groupBy, fromKey, groups)` asks Jira (`drop_targets`) and puts the
places in the view's order (`arrange`: the view's groups first, then places it has no group for, such as a teammate
without issues here; never the issue's own group). `DropTargets { targets, note }`: each `DropTarget` has the group
key, a label, the group mark and the action in words ("Set status to Code Review", "Assign to Jamie Doe", "Set
priority to High"). `issue_drop(id, groupBy, key)` checks again and writes through `apply`
(`DropOutcome::Done { outcome }`, or `Choose { moves }` for a status type several statuses fit).

While a drag lasts the view keeps its layout (a List stays a List, a Table a Table, Board columns keep their
width), and every group becomes an equally easy target:

- **The issue:** its row or card stays in its own group at a third of its opacity (`[data-drag-source]`), so it can
  go back; a copy of exactly that row or card follows the pointer where it was grabbed (`writes/draggedCopy.ts`,
  on the body with the custom properties the original inherits: the List's column template, the Table's column
  widths; a card tilts 2°). Over a zone that takes it, the copy shows what the drop would set: the status square and
  pill take the zone's style (and name, grouped by status), the priority flag the zone's priority.
- **List and Table** (`views/list/dragFold.ts`, `useDragFold.ts`, `useFoldMotion.ts`): the view folds. Every
  group, closed ones too, glides to the same height: its first two rows (the dragged one always among them, in its
  place) and a `more` piece that closes the card ("+N more"). Rows beyond them shrink and fade at the group's
  bottom edge, a closed group's rows grow in, and the groups below glide, like a group opening or closing
  (`--motion-slow`, instant under reduced motion). The dragged row keeps its place on screen while the view folds
  and unfolds: the view scrolls by as much as the row would move. After the drag the view unfolds the same way.
- **Board:** every column (and collapsed strip) glides to the board's full height and back (`useHeightGlide` in
  `lib/motion.ts`).
- **Zones** (`writes/DropZone.tsx`): a group's zone lies exactly over its card below the header (List: the card's
  corners, square under the tab), its rows (Table: square) or its column (Board: the column's corners). A place that
  takes the issue has a dashed outline and a light tint in the group's color (status style, avatar, priority flag).
  Under the pointer (`Drag.over`, followed in code: `:hover` doesn't update while a button is held) the outline turns
  solid, the tint darker, a halo grows around it and the action fades in as a pill ("Set status to Code Review").
  Where the issue can't go the group fades under a veil of the canvas; until Jira answers zones show a faint dashed
  outline. The issue's own group has no zone.
- **The tray** (`DropTray`): places without a group in the view, as chips with their action along the view's bottom
  edge, dashed, solid under the pointer.
- Near the view's edge (top and bottom in the List and Table, left and right on the Board) the view scrolls.

The drag is pointer-driven (the webview's file drops take the HTML drag events): it starts after 6 px, esc or a
release outside a zone cancels, and the click that ends it selects nothing. No drag starts while changes to Jira
are off, with a modifier key, or on a button in the row.

## Comments

`issue_comment(id, text)` (`writes::add_comment`) takes Markdown, at most `MAX_COMMENT_CHARS` (Jira's 32,767),
and turns it into ADF (`adf::from_markdown`, [ADR 0063](adr/0063-comments-are-written-in-markdown.md): emphasis,
strike-through, code, links, lists, quotes, headings, code blocks and rules; a line break is a hard break, bare
`http(s)://` addresses become links, raw HTML stays text). The comment shows in the activity at once as a `cache.comment` row with
the id `pending:<random>` by the connected account, and the issue's `comment_count` goes up, so the fetch that
brings Jira's comment records no change and the own comment isn't unread. Both comment replacements (the sync's
and `issue_sync_comments`) keep `pending:` rows. Jira accepts: its answer replaces the pending row
(`issues::store_comment`) in the transaction that stores the fetched issue. Jira refuses: the row and the count go
and the box keeps the text. There is no reverse; editing and deleting comments aren't built. The box sits above
the Issue view's activity (`CommentBox`): ⌘↩ sends, esc leaves it.

## Titles

`issue_write(id, Title { text })` (`writes::title`) renames the issue with `PUT issue/{id}` and the `summary` field
only. The text is trimmed and must be one line of 1 to 255 characters (`IssueTitle`); the pending row holds it
(field `title`, column `summary`), so the row, the Preview and search show it at once. The reverse sets the old title
("Change back to “…”"). The record of changes keeps its length, not the text ("Title (42 characters)"), like a
comment's.

In the page the Preview's and the Issue view's title is the editor (`EditableTitle`): a double-click, **F2**
(`issue.rename`) or Issue → Rename… makes the heading itself editable (`contentEditable="plaintext-only"`), so it
keeps its place and size; ↩ or clicking elsewhere saves, esc cancels without closing the Preview or the Issue view,
and leaving the app keeps the edit open. F2 on a List or Table row opens its Preview to edit there, on a Board card
the Issue view. The sent title shows until Jira's answer replaces it; a refusal puts the old one back.

## Several issues at once

`writes::bulk` serves the bulk bar ([ux.md](ux.md#picking-issues)). The menus ask side by side
(`jira::ask_all`, the rate limit's burst at a time) through [Offers](#offers): the moves of every picked issue,
the priorities of one issue per kind (`offers::one_per_kind`). They keep what all of them allow: a status by name,
since workflows differ between projects, leaving out moves that need Jira's own screen and issues already in it; the
priorities in the first issue's order; me and the people on every picked issue's Space's team. `apply_many` sends the
changes side by side (`jira::run_all`) through `apply`, so each is checked against the switches, recorded and settled like a
single change, and a refusal doesn't stop the rest. Its `BulkOutcome` words the toast, counts what changed and what
Jira refused, and carries each changed issue's reverse for **Change back**, itself a bulk change.

## Web links

`issue_add_web_link(id, url, title)` (`writes::add_web_link`) adds a web link (Jira's remote link: a pull request,
a document) with `POST issue/{id}/remotelink`. The address must be http or https (`WebUrl`, at most 2000
characters); without a title the link is named after its pull request (`repo#481`, `issues::classify`) or its
address. Jira answers the link's id; the link joins the cached web links at once (`cache.remote_link`) and the next
fetch of the issue's links replaces them as usual. There is no reverse and no removing. In the Issue view the Web
links section shows while changes to Jira are on, even without links, with an "Add link" button that grows a form
open (`AddWebLink.tsx`): the address, focused, and an optional title; ↩ adds, esc closes.

## Record of changes

Every change Jira is asked for is recorded with Jira's answer in `local.write_log` (`writes::sent`): when, the issue
key, what in plain words ("Status: Done", "Assignee: Jamie Doe", "Comment (42 characters)", "Web link: …") and the
refusal, if any; a comment's text is not kept. Field changes record in `send`, comments and web links after their
request. Records are kept for 180 days. Settings → Status lists the newest 50 ("Changes Sent to Jira",
`diagnostics::sent`); Export… (`app:exportChanges`) saves all of them as a CSV in Downloads and shows it in Finder.
A change refused before it is sent (changes off, a switched-off kind) records nothing.

## Commands

| Command | Does |
|---|---|
| `issue_write(id, change)` | `Change::Assign { accountId }` (null unassigns) |
| `issue_toggle_mine(id)` | assigns to me, or unassigns when it is mine |
| `issue_assignees(id)` | who the issue may be assigned to, and who has it |
| `issue_write(id, Transition { transitionId })` | moves the issue along a transition from `issue_transitions` |
| `issue_write(id, Priority { name })` | sets the priority to one the edit screen offers |
| `issue_write(id, Title { text })` | renames the issue |
| `issue_transitions(id)` | the moves Jira offers from the issue's status |
| `issues_bulk_statuses(ids)` | the statuses every picked issue can move to (or is in), with each issue's move |
| `issues_bulk_priorities(ids)` | the priorities every picked issue's edit screen offers |
| `issues_bulk_assignees(ids)` | me and whoever is on the team of every picked issue's Space |
| `issues_write_many(changes, what, total)` | sends the changes side by side through `apply`; one outcome for the toast, with the reverses |
| `issue_priorities(id)` | the priorities the issue's edit screen offers, and the one it has |
| `issue_comment(id, text)` | adds a plain-text comment |
| `issue_add_web_link(id, url, title)` | adds a web link; `title` null names it after its pull request or address |
| `issue_drop_targets(id, groupBy, fromKey, groups)` | the places a dragged issue may go, each with its action, in the view's order |
| `issue_drop(id, groupBy, key)` | moves the issue into a group of the view (a drop) |

In the page: the row hover toolbar's person button, the Issue view's and the Preview's header button (pressed
while it is mine), the key **I** (`issue.toggleMine`) and Issue → Assign to Me or Unassign (also in the palette).
The status pill in the Preview and the Issue view's field grid is the status menu's button (`StatusMenuButton`);
**D** (`issue.changeStatus`, Jira's own key) and Issue → Change Status… open the same menu under the pill, or under the selected
row or card. Rows and cards drag onto tiles of their groups (above).
The Issue view's assignee is the assignee menu's button (`AssigneeMenuButton`); **A** (`issue.assignTo`) and
Issue → Assign To… open it under the assignee, or under the selected row or card. `issue_assignees(id)`
(`writes::assignees`) lists me and the issue's Space's active team (`team::active`, me always first), and who has it now; the
menu checks the current one and ends with Unassigned.
The priority in the Issue view's field grid and the Preview is the priority menu's button (`PriorityMenuButton`);
**⇧P** (`issue.changePriority`) and Issue → Change Priority… open it under the priority, or under the selected row or
card. `issue_priorities(id)` (`writes::priorities`) asks the edit screen each time; the menu checks the current one.
In the List and the Table the Status, Assignee and Priority cells open the same three menus under the cell, and
select the row without opening it (`views/list/cellEdits.tsx`); they shade the value on hover. Each opens only while its kind of change is on.
