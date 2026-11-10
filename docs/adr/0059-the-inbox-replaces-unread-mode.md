# 0059. The inbox replaces Unread mode

## Status

Accepted (2026-11-10). Owner's request after several weeks of use. Supersedes
[0046](0046-unread-is-a-mode-of-every-view.md).

## Context

Unread only filtered every view and the sidebar counts, tinted changed rows and kept read issues listed until the
owner left the place (`nav::NavStore.read_here`). In use it confused: an issue marked read stayed after turning the
mode off, and the markers were everywhere.

## Decision

- One place lists unread issues: the inbox, a dropdown under a bell in the content panel's top-right corner, on
  every page (`src/inbox/`). The bell's badge counts unread tracked issues (`inbox::count`).
- Opening it snapshots the unread issues in Rust (`inbox_open`, `InboxSession` in `AppState`) with each one's note
  (`changes::sentence`). While open, `inbox_rows` lists the snapshot with the current read state: a read row stays,
  dimmed, in place. Closing forgets the snapshot; the next open lists what is unread then.
- Its keys are commands of an `inbox` scope that shadows `main` while open (J/K, ↑/↓, M, ↵, ⌘↵, ⇧⌘A, ⎋).
  ⇧⌘U toggles it; the Go menu, the palette and a notification's "Show Notifications" (`inbox:show`) open it.
- Gone: `modes.unreadOnly`, the unread narrowing, `read_here`, unread sidebar counts, row tints and change chips,
  the toolbar's Unread button and the list's Mark all as read. An issue's own "Since yesterday" line, its activity
  divider and the M key stay: they describe the issue, not a list.

## Consequences

- Views and counts no longer depend on read state, so reading never moves a row.
- The inbox is one more overlay with its own keys; a child scope keeps them from clashing with the list's.
