# 0005. Keep `panic = "unwind"`

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0005 (TimeTracker ADR 0014).

## Context

The app runs all day. With `panic = "abort"` one bug in one command takes the whole app down, and unsaved
state with it.

## Decision

The release profile keeps unwinding, so a panic fails one command and the app keeps running. Locks are taken
through the poison-tolerant helpers in `lock.rs` (`MutexExt`, `RwLockExt`); stores change a copy and swap it
in after the save, so a panic leaves the old value intact. An `unwrap()` whose failure is plausible at runtime
becomes an `AppError` instead.

## Consequences

- A slightly larger binary.
- No `unwrap()` on a lock anywhere in production code.
