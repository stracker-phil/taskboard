# 0010. The app looks native and light

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0011 (TimeTracker ADR 0024) and the window look of Daybook
ADRs 0026 and 0028, the most mature of the three apps.

## Context

TimeTracker went through cards with borders, filled web buttons, chips and repeated toolbars before arriving
at a look that matches the macOS apps beside it; Daybook took that result and refined it (lighter hairlines,
overlay scrollbars, a calmer Settings window). Taskboard starts from Daybook's state, not from a web look.

## Decision

- **Type:** the system font (SF Pro through `-apple-system`) at the macOS text styles (13px body, 11px
  secondary, 15px titles), tabular figures. Mono (SF Mono) only for machine text such as paths, URLs, ids and
  commands. No bundled fonts.
- **Surfaces:** neutral greys, no tint, no outlined cards. Content is the window background. Grouped content
  is a System Settings inset group: a translucent fill, a 10–12px radius, no border, very light hairlines.
- **Window chrome:** a unified toolbar beside the window buttons (`titleBarStyle: Overlay`), empty parts drag
  the window. No status bar.
- **Scrollbars:** none styled. Every scroller keeps macOS's overlay scrollbars, shown only while scrolling.
- **Controls:** AppKit metrics and looks; the accent marks interactive things only (keys, links, focus,
  selection); a sheet's default button is the only accent-filled button.
- **Settings** is a window of its own in the System Settings layout: a searchable page list, pages of inset
  groups, every change applies at once.

## Consequences

- `docs/ux.md` holds the token and control spec.
- No `::-webkit-scrollbar` rule anywhere: one would make WebKit draw a permanent scrollbar.
- Feature UIs reuse the shared controls before adding new ones.
