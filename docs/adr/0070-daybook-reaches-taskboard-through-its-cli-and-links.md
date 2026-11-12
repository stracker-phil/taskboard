# 0070. Daybook reaches Taskboard through its CLI and links

## Status

Accepted (2026-11-12). Inherited from the family catalog (sibling-apps-talk-through-the-cli; reference:
Launcher ADR 0047).

## Context

Daybook opens an issue in Taskboard: the release build runs `taskboard open <KEY>`, the debug build opens
`taskboard-debug://issue/<KEY>` so it never reaches the owner's Taskboard. Both were tested only as parsing in
Taskboard, so renaming the subcommand or the link form would have passed every test here and broken Daybook.

## Decision

- Taskboard is the receiving side. `taskboard open <KEY>` (exactly two arguments) and `<scheme>://issue/<KEY>`
  are a contract, pinned by tests that name Daybook (`cli/tests.rs`
  `open_keeps_the_arguments_daybook_passes`, `links.rs` `the_issue_link_keeps_the_form_daybook_sends`).
- A new subcommand, argument or link form is compatible. Renaming or removing one, or changing what it takes,
  needs the owner's approval first; the step's report names what Daybook has to change (`ship-step` asks).
- Rejected: Daybook reading Taskboard's files or database (it would bypass the single writer); relying on the
  parsing tests alone (nothing says a sibling depends on them).

## Consequences

- The contract tests change only on purpose; `docs/links.md` and `docs/adapting.md` name Daybook as a caller.
