# 0028. Automatic, light or dark, with one palette each

## Status

Accepted (2026-10-29). Supersedes [0009](0009-a-scheme-per-system-mode.md) for Taskboard; plan step 20.

## Context

Daybook's appearance (0009) picks one of three looks per macOS mode, each built from separate sidebar and
content palettes, and follows the system with no app-level switch. Taskboard's mockup has one look (light
sidebar and content) and the spec asks for a dark mode in M3. Four palettes and per-mode looks would be
machinery without a second look to choose.

## Decision

- Two palettes, `src/styles/palettes/light.css` and `dark.css`, each defining every token for the whole window
  (sidebar, content, Settings). The dark one is designed for dark (macOS dark greys, readable status fills,
  low-strength tints), not inverted. Tests keep the token sets equal and check contrast in both.
- `settings.appearance` is `system` (the default), `light` or `dark`, changed by `appearance_set`; Settings →
  Appearance shows it as Automatic / Light / Dark. `lib/theme.ts` sets `data-scheme` on `<html>`: before the
  first render from macOS, then from the setting, following macOS changes while on Automatic.
- No accent choice: the mockup's blue is part of the status and change colours.

## Consequences

- A look different from the mockup would need a new palette and an Appearance option.
- Both windows repaint from `settings_changed`; the release and debug builds share the behaviour.
