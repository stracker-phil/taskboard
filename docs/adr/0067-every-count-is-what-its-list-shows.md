# 0067. Every count is what its list shows

## Status

Accepted (2026-11-12). Owner's rule. Supersedes in part [0024](0024-filters-run-locally-in-sql-views-never-filter.md)
(the sidebar counts open issues), [0038](0038-filter-changes-are-drafts-until-the-view-is-saved.md) and
[0040](0040-subtasks-are-the-children-of-a-views-results.md) (the Filter popover's live count counts the results)
and the "open counts" consequence of [0055](0055-status-styles-are-chosen-app-wide-a-space-overrides.md).

## Context

The sidebar and the Filter popover counted open issues only, while the lists show done issues too: Recently viewed
said 2 over 6 rows, and the popover said "2 issues match" above the same 6. A count that disagrees with the rows
reads as a false claim.

## Decision

- **A sidebar count is what the list's own filter returns**: its conditions and what it inherits (the Space,
  Recently viewed's time window), every status. A view's own conditions ("This view") don't change it
  ([0033](0033-views-can-hold-their-own-filter.md): the sidebar stays list-level).
- **The view line's count** ("5 issues") is the rows the view shows, its own conditions and narrowing included.
- **The Filter popover shows the view line's number** ("N issues match" while anything narrows, else "N issues"),
  so it always equals the rows below.
- No count leaves out issues by status, and none counts in a way its label doesn't say.

## Consequences

- The sidebar counts grow by the done issues in each list's window.
- `filter::OPEN_ISSUES` and `MatchCount` are gone; `list_match_count` returns one number for the empty-level probe.
- The Space's count (`SpaceNode.count`, the CLI's "21 issues") counts every issue the Space holds.
- Settings → Sync's "open" figure stays: it is labelled open and isn't a list.
