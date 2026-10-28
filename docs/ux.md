# Design language

The reference for how Taskboard looks and behaves. The main window follows the owner's Taskboard mockup, a
light ClickUp-style window ([ADR 0017](adr/0017-the-main-window-follows-the-taskboard-mockup.md)); the Settings
window and the behaviour (keys, scrollbars, overlays) follow Daybook's design language. Colours come only from
palettes ([ADR 0007](adr/0007-color-palettes.md)). Update this page with every change that builds or changes
an item.

## Principles

- **Built to the mockup.** Sizes, colours and type match it; nothing is redesigned. Where the mockup is
  silent, Daybook's behaviour applies.
- **Light and fast.** The window opens instantly; nothing animates for show.
- **Fully keyboard-usable.** Every action is a command with a rebindable shortcut and a palette entry.
- **The sidebar decides which issues, the view tabs decide how.** Filters never live in a view.
- **One word for each thing** in every label, menu item, palette title, toast and in the docs. Accessible names
  follow the visible labels.
- **Demo data in every picture** ([demo-data.md](../.claude/rules/demo-data.md)).

## Window

- **Frame** (`src/shell/Window.tsx`, `styles/shell.css`): a 264px sidebar (padding 16 12 12, gap 16) on
  `--window-bg`, beside the content panel: `--panel-bg`, inset 8px top, right and bottom (none toward the
  sidebar), 12px radius, `--shadow-panel`. Popovers anchor to the panel.
- **Traffic lights** are the native ones, inside the sidebar's top 18px strip (`trafficLightPosition` 20, 24);
  the strip and empty chrome drag the window. There is no top bar, icon rail or status bar.
- **Sidebar** (top to bottom): traffic-light strip, Search (⌘K), home rows (Everything, Recently changed,
  Pinned), the Spaces tree, the footer with the sync chip and Settings. ⌘\ hides it; the panel then takes the
  full width under a 42px strip that keeps the traffic lights off it. The state is kept in `ui_state.json`.
