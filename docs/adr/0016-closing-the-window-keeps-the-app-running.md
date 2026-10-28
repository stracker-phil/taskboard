# 0016. Closing the window keeps the app running

## Status

Accepted (2026-10-28). Taskboard's own; Daybook keeps a menu-bar item instead.

## Context

Taskboard syncs Jira in the background (every 15 minutes, on wake, on network return), so the process has to
outlive its window. Daybook solves this with a menu-bar item, a LaunchAgent and global shortcuts; Taskboard's
spec asks for none of them, and a menu-bar item would be one more surface to keep in step.

## Decision

Taskboard is a regular Dock app. Closing the main window or Settings (the red button, ⌘W) hides it; the app
keeps running until Quit (⌘Q, the app menu, `taskboard quit`). Clicking the Dock icon, `taskboard show` or
`taskboard focus` shows the window again. No tray, no LaunchAgent, no global shortcuts.

## Consequences

- Background sync runs while the window is hidden, as long as the app runs.
- The Dock icon stays while the app runs, also with every window hidden.
- `RunEvent::ExitRequested` without an exit code is ignored, so only an explicit quit ends the app.
