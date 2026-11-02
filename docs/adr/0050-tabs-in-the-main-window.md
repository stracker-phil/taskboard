# 0050. Tabs in the main window, owned by Rust's NavStore

## Status

Accepted (2026-11-02). Owner's request. Amends [ADR 0047](0047-the-bookmarks-bar-is-the-only-top-bar.md): the tab
strip is a second top bar, shown only while two or more tabs are open.

## Context

The owner wants several places open at once ("open view in new tab", "open issue in new tab"), as in a browser,
without a second window. The window's place is Rust's (`NavStore`, `nav.json`); menus, the command line, links and
Unread's read-here follow it.

## Decision

- **Rust owns the tabs.** `nav.json` keeps the open tabs (`tabs`, `active_tab`) beside the place; the place stays the
  active tab's, so everything that follows the place follows the active tab without a change. A file from before
  tabs is one tab. Closed tabs' places are kept (the last 10) for Reopen Closed Tab.
- **One tab looks like no tabs.** The strip shows from the second tab, in a slot that grows from 0 so the panel
  glides, in the bookmarks bar's look.
- **Per tab:** the place, the Issue view and its stack (parked by the page when the tab is left, brought back when
  it is shown, before the frame paints), and the drafts and text filter of the view it shows (kept for every open
  tab's view). A view's scroll, selection and preview stay per view: two tabs on one view share them.
- **Keys follow the Mac.** ⌘⇧T New Tab (⌘T stays New View), ⌘⇧[ / ⌘⇧] Previous / Next Tab (⌃⇥ stays the
  view tabs'), ⌘⌥T Reopen Closed Tab, all in the Go menu. ⌘W closes the active tab while there are more than one,
  else hides the window ([ADR 0016](0016-closing-the-window-keeps-the-app-running.md)).
- **Opening:** "Open in New Tab" in the menus of Spaces, filter lists, Home rows, view tabs and list or view
  bookmarks; ⌘-click on them opens the tab behind the shown one, ⌘⇧-click shows it. ⌘-click on rows and cards
  keeps picking issues.

## Consequences

- The command line's `go` and links move the active tab.
- Switching tabs counts as moving: Unread's read-here starts over, as it does on any move.
