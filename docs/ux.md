# Design language

The reference for how Taskboard looks and behaves. It is Daybook's design language (the most mature of the
owner's apps, itself grown from TimeTracker's), kept as it is: a native, calm, lightweight macOS app.
Decisions: [ADR 0007](adr/0007-color-palettes.md) (token sets, no literals),
[ADR 0009](adr/0009-a-scheme-per-system-mode.md) (schemes per macOS mode),
[ADR 0010](adr/0010-native-macos-look.md) (the native look). When in doubt, open Daybook
(`../daybook/docs/ux.md`, `../daybook/src/styles/`) and match it. Update this page with every change that
builds or changes an item.

## Principles

- **Calm and native.** It reads as a macOS app beside Calendar, Reminders and System Settings, never as a web
  page: unified toolbar, inset groups, AppKit-sized controls, generous spacing, no repeated information, no
  button for what a key already does.
- **Light.** The window opens instantly; nothing animates for show; chrome stays out of the way of the content.
- **Fully keyboard-usable**, light and dark. Every action is a command with a rebindable shortcut and a
  palette entry.
- **Hierarchy from weight, opacity and space**, not from boxes and colors: a strong title, then a normal
  subtitle, then meta in `--o2`.
- **The accent marks interactive things only:** keys, links, focus, selection.
- **All configuration lives in Settings.** A concept with several items gets its own page with one card per
  item.
- **One word for each thing** in every label, menu item, palette title, toast and in the docs. Accessible names
  follow the visible labels.
- **Demo data in every picture** ([demo-data.md](../.claude/rules/demo-data.md)).

## Surfaces

Neutral greys, no tinted page, no outlined cards.

| Token | Used for |
|---|---|
| `--sidebar-bg` | the sidebar, full height, under the window buttons; the window's sidebar material instead where the scheme's sidebar tone matches the macOS mode |
| `--bg` | content: the window background under the toolbar and the view |
| `--chrome` | the Settings page list |
| `--pop` | overlays inside a window: popovers, context menus, dialogs, toasts |
| `--fill-quiet` | inset groups (System Settings style) |

Separators are hairlines (`--line`; in the main window `--hairline`, `--text` at 6%). Vertical column edges
fade out at the top and bottom. Grouped content is an **inset group**: `--fill-quiet`, 10–12px radius, no
border, rows divided by very light hairlines, no hover fill on plain rows; its heading (13px semibold) sits
above it, outside the fill.

## Scrollbars

None are styled: every scroller keeps macOS's overlay scrollbars, shown only while scrolling. Any
`::-webkit-scrollbar` rule would make WebKit draw a permanent one. Every user-scrollable box is its own
stacking context (`isolation: isolate`, listed in `base.css`), so a nested scroller's bar never stays behind
while an outer box scrolls; menus and modals portal to `<body>` for the same reason. A scroller sits below its
titlebar row, so its bar never reaches the window buttons.

## Shell

- **Unified toolbar** (`Toolbar`): every view starts with one 52px row beside the window buttons (which sit
  centred in it, 20px from the left): the view's title (15px, `--text-strong`), then its actions at the
  trailing end. Empty parts drag the window. No app title, no Settings button (⌘, the app menu and the
  palette open Settings). Without the sidebar the row leaves `--traffic-lights` (84px) for the window buttons.
- **Sidebar** (if the app has one): 220px, full height on its own palette (dark in the Default scheme), a drag
  strip under the window buttons, rows of 32px with a 16px icon, 8px inset and 6px radius; a shortcut hint is
  a key chip. The selected row is `--sidebar-selected` with its label in `--sidebar-strong` (500) and its icon
  in the accent. A key hides it; the state is kept in `ui_state.json`.
- **Inactive window:** while the window isn't key, toolbar titles and buttons and the sidebar's content sit at
  40%.
- There is no status bar.

## Appearance and tokens

The app follows the macOS appearance; there is no app mode. Settings → Appearance picks a scheme per macOS
mode ("Light mode scheme", "Dark mode scheme", three tiles each) and one accent.

| Scheme | Sidebar | Content | Default for |
|---|---|---|---|
| Default | dark (opaque) | light | light mode |
| Light | light (vibrant in light mode) | light | |
| Dark | dark (a shade lighter than the content; vibrant in dark mode) | dark | dark mode |

