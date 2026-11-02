# 0052. Back and Forward live in each tab's nav

## Status

Accepted (2026-11-02). Owner's report. Extends [ADR 0050](0050-tabs-in-the-main-window.md).

## Context

Leaving a place lost it: open an issue, follow a bookmark, then a breadcrumb, and the way back to the issue was
gone unless its key was remembered. The Issue view is page state (`issueView`), while places and tabs are Rust's
(`NavStore`, `nav.json`).

## Decision

- Each saved tab keeps its history (`nav/history.rs`): Back and Forward stacks of steps, a step being a place and
  the issue its Issue view showed there, at most 100 per direction, saved with the tab in `nav.json`.
- A new place (`NavStore::go`: the sidebar, links, the CLI, previous or next Space or list) and another issue in
  the Issue view are steps. The page reports the shown issue (`nav_set_issue`) whenever it changes; Rust notes a
  step only when it differs. Showing a tab doesn't step.
- `nav_back` and `nav_forward` move through the active tab's steps, skipping ones whose level is gone, and return
  the `Nav` with the issue to show there; the page opens or closes the Issue view to match. Going a new way clears
  Forward.
- The page commands `history.back` (⌘[) and `history.forward` (⌘]) run them from the keys, the Go menu and the
  arrows beside the traffic lights.

## Consequences

- Closing a tab drops its history; a reopened or new tab starts without one.
- The preview, a view's scroll and selection aren't steps; Back shows the place as its view keeps it.
