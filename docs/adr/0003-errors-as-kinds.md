# 0003. Errors cross IPC as `{kind, message}`

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0003 (TimeTracker ADR 0003).

## Context

The page reacts to classes of errors: a cancelled dialog is no failure, a missing item offers to reload, an
invalid field marks that field. Parsing English messages for that is fragile.

## Decision

`AppError` (`src-tauri/src/error.rs`, `thiserror`) serializes to a stable `kind` (`io`, `invalid_input`,
`not_found`, `cancelled`, and more as needed), the display `message` and, for form errors (`InvalidField`),
`field: { path, message }`. The UI branches on `kind` only; `toAppError` accepts only known kinds and turns
anything else into `io`. The socket carries the same pair, so `taskboard --json` prints it. A new kind is
added when the page has to branch on it, not for every new failure.

## Consequences

- UI flows that depend on the class of an error are plain branches, testable with a mock.
- Adding a kind touches both sides (`ErrorKind` and the page's known kinds).
- Messages may change freely without breaking the UI.
