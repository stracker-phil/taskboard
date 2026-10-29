# 0040. Subtasks are the children of a view's results

## Status

Accepted (2026-10-29). Owner's request. Supersedes in part [0037](0037-views-choose-how-subtasks-show.md) (what a
subtask is, what each mode shows, the counts, the keys) and the line of
[0038](0038-filter-changes-are-drafts-until-the-view-is-saved.md) that the popover's live count counts top-level
issues. Keeps the rest of 0037: the three modes in `view.subtasks`, the Board always hiding them, the collapsed
rows in `places.json`.

## Context

0037 took "subtask" to mean Jira's subtask issue type (`type_is_subtask = 1`) and nested only those. An "Epics"
list (filter: type = Epic) in Hierarchy then showed its epics without children: an epic's stories, tasks and
bugs are not subtasks in Jira, they are its children (`parent_id` = the epic). The owner thinks in ClickUp's
terms, where an item's children are its subtasks whatever they are, and the filter decides what the top level is.
Like "Space" isn't Jira's "Project", "subtask" isn't Jira's issue type.

## Decision

- **A view's results** are the issues its filter matches: the level's chain, the view's conditions or their
  drafts, the kind's rule, the epic narrowing. They are its top level, whatever their Jira type or parent.
- **Its subtasks** are the descendants of its results through `parent_id`, of any type, down to five levels
  (`views::subtasks::MAX_DEPTH`), gone and untracked issues left out. A path never repeats an issue, so a cycle in
  the parent links ends. They come in one query per view load, a recursive CTE over `cache.issue.parent_id`
  (indexed), never one per row.
- **Hidden** shows exactly the results. Each row's `childCount` counts its direct children of any type in the
  cache, whatever the filter, and shows as a chip ("76 child issues" as its tooltip). A Jira subtask that matches
  the filter is a row like any other.
- **Hierarchy** shows as roots the results without those that have an ancestor among the results; those nest
  under it instead, so no issue shows twice. Each root's descendants follow it in depth order, indented by level,
  unfiltered, sorted by the view's sort within each sibling set, in the root's group. A result a cycle or the
  depth cap keeps out of every subtree is a root of its own. The Me chip and the text field narrow the roots only.
  Every row with children gets a caret at any level; collapsing it hides its whole subtree. ← on a nested row
  selects its parent; ←/→ collapse and expand at any level.
- **List** shows the results and all their descendants as rows of their own, each grouped and sorted by its own
  values. A descendant that isn't a result has the subtask glyph before its title; a result looks the same
  whether it has a parent or not. The Me chip and the text field apply to every row on its own.
- **Counts:** a group counts its roots in Hierarchy and its rows otherwise. The sidebar and the Filter popover's
  live count count the results, as before subtasks had modes: no level leaves Jira's subtasks out any more.
- **Mark all as read** marks exactly what the current view shows, in its mode, with its filter, its drafts, the
  Me chip and the text (`list_mark_all_seen(listId, viewId, narrowing)`; Rust computes the ids from the same
  query the rows use). A Hierarchy view's descendants count, collapsed or not; a Board view counts as Hidden.
  Without a view it marks the level's unread issues, as before.
- Rejected: nesting only Jira's subtask type (an Epics list can't show its epics' stories); defaulting Unread and
  Bookmarks to List mode so a changed subtask isn't hidden (Hidden now shows every match, so nothing hides it);
  filtering the descendants too (a parent would show some of its children and hide others for reasons the row
  doesn't show).

## Consequences

- An Epics list in Hierarchy is a tree: each epic, its stories, tasks and bugs, and their subtasks.
- The sidebar counts may grow: a matching Jira subtask now counts like any issue.
- `Row.subtaskCount` is `Row.childCount`, `Row.parentId` is any issue's parent, and `Row.depth` says how deep a
  row nests in Hierarchy; `Row.isSubtask` marks a List-mode descendant, not Jira's type.
