# Writes

`src-tauri/src/writes/` changes Jira: it assigns issues (`assign.rs`), moves them along their workflow
(`transition.rs`), sets their priority (`priority.rs`) and adds comments (`comment.rs`) ([ADR 0044](adr/0044-writes-show-at-once-and-settle-after-jira.md)).
Every write is an allowlisted gateway endpoint with `Access::Write` ([jira.md](jira.md#gateway-and-allowlist)),
sent once and never retried.

## Turning it on

- The build: `jira/gateway/guard.rs` `WRITES_ENABLED`.
- The user: Settings → Connection → Changes → **Allow changes to Jira** (`behaviour.allowJiraChanges`, off by
  default; the debug build turns it on when it first connects to the fake Jira). Off, a write fails as
  `invalid_input` ("Changes to Jira are off…"), the row toolbar and the headers hide their write buttons, and
  the I key shows that hint.
- A scoped token needs `write:jira-work`. A `403` on a write is `invalid_input` naming that scope; it never
  pauses the sync. A `400` shows Jira's reason.

## Flow (`writes::apply`)

1. In one `Db::write`: the issue row shows the new value and `cache.pending_write` records it with the value
   the row had (`pending::begin`); `issues_changed` refreshes the page at once. A second write to the same
   field while one is on its way is refused.
2. The write goes to Jira through `Source` (`assign`, `transition`, `set_priority`).
3. Refused: the row gets `base` back (`pending::roll_back`) and the error reaches the page, which toasts it.
4. Accepted: `sync::refetch_issue` fetches the issue with the sync's fields and stores it like a sync page;
   the pending row goes in the same transaction (`pending::settle`). If that fetch fails the row goes anyway.

While a row is pending, `sync::store` keeps its value over what a sync page brings (`writes::overlay`) and
stores Jira's value as `base`. A page whose `updated` is older than the cached issue's is skipped. Own changes
record no `issue_change`, so they never show in Unread. At launch `writes::forget_all` drops rows a quit cut off.

## Outcome

`WriteOutcome { done, reverse }`: `done` is the toast ("Assigned NW-1042 to you", "Moved NW-1042 to Done");
`reverse` is the opposite change and its label ("Change back to Jamie Doe", "Change back to Unassigned"),
which the toast offers as its action (`src/writes/runWrite.ts`). It is a second change in Jira, never an undo.
Assigning to the current assignee, or moving to the current status, sends nothing and offers no reverse.

## Status changes

Jira's workflow decides which statuses an issue can move to, so `issue_transitions(id)` asks Jira each time the
status menu opens (`GET issue/{id}/transitions`) and lists the moves by target status
(`TransitionOption { id, statusName, style, opensJira }`, in Jira's order). A move with `hasScreen` needs fields
Taskboard doesn't show: the menu marks it "Opens Jira" and opens the issue in the browser, and `apply` refuses it.
`apply` asks for the moves again before it writes, so a move Jira no longer offers fails with a reason. The row
shows the target status (and its category) at once; the pending row holds the status id. The reverse is the move
back to the old status when Jira offers one without a screen ("Change back to In Progress"); otherwise none.

A priority is set by name to one the issue's edit screen offers (`GET issue/{id}/editmeta`, asked again before
each write; no priority field there means it can't be changed). The pending row holds the name; the reverse sets
the old one ("Change back to Medium"), none when the issue had no priority.
## Drag and drop

A List row or a Board card dragged onto a group changes the field the view groups by (`writes/drop.rs`,
`src/writes/useIssueDrag.ts`). Rust decides what a group means:

| Grouped by | Places it may go | A drop |
|---|---|---|
| Status (a Board grouped by nothing shows status columns) | the statuses Jira's moves lead to, without moves that need a screen | the move into that status |
| Status type | the types of those statuses | the one move into a status of that type; several → a menu of those moves at the drop point |
| Assignee | the Space's active team (`team::active`, me first) and Unassigned | assigns to that person, or unassigns |
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
- **List and Table** (`views/list/DragList.tsx`, `dragGroups.ts`): the rows give way to every group, closed ones
  too, with its header and its first two rows (the dragged one always among them) and "+N more", so each group's
  card is the same height. The view scrolls to the dragged row's group and back to where it was after.
- **Board:** every column (and collapsed strip) reaches the board's full height.
- **Zones** (`writes/DropZone.tsx`): a group's zone is its own card (List), its block of rows (Table) or its column
  (Board), with the same corners: an inner 2px ring and a light tint in the group's color (status style, avatar,
  priority flag) with the action in a pill. Under the pointer the tint deepens and the card or column glows in its
  color (the List's card lifts a little). Where the issue can't go the group fades; until Jira answers zones show a
  dashed ring. The issue's own group has no zone.
- **The tray** (`DropTray`): places without a group in the view, as chips along the view's bottom edge.
- Near the view's edge (top and bottom in the List and Table, left and right on the Board) the view scrolls.

The drag is pointer-driven (the webview's file drops take the HTML drag events): it starts after 6 px, esc or a
release outside a zone cancels, and the click that ends it selects nothing. No drag starts while changes to Jira
are off, with a modifier key, or on a button in the row.

## Comments

`issue_comment(id, text)` (`writes::add_comment`) takes plain text, at most `MAX_COMMENT_CHARS` (Jira's 32,767),
and turns it into ADF (`adf::from_plain_text`: a blank line starts a paragraph, a line break is a hard break,
`http(s)://` addresses become links). The comment shows in the activity at once as a `cache.comment` row with
the id `pending:<random>` by the connected account, and the issue's `comment_count` goes up, so the fetch that
brings Jira's comment records no change and the own comment isn't unread. Both comment replacements (the sync's
and `issue_sync_comments`) keep `pending:` rows. Jira accepts: its answer replaces the pending row
(`issues::store_comment`) in the transaction that stores the fetched issue. Jira refuses: the row and the count go
and the box keeps the text. There is no reverse; editing and deleting comments aren't built. The box sits above
the Issue view's activity (`CommentBox`): ⌘↩ sends, esc leaves it.

## Commands

| Command | Does |
|---|---|
| `issue_write(id, change)` | `Change::Assign { accountId }` (null unassigns) |
| `issue_toggle_mine(id)` | assigns to me, or unassigns when it is mine |
| `issue_assignees(id)` | who the issue may be assigned to, and who has it |
| `issue_write(id, Transition { transitionId })` | moves the issue along a transition from `issue_transitions` |
| `issue_write(id, Priority { name })` | sets the priority to one the edit screen offers |
| `issue_transitions(id)` | the moves Jira offers from the issue's status |
| `issue_comment(id, text)` | adds a plain-text comment |
| `issue_drop_targets(id, groupBy, fromKey, groups)` | the places a dragged issue may go, each with its action, in the view's order |
| `issue_drop(id, groupBy, key)` | moves the issue into a group of the view (a drop) |

In the page: the row hover toolbar's person button, the Issue view's and the Preview's header button (pressed
while it is mine), the key **I** (`issue.toggleMine`) and Issue → Assign to Me or Unassign (also in the palette).
The status pill in the Preview and the Issue view's field grid is the status menu's button (`StatusMenuButton`);
**S** (`issue.changeStatus`) and Issue → Change Status… open the same menu under the pill, or under the selected
row or card. Rows and cards drag onto tiles of their groups (above).
The Issue view's assignee is the assignee menu's button (`AssigneeMenuButton`); **A** (`issue.assignTo`) and
Issue → Assign To… open it under the assignee, or under the selected row or card. `issue_assignees(id)`
(`writes::assignees`) lists the issue's Space's active team, me first (`team::active`), and who has it now; the
menu checks the current one and ends with Unassigned.
