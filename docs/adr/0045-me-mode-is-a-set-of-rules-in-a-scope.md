# 0045. Me mode is a set of rules in a scope

## Status

Accepted (2026-11-01). Owner's request, modelled on ClickUp's Me mode.

## Context

Me mode kept the issues assigned to me, saved per view (`view.only_mine`, [0038](0038-filter-changes-are-drafts-until-the-view-is-saved.md)).
The owner is also "mine" in other ways: they reported an issue, a comment mentions them, a subtask belongs to
their issue, or a project's own user field (a "Developer" field, say) names them. Such a field exists only in
some projects, and its id differs per Jira site, so no field may be hard-coded. The owner also wants Me mode to
stay on while moving between views.

## Decision

- **Assigned to me always counts; rules add more.** `me_mode::condition` ORs the assignee with the rules
  switched on in `behaviour.meRules`:
  - **A user field holds me.** These are Reporter or a custom user field, compiled by the filter compiler as
    `field is me`. A field the cache doesn't know matches nothing.
  - **A comment mentions me.** This reads `cache.comment.body_adf`, not the recorded `mention` changes:
    retention prunes those once they are read, and the first sync records none.
  - **A subtask of an issue that is mine** by the other rules.
- **The rules are global, the switches live in the Me popup.** Settings → General lists which user fields
  the popup offers ("Add Field…" picks from `cache.field`'s user fields). The popup switches each rule on or off.
- **Scope: this view or all views.** "Only this view" is the saved `view.only_mine` with its draft.
  "All views" is an app-wide flag (`UiState.me_everywhere`) that keeps Me on in every view until it is turned
  off.
- **Me and Person stay exclusive.** Turning Me on clears the person chip; picking a person, me included,
  turns Me off. Picking yourself in the person chip is a plain "assigned to me".

## Consequences

- Mention matching walks comment JSON. `instr` skips comments without my account id first, so the cost stays
  with the comments that name me.
- Mentions in descriptions don't count; the sync doesn't read them.
