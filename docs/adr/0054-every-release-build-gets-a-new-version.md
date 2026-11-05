# 0054. Every release build gets a new version

## Status

Accepted (2026-11-05). Owner's decision; taken over from TimeTracker's ADR 0043 and its `scripts/release.sh`.

## Context

`bun run release` rebuilt and installed the app under an unchanged version, so installs could not be told
apart and it was unclear which commit the installed app runs.

## Decision

`bun run release major|minor|patch|X.Y.Z` (`scripts/release.sh`) is the only way to release. It needs a clean
working tree, moves the version forward in `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`
and `src-tauri/Cargo.lock`, builds and installs (`scripts/install.sh`), then commits "Release X.Y.Z" and tags
`vX.Y.Z` locally. A failed build puts the version files back; a tagged version is never built again.

The bump is chosen from `git log v<last>..HEAD`: major if existing use breaks (CLI, links, stored data
without migration, a removed feature), minor for new features or visible behavior, patch otherwise.

## Consequences

- The installed app's version names one commit, and `git log vA..vB` lists what a release brought.
- Every release adds a commit and a tag; a wrong bump is fixed by the next release, never by moving a tag.
