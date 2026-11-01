# 0047. The bookmarks bar is the only top bar

## Status

Accepted (2026-11-01). Owner's request. Amends the Window rule in `docs/ux.md` ("There is no top bar").

## Context

The window has no top bar: the sidebar holds the traffic lights and everything to navigate. The owner wants their
bookmarks one click away, like a browser's favorites bar, without the sidebar's Bookmarks row in between, and
with their own names and order.

## Decision

- **An opt-in bar.** View → Show Bookmarks Bar (⇧⌘B) or the Bookmarks row's context menu moves the bookmarks
  into a 36px row on the window background above the content panel; "Show Bookmarks in Sidebar" moves them back.
  The choice is a Rust-owned mode (`modes.json` `bookmarksBar`), so the View menu shows it checked.
- **Native to the frame.** The bar sits on `--window-bg`, beside the sidebar's traffic-light strip, or in the
  hidden sidebar's strip after the traffic lights. Its empty space drags the window. Items are flat like sidebar
  rows.
- **Nothing jumps.** The bar's slot grows from 0 so the panel glides; the sidebar's Home rows glide when the
  bookmarks leave or return. Items fold into » by the window's width only.
- **Own names and order** come from the bookmarks themselves (`pin.title`, `pin.position`), so the bar, the
  Bookmarks page and the sidebar always agree.

## Consequences

- With the bar on, the Bookmarks page is reached through » → All Bookmarks or ⇧⌘P.
- Folded bookmarks can't be dragged; the order is set among the visible ones or on the Bookmarks page.
