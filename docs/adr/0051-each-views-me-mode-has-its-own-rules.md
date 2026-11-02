# 0051. Each view's Me mode has its own rules

## Status

Accepted (2026-11-02). Owner's report. Amends [ADR 0045](0045-me-mode-is-a-set-of-rules-in-a-scope.md): the rules
are no longer one app-wide set.

## Context

Me mode's rules (assigned to me, reporter, mentioned in a comment, subtasks, the owner's user fields) were one set in
`settings.json`. The Me popup changed that set, so turning a rule on while trying Me in one view changed what a view
saved as "My Work" showed.

## Decision

- A view saves the rules its Me mode has on (`view.me_rules`, a JSON list of `MeMatch`); none follows the app's.
  The popup's switches are a draft of the view, like the Me chip itself: Save View writes them, Revert View drops
  them, leaving the view drops them.
- Saving a view with Me on always writes its rules, so a saved Me view is fixed against later changes elsewhere.
- The app's set (Settings → Me Mode) lists the rules there are, in order, with the switches a view starts from.
  "For all views" uses it: while Me is on only through that scope, the popup's switches change the app's set.
- The page sends the rules on (`Narrowing.me_on`); Rust switches the app's set by them
  (`me_mode::rules_for`, `Narrowing::resolve_me_rules`) and filters the rows.

## Consequences

- A view saved with Me before this change has no rules of its own and follows the app's set until it is saved again.
- A rule the owner removes in Settings stops counting in every view, since a view only switches the app's rules.
