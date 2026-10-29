# 0030. A Space carries its own settings

## Status

Accepted (2026-10-29). Owner's request, modelled on ClickUp's Space settings.

## Context

Status styles were app-wide and every Space offered every layout and custom field. Projects differ: one
names a status "Blocked" for a different meaning, one has a dozen long text fields that only belong in the
Issue view, one is only ever read as a board.

## Decision

- Settings that change how a Space reads live with the Space, in the local database (migration `local/0007`):
  the layouts its views offer and the layout and grouping of its new views (`space` columns), the style of each
  status (`space_status_style`, replacing the app-wide `status_style`, whose rows were copied to every Space),
  and where each custom field shows (`space_field`: visible, hidden in list, hidden in detail, hidden).
- `space_config` reads and writes them; the views (`views::edit::read_view`), the column catalogue and the
  Issue view apply them, so the page never decides. A status shows in its issue's own Space's style
  (`status_style::Styles::of`). Outside the Spaces (Everything, Home's lists) every layout is offered and a
  field is hidden as a column only when every Space hides it.
- Settings → Spaces opens a page per Space; the sidebar's Space menu has Space Settings…

## Consequences

- No app-wide Statuses page; a status used by several Spaces is styled in each.
- Switching a layout off never deletes views: they show the default layout until it is on again.
