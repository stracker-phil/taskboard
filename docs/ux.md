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
- **Sidebar** ([below](#sidebar)): ⌘\ hides it; the panel then takes the full width under a 42px strip that
  keeps the traffic lights off it. The state is kept in `ui_state.json`.
- **Content panel** (`main`, a flex column): the [header](#content-header), the [view tabs](#view-tabs), then
  the view body (`.content-panel__body`). Until the issue list the body is `SyncStatusView` (`src/shell/SyncStatusView.tsx`) says
  "Connect Jira in Settings" with an Open Settings button while no project is tracked; during a full sync
  "Loading your Jira issues" with a line and a 4px bar (`--accent` on `--fill-subtle`) per tracked project
  (fetched of Jira's estimate); afterwards "N issues in M Spaces" and the last sync; otherwise the sync state.

## Sidebar

`src/shell/Sidebar/` (`Sidebar.tsx` composes it), styled in `styles/sidebar.css`. The tree comes from
`useSidebar` (`sidebar_tree`), the place from `useNav` (`nav_current`); a click runs `nav_go` through
`useNavGo`. Top to bottom, 16px apart:

- **Traffic-light strip** (18px, a drag region) for the native window buttons.
- **Search**: 34px, 9px radius, a `--border` frame on `--panel-bg`, `--text-muted`; Search icon 15/2, the
  label, a "⌘K" key hint (11px, `--fill-subtle`, 5px radius). Disabled until the palette exists.
- **Home** (`HomeRows`): Everything (`Layers2`), then the Home lists in Rust's order: Recently changed
  (`History`), Pinned (`Pin`) and global filter lists (`Funnel`). 34px rows, 10px padding and gap, 8px radius,
  `--text-strong-body`; icons 17/1.8; the count 12px `--text-muted`, hidden while Rust has none.
- **Spaces** (`SpacesSection`, scrolls): the "Spaces" header (12/600 `--text-muted`) with "+" (24px), which
  opens Settings → Projects (`app:settings:projects`). Each Space (`SpaceRow`): 36px, a 10px ▾/▸ chevron in
  `--text-faint`, the 22px tile (6px radius, 11/800 letter on `--space-tile-N`), the name and the open count.
  The current Space has `--selection-bg` with `--selection-text` and a 600 name; the Space of the current
  filter list keeps no fill but a 600 name in `--text`.
- **Expanding**: the chevron toggles a Space (saved with `nav_set_expanded`); ⌥-click keeps only that Space
  expanded. The Space holding the current level always shows expanded.
- **Filter lists** (`FilterListTree`): indented 34px behind a 1px `--border` guide with 12px padding; 32px
  rows, 9px gap, `--text-secondary`, funnel 15/2, name and count (`ListCount`; a list whose filter can't be
  used shows a muted `CircleAlert` with the reason as its tooltip instead). The current list is selected like
  a Space, 600. "+ New filter list" (30px, 13px, `--text-muted`) creates "New filter list", opens it and
  starts renaming it.
- **Footer** (`SidebarFooter`): the sync chip (34px, 13px `--text-tertiary`; a 7px dot in `--sync-ok`,
  `--sync-busy`, `--sync-offline` or `--sync-error-dot` by state; the state in words from
  `describeSyncState`; a refresh icon 14/2; a click syncs now) and Settings (34px, gear 17/1.8).
- **Hover**: rows, "+", the chip and Settings take `--fill-hover`; the selected row keeps its fill.
- **Context menus** are native (`api.popupMenu`). A filter list: Rename, Duplicate, Pin to Home (disabled
  until pins), Delete. A Space: Rename, Colour and icon…, Client label…, Stop tracking.
- **Renaming** (`components/InlineRename.tsx`): the name turns into a field with a 2px `--focus-ring`; ↩ or
  leaving the field saves a changed, non-empty name, esc cancels.
- **Reordering** (`useDragReorder`): drag a Space, or a filter list within its Space, with the mouse; after
  4px a 2px `--accent` line shows where it lands, and the drop runs `space_move` or `list_move` with the new
  index. The dragged row dims to half.

## Content header

`src/shell/Header.tsx`, styled in `styles/header.css`: 14px 24px 12px padding, a row with a 10px gap.

- A Space: the 28px tile (8px radius, 14/800) and the name (19/700).
- A filter list: a breadcrumb button with the 20px tile (6px radius, 10/800) and the Space name (13px
  `--text-tertiary`, opens the Space), "/" in `--text-faint`, the funnel 15/2 in `--text-tertiary`, then the
  list name (19/700).
- Everything and the Home lists: their sidebar icon (20/1.8, `--text-tertiary`) and name.
- **Open in Jira ↗** on the right for a Space and its lists: 32px, 12px padding, 8px radius, a `--border`
  frame, 13px `--text-secondary`; opens `<site>/browse/<KEY>`. Hidden elsewhere and without a site.

## View tabs

`src/shell/ViewTabs.tsx`, a `tablist` of the current level's views (`useViews`, `views_of`): 42px, 20px
padding, 2px gap, a `--hairline` bottom line.

- A tab: 8px padding, 7px gap, `--text-tertiary`; an 18px tile (5px radius) in `--view-list`,
  `--view-board` or `--view-table` with the white `List`, `Kanban` or `Table2` glyph (11/2.8), then the name.
  The current tab is `--text`, 600, with a 2px `--text` underline over the bar's line.
- After the tabs a 1px `--border` separator (10px inset) and "+ View" (`--text-muted`), a native menu of List,
  Board and Table that adds a view of that layout named after it and opens it.
- Right-click a tab: Rename (in place), Duplicate, Change layout ▸ (the other layouts), Delete (disabled for
  the only view; deleting the current view opens the level's default).
- ⌘1–⌘9 open the nth tab.

## Status language

One language for every Jira status (`src-tauri/src/status_style/`): five styles, To do, In progress, In
review, Blocked and Done. Jira's category decides To do (`new`) and Done (`done`); otherwise a name containing
"review" is In review, one containing "block" is Blocked, everything else In progress. Settings → Statuses
overrides it per status.

- **StatusRing** (`components/issue/StatusRing.tsx`): a 2px circle in `--status-*-ring`, dashed for To do;
  15 in list rows, 12 inside pills, where it takes the pill's text colour.
- **StatusPill**: caps 12/700 (0.03em), 26 high, 7 radius, `--status-*-bg/fg/border`, the ring inside.
- **EpicChip** (22, 12/500, at most 164 wide, `--epic-N-*`): Jira's epic colour (`ghx-label-N`) maps to the
  nearest of the five tints; an epic without one gets a tint from its id.
- **Avatar** (26, white 9/700 initials on `--avatar-N`): the connected account is always `--avatar-1`, other
  people one of the rest by a stable hash of their account id. In a stack each has a 2px `--avatar-ring` border
  and overlaps by 6.
- **PriorityFlag** (14, `--flag-*`, Lowest shares Low's colour), **ChangeChip** (18, `--changed-chip-*`, 11/600),
  **CommentCount** (12, bubble icon), label chips (22, `--label-N-*` by a hash of the name).

## List view

`src/views/list/`, fed by `view_rows` (`views/query.rs`): Rust filters, groups and sorts, the page only draws.

- Column header 34 (`--hairline` below, 12px `--text-muted`), padded 50 on the left so "Name" lines up with
  the rings; then each visible column at its width: Assignee 96, Epic 176, Release 96, Priority 104, Labels
  120, Sprint 100, Due 80, custom fields 90.
- Groups 14 apart: a 44px header with a 22px chevron (▼ open, ▶ closed), the group's label (a status pill
  when grouped by status; a Space tile, an avatar, an epic chip or a flag for the other groupings) and its
  count. Done groups start closed. Without grouping there are no headers.
- Rows 46, 26 indent, `--row-hairline` between, 6 radius: ring, mono key, summary (600 when changed, on
  `--changed-bg`), change chip, comment count, then the cells. A clicked row is selected (`--selection-bg`).
- Rows are virtualised (`@tanstack/react-virtual`), so a 5,000-issue view renders only what is on screen.

## Toolbar

`src/shell/toolbar/`, 50 high under the tabs, one popover open at a time (`components/Popover.tsx`: under its
button, Escape and a press outside close it, focus goes back to the button).

- **Group: Status** (30 high, `--border`, 8 radius): a native menu of the groupings (Status, Space, Client,
  Assignee, Epic, Release, Priority, Due date, None) with Sort by ▸ (Updated, Created, Priority, Key, Due date,
  Summary, Status; Ascending, Descending). Saves to the view.
- **Columns** ("Card fields" on a board): the *Columns for this view* popover (340 wide): Jira fields, then
  "Custom fields · this project" with the field id as `cf_N`; a row per field with a drag handle (⋮⋮) to reorder
  and a 30 × 18 switch (`components/Toggle.tsx`). Each change saves to the view; "Use as Space default" sets
  the columns new views in the Space start with.
- **Filter** (badge: the filter list's own conditions): the *Filter* popover (440 wide, right-aligned): "Show
  issues where" and the live count of open issues ("N issues match" on a filter list or with a draft, "N open
  issues" on a Space before any condition); inherited conditions locked on `--fill-subtle` with a lock, "this Space" / "from Space"; own
  conditions as field / operator / value selects (value pickers from the cache, dates, days, text) with ×;
  "+ Add condition". On a filter list changes save as you edit ("Changes save to “Name”."); on a Space or
  Everything they are a draft that "Save as filter list" turns into a new list and opens.
- **Me** chip (on: `--accent-soft-*` pill with the `--avatar-1` avatar and "Me ×") and **Filter this view**
  (190 wide, filters by key or summary as you type). Both narrow only the current view and are never saved.

## Table view

`src/views/table/`: the same rows as the list, 36 high, with a sticky 34px header: Key, Summary (flexible),
Status, then the view's columns. A sortable header (Key, Summary, Status, Priority, Due) sorts on click and
again flips the direction (`aria-sort`, ↑/↓); a 6px handle at each header's right edge resizes the column
(48–800), saved on the view. Grouped tables keep the list's group headers; the table scrolls sideways when the
columns are wider than the window.

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
Everything that loses the owner's setup asks first in a `ConfirmSheet` (`components/ConfirmSheet.tsx`; the page
calls `useStore.getState().confirm({ title, text, action })` and `ConfirmHost` shows it): deleting a filter list
or a view, Stop tracking, Reset Cache. The question names what goes and what stays; Cancel and Esc keep it.

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
  - **Spaces** (`SpacesPage`): the tracked Spaces in sidebar order, each a row with its tile, name, key and
    client and Move Up / Move Down buttons; clicking the row opens its fields below it (`SpaceFields` in
    `components/SpaceEditor.tsx`): Name (empty uses the project's), Icon letter (one letter or digit), Colour
    (five swatches of `--space-tile-1..5` beside a preview tile) and Client (empty means internal). Each field
    saves on its own with Rust's field error under it. "Track projects under Projects to get Spaces." when none
    is tracked. The sidebar's "Colour and icon…" and "Client label…" items open `SpacePopover` (300px, 12px
    radius, `--shadow-popover`, at the top left of the content panel) with the same fields; Esc, Done or a
    click outside close it.
  - **Statuses**: every cached Jira status as its pill with its Jira category, and a button with a native menu
    of Automatic (naming the detected style) and the five styles.
  - **Sync**: *Scope* (the effective JQL in a monospaced field; Validate, Save, Reset to Default), *Intervals*
    (minutes in front and in the background, days to keep done issues) and *Cache* (the sync state line with
    Sync Now, one row per tracked project with its key, last full sync and "N issues · M open", then Reset
    Cache, confirmed in a sheet). It stays as the diagnostics view of the cache.
  - **General** (System): About and logs. Shortcuts and Appearance join with their features.
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
- In the Taskboard window: ⌥⌘↑ and ⌥⌘↓ open the previous and next Space (the bus commands `nav:previousSpace`
  and `nav:nextSpace`, which wrap), ⌘1–⌘9 the nth view tab.
- `app:settings:<page>` opens Settings at a page or section (the Spaces "+" uses `app:settings:projects`).

## Palette checks

`src/styles/colors.test.ts` keeps colour literals inside `src/styles/palettes/` and every palette on the same
token set; `src/styles/contrast.test.ts` checks text tokens on their surfaces (4.5:1 body, 3:1 meta and large
text) and pill text on pill fills. With a dark palette, `scripts/dev/screenshot-looks.sh <dir>` captures the
window in the current macOS mode; switch the system appearance and run it again.
