# 0002. IPC only through `api.ts` files

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0002.

## Context

Components that call `invoke` directly are hard to test (every test has to fake Tauri internals) and hide
which commands the page depends on.

## Decision

Only `src/api/index.ts` (and `src/<module>/api.ts` for an optional module) imports `@tauri-apps/api` or a
`@tauri-apps/plugin-*` package. Everything else imports the `api` object. UI tests mock that module with
`createApiMock()` and nothing else. `src/test/boundary.test.ts` scans the imports and fails on a violation.

## Consequences

- The page's whole contract with Rust is one file per module.
- A new command needs an `api` function and a default in `src/test/apiMock.ts`.
