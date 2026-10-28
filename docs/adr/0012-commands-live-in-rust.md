# 0012. Commands live in Rust

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0017.

## Context

The same action is reachable from the app menu, the menu-bar menu, a global shortcut, the command palette, the
CLI and a link. If each surface wires its own handler, they drift: one checks a precondition, another doesn't.

## Decision

App-level actions are commands of one Rust bus (`bus/`), each with an id (`task:new`), a title, an enabled
check and a handler. Menus, global shortcuts, the palette, the CLI words and links run them by id. The page
keeps a registry only for keys inside a window (moving the selection, toggling a column), rebindable in
Settings → Shortcuts. Every action is a command with a palette entry and, where useful, a shortcut.

## Consequences

- One place decides whether an action is possible, for every surface.
- The palette and the help list come from the bus, so they can't miss a command.
