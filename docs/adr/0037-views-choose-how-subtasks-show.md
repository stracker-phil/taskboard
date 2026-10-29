# 0037. Views choose how subtasks show

## Status

Accepted (2026-10-29). Owner's request, after ClickUp's subtask modes.

## Context

Subtasks (`cache.issue.type_is_subtask = 1`; their `parent_id` is the parent) were ordinary rows whenever they
matched a level's filter. A sprint list then mixed a story with its own small steps, in different groups, with
nothing tying them together. ClickUp lets each view choose: hide subtasks, show them under their parent, or show
them as rows of their own.

## Decision

- **A view holds its subtask mode** in `view.subtasks` (migration `local/0013`): `hidden`, `hierarchy` or `list`;
  null (every existing view) reads as hidden. `view_set_subtasks` sets it; duplicating a view or a filter list
  copies it. The Board ignores it and always hides subtasks; its toolbar has no Subtasks button.
- **Hidden:** the view's query leaves subtasks out. Each row carries `subtaskCount`, all its subtasks in the
  cache (not filtered), shown as a small chip after the title.
- **List:** subtasks are rows like any other. The level's filter, the view's filter, the Me chip and the text
  field apply to each issue on its own, and a subtask is grouped and sorted by its own values, so it can sit in
  another group than its parent. Subtask rows show the subtask icon before the title.
- **Hierarchy:** the query runs on top-level issues only. Every subtask of a matching parent follows it, in the
  parent's group, indented one level, sorted by the view's sort among themselves, without the filter, the Me chip
  or the text field. Subtasks of a parent that isn't shown don't show. A parent with subtasks gets a caret that
  collapses them; expanded is the default, and the collapsed parents are kept per view in `places.json`
  (`ViewPlace.collapsedParents`).
- **Group counts** count top-level rows in Hierarchy and every row in List.
- **The sidebar counts top-level issues only**, on every level and whatever its views' modes.
- **Keys:** ↑/↓ walk the visible rows, subtask rows included. In Hierarchy, ← on a subtask selects its parent and
  ←/→ on a parent with subtasks collapses or expands them; elsewhere ← and → keep closing and opening the group.
- Rejected: one app-wide mode (a sprint list and a bug list want different ones); filtering subtasks in Hierarchy
  too (a parent would show some of its steps and hide others for reasons the row doesn't show).

## Consequences

- Unread follows its views' modes like any level: a subtask that changed shows on its own in List, under its
  parent in Hierarchy only when the parent is unread too, and not at all in Hidden. The sidebar's Unread count
  doesn't count it.
- A view's rows and the sidebar count can differ: a List-mode view may show more rows than its level counts.
