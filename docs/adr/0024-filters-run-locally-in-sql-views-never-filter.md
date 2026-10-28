# 0024. Filters run locally in SQL; views never filter

## Status

Accepted (2026-10-28).

## Context

The spec separates *which* issues a sidebar level holds from *how* a view tab shows them: Everything, a Space
(one per tracked project), filter lists under a Space or under Home, and any number of views per level. Filter
lists must update on every sync, count instantly in the sidebar and never cost a Jira request; Jira is only
queried through the sync scope.

## Decision

- **A filter is a typed condition list** (`filter::Filter`, stored as JSON in `filter_list.filter`): field,
  operator, value. Fields are Jira's standard fields plus custom fields by id; `me` stands for the connected
  account. The conditions of a level are ANDed with every ancestor's, and a Space adds `project = X`.
- **Conditions compile to parameterised SQL** over `cache.issue` (`filter::compile`), never to JQL. Values only
  reach SQLite as bound parameters. Labels, fix versions and custom arrays are matched with `json_each`, custom
  fields with `json_extract` on the cached issue JSON. Every query is limited to tracked projects and issues
  Jira still lists.
- **A stored filter that can't be read or compiled** (an unknown field, a broken file) puts that list in an error
  state; the rest of the tree still works.
- **The sidebar counts open issues** of every level in one query (`COUNT(*) FILTER (WHERE …)` per node).
- **Views hold no conditions**: layout, grouping, sort and columns only. The Me chip and the text field narrow
  the moment and are never saved; a set worth keeping becomes a filter list.
- Rejected: JQL per filter list (a request per count and per view; offline impossible); filters on views
  (blurs which vs. how, the spec's core model).

## Consequences

- New filterable fields need a compile rule, not a re-sync, as long as the cached JSON holds them. A custom field
  used in a filter is marked `shown`, so the next sync fetches it.
- Date conditions are whole days in the account's time zone.
