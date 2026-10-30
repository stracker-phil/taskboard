# Writes

`src-tauri/src/writes/` changes Jira: today it assigns issues ([ADR 0044](adr/0044-writes-show-at-once-and-settle-after-jira.md)).
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
2. The write goes to Jira through `Source` (`assign`).
3. Refused: the row gets `base` back (`pending::roll_back`) and the error reaches the page, which toasts it.
4. Accepted: `sync::refetch_issue` fetches the issue with the sync's fields and stores it like a sync page;
   the pending row goes in the same transaction (`pending::settle`). If that fetch fails the row goes anyway.

While a row is pending, `sync::store` keeps its value over what a sync page brings (`writes::overlay`) and
stores Jira's value as `base`. A page whose `updated` is older than the cached issue's is skipped. Own changes
record no `issue_change`, so they never show in Unread. At launch `writes::forget_all` drops rows a quit cut off.

## Outcome

`WriteOutcome { done, reverse }`: `done` is the toast ("Assigned NW-1042 to you", "Unassigned NW-1042");
`reverse` is the opposite change and its label ("Change back to Jamie Doe", "Change back to Unassigned"),
which the toast offers as its action (`src/writes/runWrite.ts`). It is a second change in Jira, never an undo.
Assigning to the current assignee sends nothing and offers no reverse.

## Commands

| Command | Does |
|---|---|
| `issue_write(id, change)` | `Change::Assign { accountId }` (null unassigns) |
| `issue_toggle_mine(id)` | assigns to me, or unassigns when it is mine |

In the page: the row hover toolbar's person button, the Issue view's and the Preview's header button (pressed
while it is mine), the key **I** (`issue.toggleMine`) and Issue → Assign to Me or Unassign (also in the palette).
