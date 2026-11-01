# 0046. Unread is a mode of every view

## Status

Accepted (2026-11-01). Owner's request. Replaces the Unread list of `local/0002` (kind `recent`).

## Context

Unread was a Home list that showed every unread issue in one place. The owner reads news per Space and per
list. One flat list didn't say where the news was, and a list's own views couldn't show only its unread issues.

## Decision

- **One app-wide switch.** "Unread only" (`modes.json` `unreadOnly`, `store/modes.rs`) narrows every view to its
  unread issues: `views::Narrowing.unread`, set by Rust from the mode, adds `changes::UNSEEN`, plus the issues read
  since the owner came to this place (`NavStore::read_here`). The page never sends it.
- **The sidebar shows where.** While it is on, every count is an unread count, open or done
  (`lists::sidebar_tree(…, unread)`), after a dot. Lists without unread issues show no number.
- **The toolbar switches it.** It is a bell button between Filter and Me (⇧⌘U, `toolbar.unread`), the same in
  every view because it is one mode. Mark all as read moves to the header of any level while it is on.
- **The Unread list goes.** `local/0018` deletes it with its views and bookmarks. The notification's
  "Show Unread" turns the mode on and opens Everything.
- Rejected: a scope per view or per list. Sidebar counts are per list, so a per-view mode can't say what a
  count means, and a mode per list would leave the sidebar half in one mode and half in the other.

## Consequences

- Turning it on changes every view at once, which is the point. The bell's color says it is on everywhere.
- The CLI keyword `recent` is gone: `taskboard go everything` and the bell do the same.
