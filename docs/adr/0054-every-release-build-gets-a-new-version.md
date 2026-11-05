# 0054. Every release build gets a new version

## Status

Accepted (2026-11-05). Owner's decision.

## Context

`bun run release` installs a build over the running app. When two builds carry the same version, they cannot
be told apart, and it is unclear which commit the installed app runs.

## Decision

Every release build gets a new version; the same version is never built twice. Before `bun run release`, the
version is bumped by what changed since the last release (semantic versioning):

- patch (`1.0.1` → `1.0.2`): fixes and small polish, nothing new to learn;
- minor (`1.0.1` → `1.1.0`): a new feature, command, setting or visible behavior;
- major (`1.0.1` → `2.0.0`): a change that breaks stored data, the CLI or links without migration, or a
  redesign.

The version lives in `package.json`, `src-tauri/Cargo.toml` (with `src-tauri/Cargo.lock`) and
`src-tauri/tauri.conf.json`; all four change together in their own commit, "Set the version to X.Y.Z", after
the step's commit and before the build.

## Consequences

- The installed app's version names one commit.
- A release without code changes since the last one is not built.
