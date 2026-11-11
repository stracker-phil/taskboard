# 0060. A status bar under the content panel

## Status

Accepted (2026-11-11). Owner's request. Amends the Window rule in `docs/ux.md` ("There is no icon rail or status
bar").

## Context

The sidebar footer's sync chip had about 100px for its text, so "Synced 2 min ago" showed as "Synced 2…". It also
left with the sidebar. And the app said nothing while it talked to Jira outside a sync: loading the statuses an
issue can move to, saving a status or a priority, sending a comment. The owner also wants to see when the Jira
data lives in memory only (ADR 0057), and when the last sync was, readably.

These are facts about the whole app, not about the view on screen, and they must stay visible with the sidebar
hidden and in the Issue view.

## Decision

- **A 32px row on the window background under the content panel**, level with the sidebar footer. The panel's
  bottom gap becomes the bar, so the view gives up 24px. The empty bar drags the window.
- **App facts only, at its right end:** what Taskboard is asking Jira, the memory-mode chip, then the sync state
  in words with its dot (a click syncs now). Their details are tooltips: the last sync and full sync in full,
  what the memory holds.
- **View facts stay with the view.** The number of issues a view shows sits on its view line, beside the
  controls that change it, not in the bar.
- **The sidebar toggle lives in the row:** the sidebar footer's first item, or the bar's while the sidebar is
  hidden. It no longer floats over the panel, so no view keeps room clear for it. Settings stays in the sidebar
  footer.

## Consequences

- Toasts and the bulk bar's toasts rise 24px higher, clear of the bar.
- The footer's items shrink from 34px to 26px to fit the row.
