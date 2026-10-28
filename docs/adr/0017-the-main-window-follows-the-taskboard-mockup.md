# 0017. The main window follows the Taskboard mockup

## Status

Accepted (2026-10-28). Supersedes [0010](0010-native-macos-look.md) for the main window and
[0009](0009-a-scheme-per-system-mode.md)'s three schemes and accent choice.

## Context

The owner designed Taskboard's main window as a mockup in the style of ClickUp: a light grey sidebar, a white
content panel inset into the window, 14px body text, a fixed blue accent, filled status pills and coloured
tiles. The spec says to build it pixel-close and not to redesign it. Daybook's look (13px System Settings
type, inset groups, system accent, three schemes) does not produce that window.

## Decision

- The main window is built to the mockup: tokens, type scale and metrics are in `docs/ux.md`.
- Colours still come only from palette files (ADR 0007). There is one palette per macOS mode, `light.css` now
  and `dark.css` later, selected by `data-scheme` on `<html>`. No scheme choice per mode, no accent choice:
  the accent is the mockup's blue.
- Backgrounds are solid; no window vibrancy.
- Overlay scrollbars, the overlay title bar with the native traffic lights inside the sidebar, and empty
  chrome that drags the window stay as in ADR 0010.
- The Settings window keeps Daybook's System Settings layout until the owner's own Settings design replaces
  it; it draws its colours from the same palette.

## Consequences

- The contrast and token-set tests run per palette (`src/styles/colors.test.ts`, `contrast.test.ts`).
- A dark palette must define every token of the light one.
