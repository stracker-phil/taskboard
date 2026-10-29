# 0033. Views can hold their own filter

## Status

Accepted (2026-10-29). Owner's request. Supersedes the "views hold no conditions" line of
[0024](0024-filters-run-locally-in-sql-views-never-filter.md); keeps the rest of 0024 (typed conditions compiled
to SQL, the sidebar counts, the error state).

## Context

A filter list often wants several views that differ in more than layout: "Bugs" with a Table "All bugs" and a
Table "For release" that only shows bugs with a release set. With only list conditions, a condition added for
one view changed every view of the list; the owner had to make a second filter list and lose the shared base.

## Decision

- **A view holds its own conditions** in `view.filter` (migration `local/0012`), the same JSON `filter::Filter`
  as `filter_list.filter`; null holds none.
- **What a view shows** is its own conditions ANDed with its level's chain (the list, its ancestors, a Space's
  project), plus the level kind's rule and the toolbar's narrowing (`views::query`). This covers the rows, the
  groups and the Board columns.
- **The sidebar stays list-level**: its counts and Mark all as read use the level's chain only.
- **A view filter that can't be read or compiled** puts only that view in an error state.
- **`view_set_filter`** compiles the conditions under the level before storing them and marks the custom fields
  they use `shown`, like `list_set_filter`. Duplicating a view or a filter list copies the view filters.
- **The Filter popover has two sections** under the locked ancestors: "All views of this list" (the list's own
  conditions) and "This view". Levels without editable conditions of their own (Everything, a Space, Unread,
  Bookmarks) show only "This view". The toolbar badge counts both layers.
- The Me chip and the text field stay per view in `places.json`, as before.
- Rejected: view conditions that replace the list's (the list would no longer say which issues it holds);
  a second filter list per variant (duplicates the shared base and the sidebar entry).

## Consequences

- "Which issues" now lives on two levels; the popover labels each section so the owner sees which one a
  condition changes.
- A sidebar count can be higher than what a narrowed view shows.
