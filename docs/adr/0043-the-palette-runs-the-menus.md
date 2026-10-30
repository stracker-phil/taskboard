# 0043. The palette runs the menus

## Status

Accepted (2026-10-30). Owner's request. Builds on [0012](0012-commands-live-in-rust.md) and
[0034](0034-menus-are-drawn-in-the-page.md).

## Context

The palette listed the page's registered commands and a few actions of its own, so most menu items (Settings,
Full Sync, Open Log Folder, Reset Cache…) and the view's options (grouping, sorting, subtasks) couldn't run from
it. Daybook's palette lists every command of its Rust bus and ranks them in Rust, with `>`, `#`, `/` prefixes
for one kind. In Taskboard, what the page can do right now lives in the page: its commands' enabled checks, the
toolbar's and the view tabs' menus. Rust can't see those.

## Decision

- **The menus are the list.** `menu/palette.rs` lists every enabled app menu item in menu order
  (`menu_commands`), from the same `MENUS` table the menu bar is built from, so a new menu item is a palette
  entry without further work. The page runs a `page:` item with its own command (left out when it can't run)
  and any other item through the bus.
- **The page adds its own menus.** Components offer theirs while on screen with `usePaletteMenu`
  (`palette/menus.ts`); the palette flattens them ("Group: Sort by › Updated"). The Settings pages come from
  the Settings page registry.
- **Ranked on the page.** Unlike Daybook, the commands are ranked in `palette/rank.ts` (Daybook's rules),
  because half of them exist only in the page. Issues and lists are still found by Rust; a prefix asks it for one
  kind with room for more (`PaletteScope`).
- **Prefixes as in Daybook**: `>` commands, `#` issues, `/` lists.
- The page's commands are snapshotted when the palette opens: some only run while no dialog or text field has
  the focus, which the palette itself would take.
- Rejected: listing the page's commands in Rust (it would need the page to report its state on every change);
  a palette list of its own beside the menus (the two would drift).

## Consequences

- The palette and the menu bar can't disagree: same items, same labels, same order.
- An action that is in neither a menu nor a page menu isn't in the palette; adding it to a menu is the way in.
