# 0044. Writes show at once and settle after Jira

## Status

Accepted (2026-10-31). Owner's go-ahead for M4. Builds on [0025](0025-all-jira-traffic-goes-through-one-allowlisted-gateway.md)
and [0029](0029-issues-come-from-a-source-jira-is-the-first.md).

## Context

Taskboard was read-only. M4 adds a few changes (assign, then transitions and comments). The spec asks for an
optimistic update, a rollback on error, writes that are never retried, and no flicker when the next sync comes:
a delta that fetched the issue before the write would otherwise show the old value again for a moment.

## Decision

- **Two locks.** `jira/gateway/guard.rs` `WRITES_ENABLED` decides whether the build may write at all; each
  write is a rule with `Access::Write` in the allowlist, sent once (`Retry::Never`). The setting
  `behaviour.allowJiraChanges` ("Allow changes to Jira", Settings → Connection) decides per user and is off by
  default; the debug build turns it on when it first connects to the fake Jira.
- **Shown at once, held until settled.** `writes::apply` sets the field in `cache.issue` and records a
  `cache.pending_write` row in one transaction, then sends the write. While the row exists, the sync stores
  what Jira sent but keeps the pending value over it (`writes::overlay`) and remembers Jira's in `base`.
- **Settled by fetching the issue again.** After Jira accepts, the issue is fetched (`sync::refetch_issue`) and
  stored like a sync page, and the row goes in the same transaction. A refusal puts `base` back. A page older
  than the cached issue (`updated`) is never stored, so a page fetched before the write can't win afterwards.
- **Own changes aren't unread.** The cached row already holds the written value when Jira's arrives, so the
  sync records no change.
- **No undo.** Reversing a write is a second write in Jira's history. The outcome offers it by what it sets:
  "Change back to Jamie Doe", "Change back to Unassigned" (owner, 2026-10-31).
- Rejected: queueing writes on the sync task (a full sync would delay them for minutes); comparing `updated`
  with the write's time instead of a pending row (Jira's clock and the Mac's differ).

## Consequences

- A write that a quit cuts off can't settle: `writes::forget_all` drops its row at the next launch and the
  issue keeps what it shows until the sync brings a newer version.
- A field can have one write on its way at a time; a second one is refused until the first settles.