Four palettes in `src/styles/palettes/` (`content-light`, `content-dark`, `sidebar-light`, `sidebar-dark`)
define the colors; `theme.css` only derives. Start them as copies of Daybook's.

**Content palette** tokens:

| Token | Light | Dark | Role |
|---|---|---|---|
| `--bg` | `#ffffff` | `#1e1e1e` | content surface |
| `--chrome` | `#f5f5f5` | `#252525` | the Settings page list |
| `--pop` | `#ffffff` | `#2c2c2c` | overlays |
| `--text`, `--text-strong` | `#262626`, black | `#e6e6e6`, white | primary text; titles and pressed segments |
| `--fill-quiet`, `--fill`, `--fill-strong` | black at 3.5, 5.5, 10% | white at 4.5, 8, 14% | inset groups; hover bezels, tracks, search fields, key caps; pressed bezels, switch track |
| `--line` | black at 12% | white at 10% | hairlines |
| `--control`, `--control-pressed`, `--control-shadow` | white, `#ebebeb`, ring + 1px drop | `#4a4a4a`, `#5c5c5c`, highlight + ring | push and popup button bezels |
| `--field`, `--field-line` | white, black at 16% | white at 5%, white at 16% | text fields |
| `--raised`, `--knob` | white | `#5a5a5a`, white | the pressed segment; the switch thumb |
| `--shadow`, `--shadow-small` | | | overlays (a hairline ring and a soft drop; no border); raised segment, thumb |
| `--scrim` | black 14% | black 35% | behind sheets |
| `--a-<hue>`, `--a-green` | Apple's light system colours | Apple's dark system colours | the nine accent hues and green; also data colours (projects, labels) |

**Derived** (`theme.css`): `--accent` (`AccentColor` for System, else the hue), `--o2`/`--o3` (`--text` at
68% and 52%), `--sel` (accent at 20%), `--accent-text` (the accent with its lightness capped for text),
`--on-accent` (white or near-black per hue), `--focus-ring`, `--style-red|yellow|blue|purple`, the type scale,
spacing, control heights and radii below.

## Type

| Token | Size | Used for |
|---|---|---|
| `--fs-heading` | 17px | large headings inside a view |
| `--fs-title` | 15px | toolbar titles, Settings page titles, the palette's field |
| `--fs-body` | 13px (the UI size) | everything else; sheet titles bold, group headings semibold |
| `--fs-meta` | 11px | secondary text: meta lines, labels, descriptions, counts |
| `--fs-caption` | 10px | key caps |

The system font (SF Pro) everywhere, tabular figures for numbers; SF Mono only for paths, ids, URLs and
commands. No bundled fonts. Weights 400, 500, 600, 700. Appearance → "Text size" (Small, Medium, Large; ⌘=,
⌘-, ⌘0) scales chrome, which is sized in rem of the UI size; the window's own chrome (toolbar, traffic
lights, sidebar width) stays in px.

| Level | Token | Opacity | Used for |
|---|---|---|---|
| Primary | `--text` | 100% | the thing you came for |
| Secondary | `--o2` | 68% | labels, units, relative times, meta (at least 4.5:1 on `--bg`) |
| Tertiary | `--o3` | 52% | counts, empty states, placeholders (at least 3:1 on `--bg`) |

## Spacing and radius

An 8px grid with 4px half steps (`--space-1` 4px to `--space-6` 24px, in rem); content keeps 20px margins.
Rows are at least 28px (36–48px in inset groups). Radii: 6px controls (`--radius-sm`), 10px menus, popovers
and inset groups (`--radius`), 12px sheets and panels (`--radius-lg`).

## Controls

AppKit sizes and looks, in `styles/controls.css`:

