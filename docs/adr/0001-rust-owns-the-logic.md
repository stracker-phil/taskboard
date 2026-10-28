# 0001. Rust owns the logic, React only renders

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0001 (TimeTracker ADR 0001).

## Context

A task manager is mostly rules: what is due, what is blocked, how a list is ordered and grouped, what a move
or a status change does to the tasks around it. Each rule must be right once and testable without a browser.
The webview renders text typed by the owner and pasted from other apps, so it is the part most exposed to
surprises.

## Decision

Every rule lives in Rust: storage, validation and settings sanitizing, ordering, filtering, grouping, date math
and the text the app shows. The page asks for views and results, renders them and forwards intent (a key, a
click, a field value); it never decides on its own what a command does. Splitting logic between the two sides
was rejected because the same rule would then exist twice, or only in the half that cannot be unit tested
without a DOM.

## Consequences

- The logic is unit tested in Rust.
- More IPC types and round trips, even for small things.
- A UI feature usually needs a Rust change first.
