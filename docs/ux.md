# Design language

The reference for how Taskboard looks and behaves. The main window follows the owner's Taskboard mockup, a
light ClickUp-style window ([ADR 0017](adr/0017-the-main-window-follows-the-taskboard-mockup.md)); the Settings
window and the behavior (keys, scrollbars, overlays) follow Daybook's design language. Colors come only from
palettes ([ADR 0007](adr/0007-color-palettes.md)). Update this page with every change that builds or changes
an item.

## Principles

- **Built to the mockup.** Sizes, colors and type match it; nothing is redesigned. Where the mockup is
  silent, Daybook's behavior applies.
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
- **Sidebar** ([below](#sidebar)): ⌘\ or the sidebar toggle hides it; the panel then takes the full width under
  a 42px strip that keeps the traffic lights off it. The state is kept in `ui_state.json`.
- **Sidebar toggle** (`src/shell/SidebarToggle.tsx`): a 34px button (panel icon 17/1.8, styled like Settings)
  that the frame, not the sidebar, draws 12px from the window's left and bottom edges, so it never moves. With
  the sidebar shown it is the footer's first item; hidden, it floats over the panel's lower-left corner on
  `--panel-bg` with `--shadow-panel` (`--shadow-hover` on hover), and the Table and Board keep 46px clear
  below their content. "Hide sidebar" or "Show sidebar" with the current shortcut; `aria-pressed` while the
  sidebar shows. It runs `view:toggleSidebar` and stays the same element, so focus stays on it.
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
- **Home** (`HomeRows`): Everything (`Layers2`), then the Home lists in Rust's order: Unread
  (`History`), Bookmarks (`Bookmark`) and global filter lists (`Funnel`). 34px rows, 10px padding and gap, 8px radius,
  `--text-strong-body`; icons 17/1.8; the count 12px `--text-muted`, hidden while Rust has none. Recently
  changed has no count: while it has unseen issues its label is 600 and a badge (20px, `--accent`, white
  11/700) shows how many.
- **Spaces** (`SpacesSection`, scrolls): the "Spaces" header (12/600 `--text-muted`); Spaces are added in
  Settings → Projects, not here. Each Space (`SpaceRow`): 36px, a 10px ▾/▸ chevron in `--text-faint`, the 22px
  tile (6px radius, 11/800 letter on `--space-tile-N`) and the name; no count (only filter lists count). Once it
  has filter lists, hovering shows a 24px + on the right that adds one.
  The current Space has `--selection-bg` with `--selection-text` and a 600 name; the Space of the current
  filter list keeps no fill but a 600 name in `--text`.
- **Expanding**: the chevron toggles a Space (saved with `nav_set_expanded`); ⌥-click keeps only that Space
  expanded. The Space holding the current level always shows expanded.
- **Filter lists** (`FilterListTree`): indented 34px behind a 1px `--border` guide with 12px padding; 32px
  rows, 9px gap, `--text-secondary`, the list's icon 15/2 (the funnel unless one was picked), name and count (`ListCount`; a list whose filter can't be
  used shows a muted `CircleAlert` with the reason as its tooltip instead). The current list is selected like
  a Space, 600. A Space without filter lists shows "+ New filter list" (30px, 13px, `--text-muted`); it and
  the Space row's + create "New filter list", open it and start renaming it.
- **Selected items edit in place**, like Finder: a first click on a Space or list selects it; a click on the
  selected one's name renames it, one on its tile or icon opens the look editor (`LookPopover`, saved with
  `space_update` or `list_set_look`) under it. A double-click renames as well. The header and Home show the same
  tile.
- **Footer** (`SidebarFooter`): room on the left for the window's [sidebar toggle](#window), then the sync
  chip (34px, 13px `--text-tertiary`; a 7px dot in `--sync-ok`, `--sync-busy`, `--sync-offline` or
  `--sync-error-dot` by state; the state in words from `describeSyncState`; a refresh icon 14/2; a click syncs
  now) and Settings (34px, gear 17/1.8).
- **Hover**: rows, "+", the chip, Settings and the sidebar toggle take `--fill-hover`; the selected row keeps
  its fill.
