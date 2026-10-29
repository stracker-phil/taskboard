# 0034. Menus are drawn in the page

## Status

Accepted (2026-10-29). Owner's request. Amends [0010](0010-native-macos-look.md) for context menus and pop-up
buttons; the app's menu bar stays native.

## Context

Context menus and pop-up buttons (Group, the Space settings' pickers) were native macOS menus, shown through
Tauri's menu API from the page. The "+ View" popover, drawn in the page, showed what the owner wants every menu
to look like: captioned groups, a tile and a hint per row. A native menu can't draw tiles in the app's colors,
captions or a red Delete, keeps its own look beside the app's popovers, and every caller had to cross IPC to
show one.

## Decision

- **One in-app menu.** `components/menu` draws every context menu and pop-up button's menu; the store slice
  `state/slices/menu.ts` holds the open one, `MenuHost` (once per window) shows it, `openMenu` and
  `openContextMenu` open it from anywhere. Entries carry an icon, a hint, a check, a danger style, a caption or
  a submenu.
- **It behaves like a macOS menu.** It opens at the mouse or under its button, stays inside the window, closes
  on Escape, a press outside, a scroll or leaving the window, moves with the arrow keys and letters, and gives
  the focus back. Roles are `menu`, `menuitem`, `menuitemradio` and `menuitemcheckbox`.
- **Short choices are inline.** A short set of choices, such as a view's layout, is a captioned group with a
  check rather than a submenu.
- `api.popupMenu` and the page's use of Tauri's menu API are gone. The menu bar (`src-tauri/src/menu/`) is
  unchanged.
- Rejected: native menus with custom item icons (no captions, colors or tiles in the app's palette, and still
  a second look beside the popovers).

## Consequences

- Menus follow the palettes (light and dark) and are tested like any component, without IPC.
- The menu can't extend past the window's edges as a native one can; it flips and shifts inside instead.
- Accessibility, keys and placement are the app's to keep right (`docs/ux.md` → Menus).
