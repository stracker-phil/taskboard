# 0007. Color palettes are complete token sets; no color literal outside a palette

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0007 (TimeTracker ADR 0020).

## Context

Colors scattered over component styles make a dark mode or a new look a hunt through every file, and a token
forgotten in one look only fails visually.

## Decision

Every color is a CSS custom property defined in a palette file in `src/styles/palettes/` (one file per
surface and tone). Palettes of a kind define the same tokens. Everything else uses `var(--token)` or derives
from tokens with `color-mix()` and relative OKLCH colors in `theme.css`. `src/styles/colors.test.ts` fails on
a color literal outside the palettes and on a token missing from one palette of a kind;
`src/styles/contrast.test.ts` checks the text contrast of each palette.

## Consequences

- A new look is a palette file, not a refactor.
- Data colors (projects, labels, priorities) are token names, never hex values in data.