- **Context menus** ([Menus](#menus)). A filter list: Rename, Icon and Color…, Duplicate, Add to Home or
  Remove from Home, Delete. A Space: Rename, Icon and Color…, Client label…, Space Settings…, Stop tracking.
- **Renaming** (`components/InlineRename.tsx`), everywhere a name is renamed: the name turns into a field with
  a 2px `--focus-ring`; ↩ or leaving the field saves a changed, non-empty name, esc cancels. No rename uses a
  dialog.
- **Reordering** (`useDragReorder`): drag a Space, or a filter list within its Space, with the mouse; after
  4px a 2px `--accent` line shows where it lands, and the drop runs `space_move` or `list_move` with the new
  index. The dragged row dims to half.

## Content header

`src/shell/Header.tsx` and `HeaderTitle.tsx`, styled in `styles/header.css`: 14px 24px 12px padding, a row
with a 10px gap.

- A Space: the 28px tile (8px radius, 14/800) and the name (19/700).
- A filter list: a breadcrumb button with the 20px tile (6px radius, 10/800) and the Space name (13px
  `--text-tertiary`, opens the Space), "/" in `--text-faint`, the funnel 15/2 in `--text-tertiary`, then the
  list name (19/700).
- The level shown is the selected one: a click on a Space's or list's name renames it in place, one on its
  tile or funnel opens `LookPopover` under it.
- Everything and the Home lists: their sidebar icon (20/1.8, `--text-tertiary`) and name.
- **Mark all as read** on the right for Unread: 28px, 7px radius, a `--border` frame, 12/600; disabled
  while nothing is unseen. It marks what the current view shows, in its subtask mode and with its drafts.
- **Open in Jira ↗** on the right for a Space and its lists: 32px, 12px padding, 8px radius, a `--border`
  frame, 13px `--text-secondary`; opens `<site>/browse/<KEY>`. Hidden elsewhere and without a site.

## Tiles and the look editor

Spaces, filter lists and views share one look (`components/look/`, Rust `look.rs`): an icon and a tile color.
The icon is one letter, digit or emoji (the first grapheme typed; ⌃⌘Space opens macOS's emoji picker) or the
name of a line icon from `components/look/icons.ts`; the color is one of the twelve `--space-tile-N`. `Tile`
draws the icon in `--text-on-accent` (white) on the color with a 5–8px radius; without a color (lists and views
only) the icon stands alone in the text color. A Space without an icon shows its name's first letter, a list
the funnel, a view its layout's tile. `LookEditor`: a 32px preview, Icon / Letter or emoji, Reset, the 8-column
icon grid or the text field, then the swatches ("No color" first where allowed). `LookPopover` puts it under the
tile it changes: a click on the tile of the selected Space, list or view tab (sidebar, header, tabs; `LookButton`
and `SpaceLookButton`), on the tile in the Space settings header, or "Icon and Color…" in a context menu. The
Space settings' General group also shows it inline.

## View tabs

`src/shell/ViewTabs.tsx`, a `tablist` of the current level's views (`useViews`, `views_of`): 42px, 20px
padding, 2px gap, a `--hairline` bottom line.

- A tab: 8px padding, 7px gap, `--text-tertiary`; an 18px tile (5px radius) in `--view-list`,
  `--view-board` or `--view-table` with the white `List`, `Kanban` or `Table2` glyph (11/2.8), then the name.
  The current tab is `--text`, 600, with a 2px `--text` underline over the bar's line.
- After the tabs a 1px `--border` separator (10px inset) and "+ View" (`--text-muted`, also ⌘T): a menu
  captioned "New view" with a row per layout (its tile, name and a one-line hint) that adds a view of that
  layout named after it and opens it.
- A click on a tab opens it; a click on the open tab's name renames it in place (so does a double-click), one
  on its tile edits the look. Drag a tab sideways to reorder (`useDragReorder` on the x axis, `view_move`), with
  a 2px `--accent` line where it lands; a drag is never a click.
- Right-click a tab: Rename (in place), Icon and Color…, Duplicate, a "Layout" group (a row per offered layout
  with its tile, the current one checked), Delete (red; disabled for the only view; deleting the current view
  opens the level's default).
- ⌘1–⌘9 open the nth tab.

## Status language

One language for every Jira status (`src-tauri/src/status_style/`): six styles (status types), Backlog,
To do, In progress, In review, Blocked and Done. Jira's category decides To do (`new`; Backlog when the name
contains "backlog") and Done (`done`); otherwise a name containing "review" is In review, one containing
"block" is Blocked, everything else In progress. The Group menu and the Columns popover also offer Status type:
one group, Board column or pill per style, whatever the status is called; the Board orders them Backlog first. A Space's settings
(Settings → Spaces → the Space → Statuses) override it per status for that Space only.

- **StatusSquare** (`components/issue/StatusSquare.tsx`): the only status marker, a filled 10 × 10 square,
  radius 3, in `--status-*-square` (To do gray, Backlog lighter). No rings or outlines anywhere.
- **StatusPill**: caps, text only, symmetric padding, `--status-*-bg/fg/border`. Variants: default (26 high,
  12/700, 7 radius, 1px border, dashed for Backlog), `peek` (24, 11/700, no border), `activity` (20, 10/700), `tab` (the List's
  group tab: 30, 0.04em, top corners 8, all corners when closed; To do uses `--status-todo-tab-*`).
- **EpicChip** (22, 12/500, `--epic-N-*`): Jira's epic color (`ghx-label-N`) maps to the
  nearest of the five tints; an epic without one gets a tint from its id.
- **Avatar** (26, white 9/700 initials on `--avatar-N`): the connected account is always `--avatar-1`, other
  people one of the rest by a stable hash of their account id. In a stack each has a 2px `--avatar-ring` border
  and overlaps by 6. A person with a Jira profile picture shows it instead of the initials, round and covering
  the circle (`object-fit: cover`), as a `data:` URL from `user_avatar` (`queries/avatars.ts`, stale after an
  hour); people on Jira's default (Gravatar) picture and the demo people keep their initials
  ([ADR 0032](adr/0032-avatars-come-from-atlassians-avatar-host-through-rust.md)).
- **PriorityFlag** (14, `--flag-*`, Lowest shares Low's color), **ChangeChip** (18, `--changed-chip-*`, 11/600),
  **CommentCount** (12, bubble icon), label chips (22, `--label-N-*` by a hash of the name).

## List view

`src/views/list/`, fed by `view_rows` (`views/query.rs`): Rust filters, groups and sorts, the page only draws.

- The list canvas is `--list-bg` under a `--hairline`; padding 22 24 40 52 (a 30px chevron gutter).
- Each group is a white card (radius 10, top-left 0, `--shadow-group-card`, `--row-hairline` between rows)
  with its **tab** sitting on it: a status pill (`tab` variant; for other groupings a neutral tab with the
  Space tile, avatar, epic color or flag) and the count. Clicking the tab toggles the group; a closed group is
  the tab alone. Groups are 44 apart; Done starts closed.
- **Column headers** sit in each group's tab row (11/600 caps, 0.05em, `--text-muted`), bottom-aligned with
  the tab; there is no global header row. Without grouping the one card has the headers and no tab.
- The **chevron** lives in the gutter, hidden until the pointer or focus is in its group, and turns −90° when
  closed.
- **Columns** share one grid (`--list-columns`): the title takes the rest, every other column is as wide as
  its header or its widest cell across all loaded rows of the view (`measure.ts`, canvas text metrics,
  cached), so columns line up across groups, but no column is wider than 200 (`MAX_COLUMN_WIDTH`); a longer
  value, such as a long epic name, truncates and shows in full on hover. Gap 20, rows padded 16.
- Rows 44: square, mono key, summary (600 when changed, on `--changed-bg`), change chip, comment count, then
  the cells. Long titles wrap onto more lines by default and the row grows (each row is measured); "Wrap long
  titles" in the Columns popover turns that off per view, and titles truncate instead (`view.wrap_titles`). A
  wrapped title is one flow: the key and the subtask glyph lead it, the change chip, comment count and child count
  follow its last word, and its lines start under the key; only the square keeps its own column. The Table
  always truncates at a fixed row height; Board cards always wrap. A clicked row is selected (`--selection-bg`).
  A cell without a value shows a muted "—" (`EMPTY`, `views/list/columns.tsx`), in the List and the Table alike.
- An issue's labels show as small tags (`row-labels`, 18px `label-chip`s) after its title in the List, the
  Table and on Board cards, unless the view shows the Labels field. The Columns popover lists the field as
  "Labels (column)" ("Labels (card field)" on a Board) and has a "Labels (inline)" switch (`view.inline_labels`,
  on by default) for every layout.
- Labels never leave their cell. A label is at most as wide as its cell and ends in an ellipsis (the whole name
  shows on hover). With wrapped titles, inline labels flow after the title's last word and wrap like text,
  and the Labels column's chips wrap onto more lines in the cell; the row grows to fit. With "Wrap long titles"
  off, both stay on the row's one line and are cut at the cell's edge; the title comes first and truncates
  only when the cell is narrower than the title itself, and inline labels fill what it leaves. On one line a
  change chip gives way before the title: it truncates with an ellipsis down to a few letters first.
- Rows are virtualized (`@tanstack/react-virtual`): a card is drawn from its pieces (the header item, then
  rows that know whether they are first, middle, last or the only one).
- **Subtasks** are the children of the view's results at any depth and of any type, and follow the view's
  mode ([ADR 0040](adr/0040-subtasks-are-the-children-of-a-views-results.md)). Hidden: a row with children shows
  "⟨glyph⟩ 3" after its title (`ChildCount`; the subtask glyph of `components/issue/SubtaskIcon.tsx`, a circle top
  left, a line down bending right into a circle bottom right, lucide style), muted 12px like the comment count,
  "3 child issues" as its tooltip. List: a subtask is a row like any other, with the glyph before its title.
  Hierarchy: each result's subtree follows it, every level indented 24 more (`list-row--child`, the row's
  `--depth`); a 16px caret (`SubtaskCaret`) collapses or expands its subtree at any level: a top-level row's sits
  in the gutter left of the card, under its group's chevron; a nested row's 6 before its square. Like the group's
  chevron it shows only while its row has the pointer (or it has focus). A collapsed row shows the count chip.
  Collapsed rows are kept per view (`places.json`); expanded is
  the default.

## Peek

⌥ + hover over any issue (anything marked `data-issue-id`: List and Table rows, Board cards, related and child
issues, bookmarked issues, epic chips) shows a 380px card beside it after 150 ms (`issue/hoverPeek/`): key ·
type, the 16/700 title, status pill, epic, priority and assignees, and the description fading out at 360px. It
sits on the right of the element when there is room, else on the left, never takes the pointer and hides when
⌥ is released or the pointer leaves the issue. Nothing navigates and nothing counts as read.

## Issue preview

`src/issue/Peek.tsx` (the code still calls it peek), shown by `shell/CurrentPeek.tsx`: clicking a row in a List
or Table view selects it and opens a 540px panel at
the content panel's full height; header, tabs, toolbar and view narrow beside it and the List hides the
columns that no longer fit beside a 240px title, rightmost first, until it closes. It slides in.

What a click opens is the view's choice (Columns → Open issues in: Default, Preview, Full view; `view.open_with`,
null for Default) or else Settings → General → Open issues in (`behaviour.openIssuesIn`, Preview unless changed;
`useIssueOpening`). Full view opens the Issue view at once, the row still selected. A double-click on a List or Table row
always opens the Issue view; a Board always opens it.

- Header 50: mono key · type text, then Bookmark (with bookmarks), Open in Jira, Full view, Close.
- Body: title 22/750, one line of chips (status pill `peek`, epic chip, release outline chip, priority,
  assignee), the sanitized description, web links, the latest three activity items with "All activity".
- Footer 36: the keys.
- Hovering a row shows its toolbar at the end of the title: Mark as read (unread rows) or Mark as unread, Copy
  key, Bookmark, Open in Jira (`RowActions`).

## Change tracking

An issue is unread while the sync recorded a change after the owner last read it, or while the owner marked it
unread (`src-tauri/src/changes/`). The UI says read and unread; the code says seen and unseen.

- Changed rows: `--changed-bg`, a 600 summary and the change chip with the most important change ("New",
  "2 new comments", "Moved to Review", "Priority raised", "Assigned to you", "Due date changed"; "+N" for
  more kinds). Changed epic children in the Issue view get the same background.
- The Since-yesterday banner (`SinceBanner`, `--banner-info-*`) in the preview and the Issue view: "Since
  yesterday:" (or "Since you last looked:" when that was over a day ago) and every change in words; the Issue
  view adds Mark as read.
- Activity after the last look (`fresh`, decided in Rust) sits under a "New since yesterday" divider
  (`--accent-text`, `--changed-border` lines); new comments and changes have `--changed-bg`.
- An issue open in the preview or the Issue view for the owner's delay (Settings → General → Behavior, 0–10 s,
  default 2) counts as read (`useSeenAfter`); leaving sooner doesn't. Mark as read, Mark all as read and M
  clear it at once.
- Mark as unread (row toolbar, the Issue view header, M on a read issue) makes the issue unread
  (`issue_seen.unread`). Hovering a comment or change in the activity shows Mark unread from here: that item
  and everything newer count as new again. An issue marked unread while it is open stays unread until it is
  opened again (`holdUnread`).
- Unread keeps the issues read there (`NavStore` remembers them) until the owner goes to another level or
  view; they lose the unread look but don't vanish.

## Command palette

`src/palette/` (⌘K, or Search in the sidebar); Rust's `search::palette_query` finds the issues and lists.

- A `--scrim` over the window; the 640px panel (16px radius, `--shadow-palette`) 110px from the top, rising in.
- Input row 56: search icon, the 17px field, "N issues · M actions" on the right. It searches once typing
  pauses for 80 ms; the last results stay until the next arrive.
- Issues (max 8): a whole key or a bare number first, then full-text hits (every word as a prefix; key and
  summary weigh most). Each 46px row: status square, mono key (70), the summary with the match bold, the
  status pill, ↩ on the active row; a comment-only match adds "Matched in comment: “…”" under it.
- Filter lists (max 6): Spaces and lists whose name holds the text; Space tile and "Space › List".
- Actions (max 6): Open KEY in Jira (⌘↩), Save “text” as a filter list (⌘S; a `text contains` list under the
  current Space or Everything, opened), Refresh now (⌘R), Full sync, Group by …, then every titled command.
- Keys: ↑↓ move (wrapping), ↩ opens (an issue in the full Issue view over its Space's view, its row selected; a list; an action), ⌘C copies
  the key, esc or a click outside closes. The active row is `--palette-active-bg`. Footer 38 lists the keys.

## Bookmarks

Local only (`src-tauri/src/pins/`, `src/pins/`; the code calls them pins). Issues, comments and filter lists can be
bookmarked; a filter list's bookmark puts it under Home.

- `PinButton` (the `Bookmark` icon) in the preview and Issue view headers ("Bookmark this issue") and on each
  comment (on hover) opens `PinPopover` (340px): "Keep bookmarked for" chips (Today, 2 days, 1 week, Until
  removed; the owner's default from Settings → General preselected, Until removed unless changed; the chosen one
  `--button-primary-bg`), an optional one-line reason, Cancel and Bookmark; ↩ bookmarks. ⌥-click bookmarks at
  once with the default; a bookmarked target's button is amber and removes the bookmark on click.
- The row toolbar's Bookmark and P toggle the issue's bookmark (with the default duration) without the popover;
  the filter-list context menu has Add to Home / Remove from Home.
- Today ends at local midnight, 2 days at the end of tomorrow, 1 week seven midnights on. The page refreshes
  when the next bookmark runs out (`usePinExpiry`).
- A bookmarked comment shows the amber strip (`--pin-bg`, `--pin-text`): bookmark icon, the reason, the time left.
- The Bookmarks page (`PinnedView`, no tabs or toolbar): one card with the bookmarked issue (status square, key,
  summary), comment (author, excerpt, issue key) or list (Space / name), Reason and Time left; hover shows ×
  to remove the bookmark. "Recently removed" lists what ended in the last 7 days. The Home row counts the active
  bookmarks. Time left refreshes every minute while the page is open.

## States

- **First sync** (`FirstSync` in `shell/SyncStatusView.tsx`): a 360px card on the content panel while the sync
  runs and the current Space hasn't finished its first full sync: "Loading your Jira issues", a line, a 6px
  bar ("About X of Y issues"), then each Space done (✓ on `--progress-done`, its count), loading (spinner,
  600 name, "page n") or waiting (dashed ring, muted); "You can already browse finished Spaces." Finished
  Spaces open their views meanwhile.
- **Empty level** (`EmptyLevel`, List, Board and Table): the funnel tile, "Nothing in <name>", the conditions
  in words plus the Me and text narrowing, "Edit filter" on a filter list (opens the toolbar's Filter) and
  "Clear Me filter" while Me is on.
- **Gone issue** (`GoneNotice`, preview and Issue view): the title struck through in `--text-muted`, the notice
  on `--notice-neutral-*` ("Deleted in Jira or you lost access. Last synced <day>. This is the cached copy."),
  Remove from cache (`issue_forget`, gone issues only) and Remove bookmark while bookmarked.

## Sync chip and token banners

The sidebar footer's chip (artboard 8, panel 5) says "Synced 2 min ago" (counting on), "Syncing… 3 changes"
with a turning icon, "Offline · cache from 14:02" on white, or "Sync paused · token" / "Sync failed" in red; its
tooltip explains, a click syncs now (⌘R). Under the content header (panel 4) a red banner says Jira rejected
the token (sync paused) with Update token, and an amber one warns 14 days before the token expires, with
Replace and × (dismissed for the rest of the day). Nothing blocks: the app keeps working from the cache.

## Issue view

`src/issue/IssueView.tsx`, shown in place of the view (the sidebar stays) by ↩ on a selected row, the preview's
Full view button or E (the selected issue's epic); esc goes back one step (an opened epic or child, then the
view with its place intact).

- Header 50: back, breadcrumb (Space tile and name / list or view / key and a copy-link button), "Updated …", previous and next in the
  view's order (also J/K), Mark as read or unread, Bookmark (with bookmarks), **Open in Jira** (primary).
  Previous and next step through the rows the view shows (not those in collapsed groups or subtrees), move the
  view's selection along, and keep the current issue on screen until the next one loads (the neighbors load
  ahead). An issue missing from the cache says so; any other failure shows Rust's reason.
- Main column: type badge · key · comment count (· attachment count, when there are any), the 30/750 title, the field grid (Status, Assignee, Epic chip
  that opens the epic here, Release, Priority, Sprint, then the custom fields shown as columns or marked shown,
  then every other custom field the issue fills, by name), links ("blocks", "is blocked by"), the description,
  then each rich text custom field with content (acceptance criteria, steps to reproduce) as its own section
  under its Jira name (`RichFields`; the preview shows them under its description too), Web links when there are
  any (`WebLinks`, also in the preview: a pull request icon and `repo#481` for GitHub, GitLab and Bitbucket pull or
  merge requests, else a link icon and the host; a row opens its address in the browser through `open_url`, only
  when it is https, otherwise it is plain text; no remote icon loads), and Child issues (epics) or Subtasks, when there are any:
  open first, done behind "Show N done"; "Open as list view" narrows the origin view to the epic (a removable
  chip in the toolbar, never saved). Last, Attachments when there are any (`Attachments`): a 16/700 heading over
  a two-column grid (gap 8) of tiles (h 44, r 9, `--hairline` border): a 28 × 28 badge with the extension in caps
  (`--fill-subtle`; "FILE" without one), the file name (13/500, ellipsis) and "1.2 MB · Marta Klein" (11, muted).
  A click or Space shows the file in Quick Look ([ADR 0035](adr/0035-attachments-preview-in-quick-look.md)): Rust
  downloads it first, while the badge spins and the meta line reads "Loading…"; a failure shows the error toast.
  The context menu has Quick Look, Open in Browser and Copy link ("Copied the link to …"); the last two only for
  an address on the connected https site (`open_url`). Names are plain text.
- The breadcrumb's Space and list are buttons that go there. Going to the place already shown (sidebar, tabs,
  breadcrumb) closes the Issue view and the preview (`useNavGo`).
- Activity (400 wide by default, 300–720 by dragging its left edge or ←/→ on the focused edge; saved in
  `ui_state.json` `activityWidth` for every issue, `useActivityWidth`; `--panel-subtle-bg`): All / Comments / Changes, comment cards and change lines (status
  changes as activity pills; a change line stays on one line, its author and field truncate before from → to), newest first; the comment box waits for writes. Opening an issue fetches its
  comments when the cached ones may be behind (`issue_sync_comments`). Change lines come from Jira's changelog
  ([ADR 0031](adr/0031-issue-history-comes-from-the-changelog-on-demand.md)) and the sync's own records: "<Author>
  changed priority Medium → High", "Changed description" for long text without values, other fields by their
  Jira name in lowercase. Selecting a row (List, Table) or opening an issue asks Rust to fetch the history and
  the web links (`refreshIssueExtras`; each at most every 5 minutes), so they are usually there by the time the
  issue opens.
- Rich text is rendered by Rust from ADF ([ADR 0036](adr/0036-rich-text-renders-from-adf-not-renderedfields.md),
  [jira.md](jira.md#rich-text-adf)) and sanitized (`lib/jiraHtml.ts`,
  [ADR 0027](adr/0027-jira-html-is-sanitised-and-shows-no-remote-content.md)): links open in the browser, remote
  images and other attachments become chips, and only the renderer's `adf-*` class names and `jira-chip` survive.
  An image attached to the issue shows inline ([ADR 0039](adr/0039-inline-images-come-from-issue-attachments-through-rust.md),
  `InlineImage`): at most the content width, keeping its aspect ratio, radius 8 and a `--hairline` border; while
  it loads a `--fill-subtle` box of its size holds its place, and without an image it stays the chip
  "Attachment · name". It is loaded from Rust's `inline-image:` scheme ([ADR 0042](adr/0042-inline-images-load-through-an-app-scheme.md)) only once on screen (Issue view, preview, ⌥ Peek). A click shows it
  in Quick Look (cursor zoom-in); its context menu is the attachment tile's. The preview
  shows the same description and the latest three activity items.
- Rich text styles (`.jira-html` in `issue-view.css`): inline `code` and old `tt` as a mono chip on
  `--fill-subtle` (not inside `pre`), code blocks on `--fill-subtle`, struck text muted, quotes with a
  `--border-strong` bar, tables with `--border` lines and `--fill-subtle` headers, panels on the banner and status
  backgrounds by type, expands in a bordered box with a bold title, mentions in `--accent-text`, and status
  lozenges as small uppercase chips in the status and label colors.

**Remember my place:** each view keeps its opened and closed groups, scroll position, selected row and open
peek (List and Table, `views/list/useRowsPlace.ts`; the Board keeps its current card and scrolls back to it;
the scroll is written a moment after scrolling stops and on leaving, `views/useRememberedScroll.ts`;
`state/slices/places.ts`, saved by Rust in `places.json` a moment after the last change, at most 200
views), also across restarts. The Me chip is saved with the view and the text filter is never saved
([below](#toolbar)); while the Issue view, the preview or the Peek is open over a view, its drafts and text stay.

## Text and controls never wrap

Buttons, tabs, chips, pills, column headers and table cells stay on one line (`base.css`); long text
truncates with an ellipsis. When space runs out the flexible element shrinks (the text filter, a title), never
a button. Only descriptions, comment bodies, bookmark reasons and empty-state text may wrap.

## Menus

Every context menu and every pop-up button (Group, "…", "+ View", the pickers in Space settings) is one in-app
menu ([ADR 0034](adr/0034-menus-are-drawn-in-the-page.md)); only the app's menu bar is native.
`openMenu(entries, at, label)` or, for a right-click, `openContextMenu(e, entries, label)` from
`components/menu` asks the store (`state/slices/menu.ts`); `MenuHost`, mounted once per window, draws it.

- **Look**: the popover panel (12px radius, `--border`, `--shadow-popover`, 6px padding), rising in. Rows 30
  high, 7px radius, 13px `--text`, an optional 18px leading icon or tile, the label, an optional hint or key
  (12px `--text-muted`) and at the right a check (the current choice) or › (a submenu). The highlighted row takes
  `--hover-bg`, a disabled one `--text-faint`, a destructive one (Delete, Stop tracking) `--danger-text`. A caps
  caption (`popover__caps`) starts a group, a hairline separates groups.
- **Placement** (`components/menu/placement.ts`): a right-click's menu opens at the mouse, a button's under
  the button, a submenu beside its row; each flips to the other side when the window has no room and then
  shifts inside it, 6px from the edges.
- **Closing**: Escape, a press outside, a scroll, the window losing focus or a resize close it, and so does
  choosing a row. Focus goes back to where it was, unless the row moved it on (a rename field, a sheet). A
  second press on a menu's button closes it.
- **Keys**: ↑ ↓ (wrapping), Home and End move through the enabled rows, ↩ or Space runs one, typing a letter
  jumps to the next row starting with it; → opens a submenu (hovering does too), ← closes it. A menu opened
  from a button or a key starts on its first row. The command keys stay off while a menu has the focus.
- **Roles**: `menu` and `menuitem`; a choice is `menuitemradio` (`menuitemcheckbox` for an on/off one) with
  `aria-checked`; a caption's rows are a `group` named by it; disabled rows are `aria-disabled`.
- **Choices are inline**: a short set of choices is a captioned group with the current one checked (the view
  tab's Layout group), not a submenu; long ones (Sort by) stay a submenu.

## Tooltips

`components/TooltipHost.tsx`, mounted once per window, replaces the native tooltip: it takes any `title` over
as `data-tooltip` (an icon-only control keeps it as its `aria-label`) and shows a `--tooltip-bg` bubble below
the control (above when there's no room) after 450 ms, at once while moving between controls within 600 ms,
and on keyboard focus. A press, a key, the wheel or leaving hides it. Text cut off with an ellipsis (or
clamped to fewer lines) shows its whole text the same way, before any `title` around it; so every label that
can be cut (titles, names, epic chips, breadcrumbs) uses `text-overflow: ellipsis`, never a bare clip.

## Motion

Nothing jumps from one state to another (`styles/motion.css`). Tokens: `--motion-fast` 120ms (hover, press),
`--motion-base` 150ms (fades, chevrons, toggles, popover exit), `--motion-slow` 200ms (expand/collapse,
popover entry), `--ease-out`, `--ease-in-out`; no literal durations elsewhere. Under `prefers-reduced-motion`
movement is instant and fades are short.

- Show and hide fade: popovers rise in and fade out (`lib/motion.ts` `usePresence` keeps them mounted for the
  exit, inert), chevrons fade in with their group, a view's content fades in when it changes.
- Hover gives a short response: a `--hover-bg` tint, a `--border-strong` border on outlined controls,
  `--row-hover-bg` on rows.
- Expand and collapse animate: a List group's rows shrink and fade while the groups below glide up
  (`useGroupMotion`, which scales the virtual rows' heights), sidebar Spaces grow open (`components/Disclosure`).

## Toolbar

`src/shell/toolbar/`, 50 high under the tabs, one popover open at a time (`components/Popover.tsx`: under its
button, Escape and a press outside close it, focus goes back to the button).

- **Group: Status** (30 high, `--border`, 8 radius): a [menu](#menus) of the groupings (Status, Space, Client,
  Assignee, Epic, Release, Priority, Due date, None) with Sort by ▸ (Updated, Created, Priority, Key, Due date,
  Summary, Status; Ascending, Descending). Saves to the view.
- **Subtasks** (List and Table only, right after Columns; `SubtasksButton.tsx`): the subtask icon, labeled by
  the mode: icon only while hidden ("Subtasks: Hidden" to assistive tech, "Subtasks" as its tooltip), "Subtasks"
  in Hierarchy and "As items" in List, both in the active style. A [menu](#menus) captioned "Subtasks" offers
  Hide subtasks ("Matching issues only"), Show under parent ("Indented below their parent") and Show as items
  ("Separate rows, grouped on their own"), the current one checked; folded into the "…" menu it reads
  "Subtasks" with the mode as its hint. Saves to the view; the Board always hides
  subtasks and has no button.
- **Columns** ("Card fields" on a board): the *Columns for this view* popover (340 wide): a "Search fields" box
  (name, Jira id or hint; while it filters, the order can't be dragged), Jira fields (Status first: a text pill
  beside the row's own square), then "Custom fields · this project" with the field id as `cf_N`; a row per field
  with a drag handle (⋮⋮) to reorder and a 30 × 18 switch (`components/Toggle.tsx`). A long list scrolls. Each change saves to the view; "Use as Space default" sets
  the columns new views in the Space start with.
- **Filter** (badge: the filter list's own conditions plus the view's): the *Filter* popover (440 wide,
  right-aligned): "Show issues where" and the live count of open issues ("N issues match" on a filter list,
  with a condition or with the Me chip on, "N open issues" before any); inherited conditions locked on `--fill-subtle` with a lock,
  "from Space" or "this Space"; then two sections under small caps headings ([ADR 0033](adr/0033-views-can-hold-their-own-filter.md)):
  "All views of this list" (the filter list's own conditions, only on a filter list) and "This view" (the
  view's own conditions; Everything, a Space, Unread and Bookmarks show only this one). Each holds conditions as
  field / operator / value selects (value pickers from the cache, dates, days, text) with × and its own
  "+ Add condition". Inside a Space the Project field isn't offered (`projectFixed`). Changes apply to the rows
  at once as drafts and are not saved ([ADR 0038](adr/0038-filter-changes-are-drafts-until-the-view-is-saved.md)):
  the footer says "Changes show until you leave this view.", and while something differs from what's saved "Not
  saved: Save View keeps it." with Save View as a link. The live count follows the drafts and the Me chip (not
  the text filter, a search) and counts every matching issue, like the sidebar. On Everything and a Space "Save as filter list" turns the view's (drafted)
  conditions into a new list and opens it. A picked field's value opens a
  list with a search box and a checkbox per value (`ValuePicker`); every tick applies at once, and several values
  are stored as `in` / `not in` (shown as "is" / "is not").
- **Me** chip (on: `--accent-soft-*` pill with the `--avatar-1` avatar and "Me ×") and **Filter this view**
  (190 wide, filters by key or summary as you type). Both narrow only the current view. A view opens with the Me
  chip it saved (`view.only_mine`); a click changes it as a draft. The text is never saved.
- **Save View** (left of the Filter button, `SaveViewButton.tsx`, on `--accent` with `--text-on-accent`; pops in
  with a short fade and scale, instant under reduced motion) shows while the Me chip or any condition of either
  popover section differs from what the view and its list saved (`useViewDraft`). It saves the list's
  conditions (when changed), the view's and its Me chip, then the button goes. ⌘S and List → Save View run it
  (`view.save`); Revert View (`view.revert`, also the last row of the toolbar's "…" menu while there are drafts)
  drops the drafts. There is no other discard: going to another view tab, list or level drops the drafts, the
  text filter and an epic narrowing (`useDiscardDraftsOnLeave` in `shell/Window.tsx`); opening an issue (the
  Issue view, the preview, the Peek) and coming back, or switching windows, keeps them. The Filter badge counts
  the drafted conditions.
- When the bar is narrow, the text filter shrinks to 96 first; then Columns, Subtasks, Filter, Group and finally
  the Me chip move into a "…" menu (`fit.ts`, `useToolbarFit`), and come back as soon as they fit. Save View
  never moves; while it shows, it and the "…" button (holding Revert View) take their room first.

## Board view

`src/views/board/`, read-only (no drag and drop before writes), styled in `styles/board.css`.

- Columns scroll sideways (padding 6 20 20 24, gap 12), one per group of the view's Group by: per status in the
  Board order (also when ungrouped), else one per person, priority, epic, release, due bucket or Space as the List
  groups them, on the neutral To do tint with a gray pill holding the List's group mark (avatar, flag). An open column is
  300 wide, 12px radius, on its status tint (`--status-*-tint`), with the header pill, the count in
  `--status-*-count` and a collapse button; a collapsed one is a 48 × 300 strip with the header pill turned 90° (`status-pill--vertical`: the
  same 26px and padding, centered; owner, 2026-10-29) and the
  count, also turned 90° so a long number fits, and expands on click. Done starts collapsed; the choice is kept per view.
- Cards (gap 10, padding 12 14, 10px radius, `--card-border`, `--shadow-card`): the summary first, 14/400 like
  the List's titles (600 when changed), always wrapped and never truncated, with the inline labels after it;
  then the epic chip and label chips (wrapping); then avatars, release chip, comment count, priority flag and,
  in the bottom-right corner, the mono key (a 7px `--changed-dot` before it when changed). Card fields (the
  Columns button reads "Card fields") choose epic, labels, assignee, release and priority. Changed cards use
  `--changed-bg` and `--changed-border`. Hover never moves a card: it takes a `--border-strong` edge and
  `--shadow-hover`, and shows the row toolbar in the bottom-right corner over the key.
- A card opens the full Issue view (the mockup's behavior; List rows open the preview). A Board never shows the
  preview (`CurrentPeek`), whatever opened it.
- Each column virtualizes its cards (`BoardColumnCards`, measured heights), so a Space with thousands of
  issues mounts only the cards on screen; the keyboard's card scrolls into view.

## Table view

`src/views/table/`: the same rows as the list, every row 30 high (a group heading too: its 22px tab sits at
the row's bottom), with a sticky 34px header: Key, Summary (flexible),
Status, then the view's columns. A sortable header (Key, Summary, Status, Priority, Due) sorts on click and
again flips the direction (`aria-sort`, ↑/↓); a 6px handle at each header's right edge resizes the column
(48–800), saved on the view. Grouped tables keep the list's group headers; the table scrolls sideways when the
columns are wider than the window. Groups open and close with the List's animation (`useGroupMotion`). Rows
never grow: inline labels after the summary fill what the title leaves and are cut at the cell's edge (the
title truncates only when the cell is narrower than the title), and the Labels cell keeps its chips on
one line that scrolls sideways inside the cell (an overlay scrollbar, [below](#scrollbars)). Subtasks show as in
the List: the caret, the indent, the glyph and the count chip sit in the Summary cell. In Hierarchy every row
leaves the caret's room before its title (`table-row--tree`), so titles line up whether a row has children or
not, and the Summary header lines up with them; each level indents one caret's room (23) further.

## Tokens

Two palettes in `src/styles/palettes/` (`light.css`, `dark.css`), selected by `data-scheme` on `<html>`
(`lib/theme.ts`): Settings → Appearance is Automatic (follows macOS, the default), Light or Dark (ADR 0028).
Every palette defines the same tokens; the dark one is designed (macOS dark grays, status fills kept, tints at
low strength), and both pass the same contrast checks.

| Group | Tokens | Light |
|---|---|---|
| Surfaces | `--window-bg`, `--panel-bg`, `--panel-subtle-bg` (popover footers), `--fill-subtle` (key hints, tracks), `--fill-hover`, `--segmented-bg`, `--scrim` | `#F4F4F5`, white, `#FAFAFA`, `#F4F4F5`, `#EBEBEE`, `#EEEEF1`, black 38% |
| Text | `--text`, `--text-strong-body`, `--text-secondary`, `--text-tertiary`, `--text-muted`, `--text-faint` (decorative glyphs only), `--danger-text` (destructive menu rows), `--text-on-accent`, `--link`, `--link-hover` | zinc 900 → 400, white, blue 700, blue 900 |
| Lines | `--border` (controls, the tree guide), `--border-strong` (selects, dashed adds), `--hairline` (section edges), `--row-hairline` (between rows), `--card-border` | `#E4E4E7`, `#D4D4D8`, `#EDEDF0`, `#F1F1F3`, `#E9E9EC` |
| Accent | `--accent`, `--accent-text`, `--selection-bg`/`--selection-text`, `--accent-soft-bg`/`-border`/`-text`, `--button-primary-bg`/`-text`, `--spinner-track`, `--focus-ring` | blue 600, blue 700, `#E3EBFD`, … |
| Changed | `--changed-bg`, `--changed-border`, `--changed-chip-bg`/`-text`, `--changed-dot` | light blues |
| Status | `--status-{todo,progress,review,blocked,done}-{bg,fg,border,square,tint,count}`, `--status-todo-tab-{bg,fg}` | gray, blue, violet, red, green |
| Data | `--flag-*`, `--epic-{1..5}-{bg,fg}`, `--label-{1,2,neutral}-{bg,fg}`, `--avatar-{1..4}`, `--avatar-ring`, `--space-tile-{1..12}`, `--view-{list,board,table}` | |
| Messages | `--banner-{info,warn,error}-*`, `--notice-neutral-*`, `--pin-*`, `--pr-open-*`, `--sync-*`, `--progress-done` | |
| Shadows | `--shadow-{panel,popover,palette,peek,raised,card,knob,group-card,hover,hover-toolbar}` | |
| Hover, list | `--hover-bg`, `--row-hover-bg`, `--list-bg` | `#F4F4F5`, `#FAFAFB`, `#F7F7F8` |

The mockup's near-duplicates are merged: the palette's own grays map to the zinc scale, the palette's active
row uses `--selection-bg`, the PR chip uses the Checkout epic's purple, the warning banner text uses the amber
`#92400E`. The accent is fixed blue; there is no accent or scheme choice. Hover fills (not in the mockup) use
`--fill-hover`.

## Type

The system font (`-apple-system`, SF Pro) at a 14px root; SF Mono for issue keys, versions and field ids. No
bundled fonts. Weights 400, 500, 600, 700, 750 (titles) and 800 (tile letters).

| Role | Size / weight | Used for |
|---|---|---|
| Issue title | 30 / 750 | Issue view heading |
| Preview title | 22 / 750 | preview heading, page headings |
| Space title | 19 / 700 | content header |
| Card title, palette field | 17 / 700, 17 / 400 | state cards, the palette's query |
| Section | 16 / 700 | Description, Attachments, Child issues, empty states |
| Body | 14 / 400 (600 when strong) | rows, summaries, sidebar rows |
| Control | 13 / 400 (600 when strong) | toolbar buttons, popover rows, buttons, banners, breadcrumbs |
| Meta | 12 / 400 (600 for headers) | counts, column headers, timestamps |
| Status pill | 12 / 700 caps, 0.03em (11 and 10 in smaller pills) | status pills |
| Key | mono 12 (11 on cards) | issue keys |
| Chip, badge | 11 / 600–700 | change chips, counts badges |

## Metrics

Radii: 6 (tiles, chips, row hover), 7 (pills, small buttons, popover rows), 8 (sidebar rows, toolbar
buttons, group tabs), 9 (search, sync chip), 10 (cards, list group cards), 12 (panel, popovers, board
columns), 16 (palette), 3 (status squares), 50% (avatars, dots). Heights: sidebar search and home rows 34, Space
rows 36, filter-list rows 32; view tabs 42, toolbar 50 (controls 30), group tab 30, list row 44, board column
300 wide.

## Scrollbars

None are styled: every scroller keeps macOS's overlay scrollbars, shown only while scrolling. Any
`::-webkit-scrollbar` rule would make WebKit draw a permanent one. Every user-scrollable box is its own
stacking context (`isolation: isolate`, listed in `base.css`), so a nested scroller's bar never stays behind
while an outer box scrolls; menus and modals portal to `<body>` for the same reason. A scroller sits below its
titlebar row, so its bar never reaches the window buttons.

## Overlays inside a window

Toasts and dialogs are `--pop` with `--shadow` (no border). Toasts (`--radius`, bottom right) dismiss
themselves (success 2.5 s, info 4 s, errors 8 s, with an action 10 s), repeats count up (`×3`); an error reads
"Something went wrong" with Details. A view whose rows fail to load says so in one ("Could not load this view";
`meta.failure` on a query in `queries/queryClient.ts`). Sheets (`Modal`, `--radius-lg`, 20px padding, over `--scrim`) take Esc
first, a click on the scrim closes them, Tab cycles inside them, and closing returns the focus to where it was.
Everything that loses the owner's setup asks first in a `ConfirmSheet` (`components/ConfirmSheet.tsx`; the page
calls `useStore.getState().confirm({ title, text, action })` and `ConfirmHost` shows it): deleting a filter list
or a view, Stop tracking, Reset Cache. The question names what goes and what stays; Cancel and Esc keep it.

## Settings

A window of its own (about 880 × 720, at least 760 × 560) in Daybook's System Settings layout and code (there is
no separate Settings design; owner, 2026-10-28). Its colors are the `--settings-*` tokens of the same palette.

- **Page list** on `--settings-chrome` (190px): a drag strip for the window buttons and a search field that
  stay put (it narrows by page, group or section title), then rows under small group headings; the current
  page a `--settings-fill-strong` fill. Only the list scrolls. ⌘F focuses the search.
- **Page** on `--settings-bg`: the title in the strip beside the window buttons, then groups 24px apart. A
  change applies at once (fields on blur or ↵); only small forms whose fields belong together (the Jira site,
  the scope query) save with their own Save button. Esc (unless a field or a sheet takes it), ⌘W and the
  close button hide the window; it reopens on the page it showed. While another window is in front, switches,
  primary buttons and the page list turn gray (`window-blurred`), as macOS does.
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
    client and Move Up / Move Down buttons; clicking the row opens that Space's own page (`spaces/SpaceSettings`,
    ClickUp's Space settings; also the sidebar menu's Space Settings…, `app:settings:space:<projectId>`), with
    "‹ Spaces" back to the list. *General*: its fields (`SpaceFields` in `components/SpaceEditor.tsx`): Name
    (empty uses the project's), Icon and color (the look editor; a Space always has a color) and Client (empty means internal). *Layouts*: a switch per layout
    its views offer (one stays on), the default layout of new views and their grouping. *Statuses*: only the
    statuses this Space's issues are in, under a section per Jira category, with a search field; a row: the pill,
    "N issues", and a button (the style's square and "Automatic · X" or the style's name) with a menu of
    Automatic and the six styles, the chosen one checked. *Custom fields*: a row per field with a menu of
    Visible, Hidden in list (no column choice), Hidden in detail (not in the Issue view) and Hidden everywhere.
    Each field saves on its own with Rust's field error under it. The header's tile opens `LookPopover`.
    "Track projects under Projects to get Spaces." when none is tracked. The sidebar's "Client label…" item
    opens `SpacePopover` (300px, 12px radius, `--shadow-popover`, at the top left of the content panel) with the
    Client field; Esc, Done or a click outside close it.
  - **Sync**: *Scope* (the effective JQL in a monospaced field; Validate, Save, Reset to Default), *Intervals*
    (minutes in front and in the background, days to keep done issues) and *Cache* (the sync state line with
    Sync Now, one row per tracked project with its key, last full sync and "N issues · M open", then Reset
    Cache, confirmed in a sheet). It stays as the diagnostics view of the cache.
  - **General** (System): *Behavior* (read delay, where a click opens issues, default bookmark duration) and *About* (version, log folder).
  - **Appearance**: Automatic, Light or Dark. **Shortcuts**: every command and its key, rebindable.
  - **Status** (`components/settings/status/`, Daybook's Status page): `diagnostics_get` reports groups of
    rows, each a value with a tone icon (ok, warning, error, info) and maybe a bus command as a button: *Jira*
    (site, account, API token and its expiry), *Sync* (state, tracked projects, last full sync, cached
    issues) and *This Mac* (version, data and log folders with Show, the command-line tool's path). It reads
    again when the window comes to the front and after a sync.

## Keys

The app is fully keyboard controllable, and every action is also a menu item (owner, 2026-10-29). App-level
actions are Rust commands ([ADR 0012](adr/0012-commands-live-in-rust.md)); what the page does is in the page's
registry (`src/commands/`), and the menus reach it through the bus family `page:<id>`, which shows the main
window and emits `page_command` (`state/useAppEvents.ts` runs the command when it can). Components offer
their actions while on screen with `usePageAction(id, run)` (`commands/pageActions.ts`). Settings → Shortcuts
lists and rebinds the page's keys (conflicts are flagged); the menu shows the default keys. There are no
global shortcuts.

- **Menus** (`src-tauri/src/menu/`), in the Mac order: Taskboard, File, Edit, Go, List, Issue, View, Window, Help.
  File holds the core actions (New Filter List ⌘N, New View ⌘T; Refresh ⌘R, Full Sync ⇧⌘R; Reset Cache…,
  which opens Settings → Sync → Cache to confirm; Close Window ⌘W). Go moves around (Command Palette ⌘K,
  Everything ⇧⌘E, Unread ⇧⌘U, Bookmarks ⇧⌘P, Previous/Next Space ⌥⌘↑/↓, Previous/Next View ⌃⇧⇥/⌃⇥).
  List acts on the shown issues (Show as List / Board / Table ⌃⌘1–3; Group and Sort… ⌥⌘G, Columns… ⌥⌘C, Edit
  Filter… ⌥⌘F, Only My Issues ⇧⌘M, Filter This View ⌘F, Save View ⌘S, Revert View; Mark All as Read ⇧⌘A). Issue acts on the selected one
  (Open, Open or Close Preview, Open Epic, Open in Jira ⌘↩, Copy Key, Copy Link ⇧⌘C, Add or Remove Bookmark, Mark as Read or Unread).
  View is the window's appearance only (Toggle Sidebar ⌘\, Enter Full Screen; the debug build's Reload ⌥⌘R),
  never what the page shows. Help has Open Log Folder and Show Data Folder. Plain-letter keys stay off menu
  items so they never take a typed letter; Copy Key has no ⌘C there because Edit → Copy owns it.
- Bare keys never fire in text fields; there only ⌘ and ⌃ combinations and Escape do.
- ↵ and Space on a focused button or link activate it. `Tab` moves focus (sidebar, tabs, toolbar, content).
- A command has one binding; ⌘1–⌘9 open the nth view tab.
- **Issues** (`commands/issueKeys.ts`, registered by the shown List, Table, Board or Issue view; the List and the Table share `views/list/useRowKeys.ts`): J or ↓ and K or ↑
  move (on the Board through a column's cards, ← and → to the neighboring column; in a List or Table ← and → close and
  open the row's group; in Hierarchy ←/→ on a row with children collapse or expand its subtree first, at any
  level, and ← on a nested row selects its parent), Space opens or closes the preview (the Board opens the Issue view), ↩ opens the Issue
  view, E the epic, P adds or removes a bookmark (default duration), M marks read or unread, ⌘C copies the key (only with no text
  field focused and no text selected), ⌘⇧C the URL, ⌘↩ opens it in Jira, esc closes the preview or goes back.
- **A click on an issue key copies it** ("Copied NW-1042", `components/issue/CopyKey.tsx`): in List and Table rows,
  on Board cards, in Bookmarks, in the Issue view's header and title line, the preview's header and related, child
  and subtask rows. The click and a double-click stop at the key, so the row or card neither selects nor opens;
  the key shows a pointer, a `--hover-bg` tint and `--accent-text` on hover, and "Click to copy" as its tooltip.
  The ⌥ hover Peek takes no clicks and the palette's rows open on a click, so their keys don't copy.
- `app:settings:<page>` opens Settings at a page or section (Settings → Projects is `app:settings:projects`).

## Palette checks

`src/styles/colors.test.ts` keeps color literals inside `src/styles/palettes/` and every palette on the same
token set; `src/styles/contrast.test.ts` checks text tokens on their surfaces (4.5:1 body, 3:1 meta and large
text) and pill text on pill fills. With a dark palette, `scripts/dev/screenshot-looks.sh <dir>` captures the
window in the current macOS mode; switch the system appearance and run it again.
