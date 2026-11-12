# 0073. The accent follows the family: System or one of nine hues

## Status

Accepted (2026-11-12). Supersedes in part [0028](0028-automatic-light-or-dark-with-one-palette-each.md) (its "no
accent choice"). Inherited from the family catalog (appearance, option "one palette each, plus an accent";
reference: Launcher ADR 0042).

## Context

ADR 0028 ruled out an accent choice, but Taskboard gained one without a record: Settings → Appearance and each
Space's own accent, from nine hues of its own (blue, purple, pink, red, orange, yellow, green, teal, graphite),
blue by default. The sibling apps offer System (the macOS accent color) and the same nine hues, and fill the
current Settings page with the accent.

## Decision

- `settings.accent` is `system` (the default) or one of the family's hues: orange, gold, red, magenta, purple,
  indigo, blue, teal, graphite (`look::Accent`), changed by `accent_set`. System is the macOS accent color
  (`AccentColor`); each palette defines every hue and the shade it uses as text.
- A Space can still choose its own accent from the same set, shown while that Space is (Taskboard's addition).
- Stored values of the earlier hues read as their nearest one: pink as magenta, yellow as gold, green as teal; an
  unknown value reads as System.
- The current Settings page is filled with the accent; gray while another window is in front.
- Rejected: keeping Taskboard's own hues (the apps would differ for no reason); removing the accent (0028 as
  written; the owner uses it); following the macOS accent only.

## Consequences

- Owners who had blue keep blue; a new install follows macOS.
- `contrast.test.ts` checks the nine hues; System's text shade is clamped in lightness in each palette, since its
  value is known only at runtime.
