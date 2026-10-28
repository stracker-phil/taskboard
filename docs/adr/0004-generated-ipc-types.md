# 0004. TypeScript IPC types are generated from Rust with ts-rs

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0004.

## Context

Hand-written TypeScript copies of Rust types drift: a renamed field compiles on both sides and fails at
runtime.

## Decision

Every type crossing IPC derives `ts_rs::TS` with `#[ts(export, export_to = "src/api/types.gen.ts")]` and
`#[serde(rename_all = "camelCase")]`. `bun run types` regenerates the file (the ts-rs `export_bindings` tests;
`.cargo/config.toml` sets `TS_RS_EXPORT_DIR` and maps 64-bit integers to `number`). `bun run lint` fails when
the generated file differs from the committed one. Nobody edits it by hand.

## Consequences

- A changed Rust shape breaks `tsc` where the page uses it.
- Unit tests and lint both regenerate the file, so they never run at the same time.
