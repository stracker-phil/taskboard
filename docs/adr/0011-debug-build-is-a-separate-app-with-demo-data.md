# 0011. The debug build is a separate app with demo data

## Status

Accepted (2026-10-28). Combines Daybook ADRs 0014 and 0030 with TimeTracker's demo seed.

## Context

The release app runs all day; the debug build is started and stopped many times while a step is built. With
one identity both builds share the socket, the lock and the data: a debug launch hands off to the release app
or stops it, and half-finished code reads and writes the owner's tasks. The repo is also public, so
screenshots and manual tests must never show real data.

## Decision

- Debug builds (`bun run build:app`, `tauri.debug.conf.json`) are a second app: `Taskboard (Debug)`,
  `com.philippstracker.taskboard.debug`, URL scheme `taskboard-debug`. Everything that hangs off the
  identifier is separate: config folder, socket, lock, data, logs, Keychain items, privacy grants.
  `lib.rs` (`IDENTIFIER`, `APP_NAME`, `LINK_SCHEME`) has tests that compare them with both Tauri configs.
- The debug app starts on made-up data: on its first launch (an empty data folder) it loads the demo seed
  from `test_support/` (ADR 0006).
- Only release builds register global shortcuts, a login item and the `taskboard` command; the debug build is
  run by path (`scripts/dev/launch.sh`) and never installed.
- Dev hooks (`dev.rs`, JavaScript eval over the socket, `window.taskboard`) compile into debug builds only.

## Consequences

- Trying a change never touches the owner's data, and both apps run side by side.
- The README screenshots come from the debug app on the demo seed.
- Each build is its own entry in Privacy & Security, granted once.
