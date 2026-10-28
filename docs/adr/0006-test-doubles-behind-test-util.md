# 0006. Test doubles and the demo seed ship behind a `test-util` feature

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0006 (TimeTracker ADR 0015), including TimeTracker's
extension to debug builds.

## Context

Unit tests need temp config dirs, a temp database, fakes for other apps and builders for domain values.
Integration tests in `src-tauri/tests/` are a separate crate and cannot see `#[cfg(test)]` items. The debug
build needs the same made-up data as the tests (ADR 0011). Compiling any of it unconditionally would put dead
code and demo data into the release binary.

## Decision

`src-tauri/src/test_support/` (fixtures, builders, fakes, the demo seed) compiles under
`cfg(any(test, feature = "test-util", debug_assertions))`. The crate lists itself as a dev-dependency with the
`test-util` feature, so integration tests get the doubles, debug builds get the seed, and release builds get
neither.

## Consequences

- One source of made-up data for tests, the debug app and the README screenshots.
- Every new double uses the same `cfg` gate.
- The self dev-dependency is an unusual Cargo pattern; `Cargo.toml` comments point here.