| Kind | Look |
|---|---|
| Push button `.btn` | 22px, 12px side padding, 6px radius, `--control` fill with `--control-shadow`; no hover change |
| Default button `.btn-primary` | accent-filled, `--on-accent` text (500); only a sheet's or window's default action, last in its row |
| Destructive `.btn-danger` | a push button with `--style-red` text |
| Borderless `.btn-quiet`, `.icon-btn` | no bezel until hovered (`--fill`), pressed `--fill-strong`; icons in `--o2` |
| Toolbar button | borderless, 28px, icon and label, hover bezel |
| Link `.link-btn` | plain `--accent-text` |
| Popup button `select.input` | WebKit's native chevrons in the push button bezel |
| Text field `.input` | 22px, `--field` with a 1px inset `--field-line`, focus adds `--focus-ring` |
| Search field `InsetField` | a `--fill` capsule with a leading icon and an optional key cap |
| Segmented control | a `--fill` track, the pressed segment raised on `--raised` |
| Switch | 32 by 18px, `--fill-strong` track, white knob; on, the track is the accent |
| Key cap `KeyCap` | `--fs-caption`, `--o3` on a `--fill` rounded rect |

Buttons in a row end with the default button: Cancel, then Save. Destructive actions sit at the leading end.
Labels after a name are plain secondary text, never chips. Icons are lucide-react at 16px.

A focus ring is for the keyboard only: buttons, rows and links show it on `:focus-visible`; text fields keep it
on any focus. The native WebKit context menu is suppressed outside text fields.

## Rows

Every list in a window shares one row: a 20px line with padding (at least 28px), hairlines between rows, no
hover fill on plain rows, `--sel` when selected, 55% for dimmed rows. Meta follows in `--fs-meta` `--o2`/`--o3`.

## Overlays inside a window

Toasts and dialogs are `--pop` with `--shadow` (no border). Toasts (`--radius`, bottom right) dismiss
themselves (success 2.5 s, info 4 s, errors 8 s, with an action 10 s), repeats count up (`×3`); an error reads
"Something went wrong" with Details. Sheets (`Modal`, `--radius-lg`, 20px padding, over `--scrim`) take Esc
first, a click on the scrim closes them, Tab cycles inside them, and closing returns the focus to where it was.

## Settings

A window of its own (about 880 × 720, at least 760 × 560) in the System Settings layout, as Daybook's:

- **Page list** on `--chrome` (190px): a drag strip for the window buttons and a search field that stay put
  (it narrows by page, group or section title), then 26px rows under small `--o3` group headings; the current
  page a grey `--fill-strong` fill. Only the list scrolls.
- **Page** on `--bg`: the title (`--fs-title` semibold) in the strip beside the window buttons, then either an
  intro or group descriptions (never both), then groups 24px apart. Every change applies at once; there is no
  Save. Esc, ⌘W and the close button hide the window; it reopens on the page it showed.
- **Group** (`Group`): a 13px semibold heading over its inset group (12px radius, 16px side padding). **Row**
  (`Row`): label and, 4px below, an optional description (`--fs-meta`, `--o2`, at most 27rem) on the left, the
  control on the right; 14px above and below, at least 48px; `.stacked` puts the control under the label, a
  row that depends on a switch sits indented under it.
- Fields and popups are 32px with a 10px radius; push buttons and segmented controls 28px; checkboxes quiet
  16px tiles that fill with the accent. A repeating item is one compact line: its fields, then ↑ ↓ and a trash
  icon. A failure reads in red on a light red tint.
- Pages register in `registry/settingsPages.ts`, in groups. The System group always holds General
  (launch at login), Appearance (schemes, accent, text size) and Shortcuts.

## Keys

App-level actions are Rust commands ([ADR 0012](adr/0012-commands-live-in-rust.md)): the app menu gives them
their keys, global shortcuts run them from anywhere (release builds only). The page's registry has only the
keys inside a window, rebindable in Settings → Shortcuts. `?` lists the bound keys; ⌘K opens the palette.

- Bare keys never fire in text fields; there only ⌘ and ⌃ combinations and Escape do.
- While a dialog is open, only `?` and the text size keys run.
- ↵ and Space on a focused button or link activate it. `Tab` is never bound.
- A command has one binding.

## Scheme audit

The three schemes are checked in both modes: `scripts/dev/screenshot-looks.sh <dir>` captures the window in
each scheme of the current macOS mode; switch the system appearance and run it again. Fixes live in
`src/styles/palettes/` only. `src/styles/contrast.test.ts` checks `--o2` ≥ 4.5:1 and `--o3` ≥ 3:1 on `--bg`,
`--on-accent` ≥ 3:1 on every accent, and sidebar text ≥ 4.5:1 on `--sidebar-bg`.
