# 0055. Status styles are chosen app-wide; a Space overrides them

## Status

Accepted (2026-11-10). Owner's request after several weeks of use. Refines
[0030](0030-a-space-carries-its-own-settings.md).

## Context

0030 moved status styles into each Space. Most statuses are shared by several projects and mean the same in
all of them, so the owner styled the same status again in every Space, and a new Space started with none of
those choices.

## Decision

- `status_style` (migration `local/0026`) holds the owner's style per status for the whole app;
  `space_status_style` only overrides it in one Space. The migration lifts a status every overriding Space
  styled alike to `status_style` and deletes those Space rows; conflicting choices stay per Space.
- `status_style::Styles::of` resolves a style as the Space's choice, else the app's, else `detect`; `app_of`
  (a linked issue the cache doesn't hold) skips the Space. `filter::STATUS_CATEGORY` follows the same order, so
  filters and the sidebar's open counts match what the rows show.
- Settings → Statuses lists every status the tracked projects use (`status_styles_all`,
  `status_style_set_app`). A Space's Statuses section keeps its counts; its first menu item is "Default (the
  app style)". Both group the statuses by the style they show, and a changed status glides into its new group.

## Consequences

- One choice styles a status everywhere; a Space that means something else by a status still can.
- Supersedes 0030's consequence "No app-wide Statuses page".
