# 0038. Filter changes are drafts until the view is saved

## Status

Accepted (2026-10-29). Owner's request. Supersedes in part [0033](0033-views-can-hold-their-own-filter.md) (the
Filter popover saving as you edit; the Me chip and the text field kept with the view's place) and the line of
[0024](0024-filters-run-locally-in-sql-views-never-filter.md) that the Me chip is never saved. Keeps the rest
of both: conditions on two layers, compiled to SQL, the sidebar counts on the level.

## Context

Every condition the owner touched in the Filter popover was written to the list or the view a moment later,
and the Me chip and the text field came back with the view's place. A quick look ("only mine, only bugs") so
changed the view for good, and undoing it meant editing the filter back by hand. The owner wants to try a
filter freely and decide afterwards whether the view keeps it.

## Decision

- **The Me chip and the popover's conditions are drafts.** A change applies to the rows at once and is written
  nowhere. The drafts live in the page's store per view (`state/slices/narrowing.ts`); both popover sections
  ("All views of this list", "This view") draft, the locked ancestor rows stay locked.
- **Rust runs the drafts.** `Narrowing` carries `listDraft` and `viewDraft`; when present, `views::query`
  puts them in place of the list's and the view's stored conditions and compiles them under the same chain
  `list_set_filter` and `view_set_filter` check. A draft that doesn't compile shows the view's error state.
  The sidebar counts stay on what is saved.
- **The Me chip is part of the saved view**: `view.only_mine` (migration `local/0014`), set by
  `view_set_only_mine`, copied when a view or a list is duplicated. A view opens with it. Places that kept the
  chip on in `places.json` seed it once at startup; `places.json` no longer holds the chip or the text.
- **Save View** shows left of the Filter button while the drafts differ from what's saved, and in the popover's
  footer. It writes the list's conditions (only when changed), the view's and its Me chip, then drops the
  drafts. ⌘S and the menus run it (`view.save`); Revert View (`view.revert`, also in the toolbar's "…" menu)
  drops the drafts.
- **Leaving discards.** Going to another view, list or level drops the drafts, the text filter and an epic
  narrowing (`useDiscardDraftsOnLeave` in `shell/Window.tsx`, on the nav place). The Issue view, the preview,
  the Peek and switching windows don't leave the view.
- **The text filter is never saved.**
- Rejected: saving as you edit with an Undo (a quick look still writes, and the Undo has to know what each
  write replaced); a draft per level instead of per view (two views of one list would fight over it); asking
  before leaving with unsaved drafts (the owner wants leaving to be the discard).

## Consequences

- What a view shows can differ from what it saved until the owner saves or leaves; the toolbar badge counts
  the drafts, and Save View is the visible sign.
- The live count in the popover counts top-level issues, like the sidebar ([0037](0037-views-choose-how-subtasks-show.md)).
- The command line's `taskboard me` sets the current view's draft, not its saved chip.