- **Content panel** until the issue list (step 9): `SyncStatusView` (`src/shell/SyncStatusView.tsx`) says
  "Connect Jira in Settings" with an Open Settings button while no project is tracked; during a full sync
  "Loading your Jira issues" with a line and a 4px bar (`--accent` on `--fill-subtle`) per tracked project
  (fetched of Jira's estimate); afterwards "N issues in M Spaces" and the last sync; otherwise the sync state.

## Tokens

One palette per macOS mode in `src/styles/palettes/` (`light.css`; the dark palette joins later), selected by
`data-scheme` on `<html>` (`lib/theme.ts`). Every palette defines the same tokens.

| Group | Tokens | Light |
|---|---|---|
| Surfaces | `--window-bg`, `--panel-bg`, `--panel-subtle-bg` (popover footers), `--fill-subtle` (key hints, tracks), `--fill-hover`, `--segmented-bg`, `--scrim` | `#F4F4F5`, white, `#FAFAFA`, `#F4F4F5`, `#EBEBEE`, `#EEEEF1`, black 38% |
| Text | `--text`, `--text-strong-body`, `--text-secondary`, `--text-tertiary`, `--text-muted`, `--text-faint` (decorative glyphs only), `--text-on-accent`, `--link`, `--link-hover` | zinc 900 → 400, white, blue 700, blue 900 |
| Lines | `--border` (controls, the tree guide), `--border-strong` (selects, dashed adds), `--hairline` (section edges), `--row-hairline` (between rows), `--card-border` | `#E4E4E7`, `#D4D4D8`, `#EDEDF0`, `#F1F1F3`, `#E9E9EC` |
| Accent | `--accent`, `--accent-text`, `--selection-bg`/`--selection-text`, `--accent-soft-bg`/`-border`/`-text`, `--button-primary-bg`/`-text`, `--spinner-track`, `--focus-ring` | blue 600, blue 700, `#E3EBFD`, … |
| Changed | `--changed-bg`, `--changed-border`, `--changed-chip-bg`/`-text`, `--changed-dot` | light blues |
| Status | `--status-{todo,progress,review,blocked,done}-{bg,fg,border,ring,tint,count}` | grey, blue, violet, red, green |
| Data | `--flag-*`, `--epic-{1..5}-{bg,fg}`, `--label-{1,2,neutral}-{bg,fg}`, `--avatar-{1..4}`, `--avatar-ring`, `--space-tile-{1..5}`, `--view-{list,board,table}` | |
| Messages | `--banner-{info,warn,error}-*`, `--notice-neutral-*`, `--pin-*`, `--pr-open-*`, `--sync-*`, `--progress-done` | |
| Shadows | `--shadow-{panel,popover,palette,peek,raised,card,knob}` | |

The mockup's near-duplicates are merged: the palette's own greys map to the zinc scale, the palette's active
row uses `--selection-bg`, the PR chip uses the Checkout epic's purple, the warning banner text uses the amber
`#92400E`. The accent is fixed blue; there is no accent or scheme choice. Hover fills (not in the mockup) use
`--fill-hover`.

## Type

The system font (`-apple-system`, SF Pro) at a 14px root; SF Mono for issue keys, versions and field ids. No
bundled fonts. Weights 400, 500, 600, 700, 750 (titles) and 800 (tile letters).

| Role | Size / weight | Used for |
|---|---|---|
| Issue title | 30 / 750 | Issue view heading |
| Peek title | 22 / 750 | peek heading, page headings |
| Space title | 19 / 700 | content header |
| Card title, palette field | 17 / 700, 17 / 400 | state cards, the palette's query |
| Section | 16 / 700 | Description, Child issues, empty states |
| Body | 14 / 400 (600 when strong) | rows, summaries, sidebar rows |
| Control | 13 / 400 (600 when strong) | toolbar buttons, popover rows, buttons, banners, breadcrumbs |
| Meta | 12 / 400 (600 for headers) | counts, column headers, timestamps |
| Status pill | 12 / 700 caps, 0.03em (11 and 10 in smaller pills) | status pills |
| Key | mono 12 (11 on cards) | issue keys |
| Chip, badge | 11 / 600–700 | change chips, counts badges |

## Metrics

Radii: 6 (tiles, chips, row hover), 7 (pills, small buttons, popover rows), 8 (sidebar rows, toolbar
buttons), 9 (search, sync chip), 10 (cards), 12 (panel, popovers, board columns), 16 (palette), 50% (avatars,
rings, dots). Heights: sidebar search and home rows 34, Space rows 36, filter-list rows 32; view tabs 42,
toolbar 50 (controls 30), column header 34, group header 44, list row 46, board column 300 wide.

## Scrollbars

None are styled: every scroller keeps macOS's overlay scrollbars, shown only while scrolling. Any
`::-webkit-scrollbar` rule would make WebKit draw a permanent one. Every user-scrollable box is its own
stacking context (`isolation: isolate`, listed in `base.css`), so a nested scroller's bar never stays behind
while an outer box scrolls; menus and modals portal to `<body>` for the same reason. A scroller sits below its
titlebar row, so its bar never reaches the window buttons.

## Overlays inside a window

Toasts and dialogs are `--pop` with `--shadow` (no border). Toasts (`--radius`, bottom right) dismiss
themselves (success 2.5 s, info 4 s, errors 8 s, with an action 10 s), repeats count up (`×3`); an error reads
"Something went wrong" with Details. Sheets (`Modal`, `--radius-lg`, 20px padding, over `--scrim`) take Esc
first, a click on the scrim closes them, Tab cycles inside them, and closing returns the focus to where it was.

## Settings

A window of its own (about 880 × 720, at least 760 × 560) in Daybook's System Settings layout, until the
owner's own Settings design replaces it. Its colours are the `--settings-*` tokens of the same palette.

- **Page list** on `--settings-chrome` (190px): a drag strip for the window buttons and a search field that
  stay put (it narrows by page, group or section title), then rows under small group headings; the current
  page a `--settings-fill-strong` fill. Only the list scrolls.
- **Page** on `--settings-bg`: the title in the strip beside the window buttons, then groups 24px apart. A
  change applies at once (fields on blur or ↵); only small forms whose fields belong together (the Jira site,
  the scope query) save with their own Save button. Esc, ⌘W and the close button hide the window; it reopens
  on the page it showed.
- **Group** (`Group` in `components/settings/controls.tsx`): a 13px semibold heading over its inset group
  (`--settings-fill-quiet`, 12px radius). **Row** (`Row`): label and an optional description on the left, the
  control on the right, hairlines between rows.
- **Controls** are Daybook's: `TextField`, `NumberField` (commit on blur or ↵, numbers only within their
  bounds) and `DraftField` (a form's field) in `components/settings/fields.tsx`, with Rust's message for the
  field (`invalid_input` with `field`) in red under it; `Segmented` and `ControlRow` (a form's buttons and its
  result line) in `controls.tsx`. A destructive action (`.btn-danger`) asks first in a sheet
  (`components/Modal.tsx`, an `alertdialog` over `--scrim`). Other failures are error toasts.
- Pages register in `registry/settingsPages.ts`, in groups: **Jira** first, then **System**.
  - **Connection** (`components/settings/jira/`): *Jira site* (site URL, email, the API token as a password
    field that never shows a saved token, Classic or Scoped token kind, the cloud ID for Scoped; Save and
    Test Connection, with "Connected as …" or the error below) and *Token* (the expiry date and how soon it
    runs out).
  - **Projects**: Refresh after a successful test, "N of M tracked", a filter field (every word in the name or
    key, ignoring case) and a Track switch per project.
  - **Sync**: *Scope* (the effective JQL in a monospaced field; Validate, Save, Reset to Default), *Intervals*
    (minutes in front and in the background, days to keep done issues) and *Cache* (the sync state line with
    Sync Now, one row per tracked project with its key, last full sync and "N issues · M open", then Reset
    Cache, confirmed in a sheet). It stays as the diagnostics view of the cache.
  - **General** (System): About and logs. Spaces, Statuses, Shortcuts and Appearance join with their features.
- The layout stays Daybook's until the owner's own Settings design arrives.

## Keys

App-level actions are Rust commands ([ADR 0012](adr/0012-commands-live-in-rust.md)); the app menu gives them
their keys (⌘, Settings, ⌘\ Toggle Sidebar, ⌘W, ⌘Q). There are no global shortcuts. The page's registry
(`src/commands/`) has only the keys inside a window, rebindable through `keybindings.json`; ⌘K opens the
palette once it exists.

- Bare keys never fire in text fields; there only ⌘ and ⌃ combinations and Escape do.
- While a dialog is open, only `?` and the text size keys run.
- ↵ and Space on a focused button or link activate it. `Tab` is never bound.
- A command has one binding.

## Palette checks

`src/styles/colors.test.ts` keeps colour literals inside `src/styles/palettes/` and every palette on the same
token set; `src/styles/contrast.test.ts` checks text tokens on their surfaces (4.5:1 body, 3:1 meta and large
text) and pill text on pill fills. With a dark palette, `scripts/dev/screenshot-looks.sh <dir>` captures the
window in the current macOS mode; switch the system appearance and run it again.
