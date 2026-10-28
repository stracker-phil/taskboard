# 0009. Appearance follows macOS, with one of three schemes per mode

## Status

Accepted (2026-10-28). Inherited from Daybook ADRs 0009 and 0010 (TimeTracker ADRs 0021 and 0022), as they
stand after their amendments.

## Context

A gallery of editor themes makes an app look like a code editor; the apps this one sits beside (Calendar,
Reminders, Things, Daybook, TimeTracker) offer a few designed looks and follow the system appearance.

## Decision

There is no app-level light/dark switch: the app follows macOS. Settings → Appearance picks a scheme per
macOS mode (`lightScheme`, `darkScheme`) and one accent for both. A scheme is one of three looks: Default
(dark sidebar, light content), Light (all light) or Dark (all dark). The colors come from four palettes, one
per surface and tone: `content-light`, `content-dark`, `sidebar-light`, `sidebar-dark`. `lib/theme.ts`
resolves the current mode's scheme and sets `data-content`, `data-sidebar` and `data-accent` on `<html>`.
The accent is System (the macOS accent color through WebKit's `AccentColor`, the default) or one of nine of
Apple's system hues.

## Consequences

- Adding a look means a new scheme value and, at most, another palette file.
- Contrast is checked per palette, not per look.
- Where a scheme's sidebar tone matches the macOS mode, the window's sidebar material shows through.
