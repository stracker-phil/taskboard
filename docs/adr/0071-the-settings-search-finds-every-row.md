# 0071. The Settings search finds every row

## Status

Accepted (2026-11-12). Inherited from the family catalog (settings-window; reference: Launcher ADRs 0045 and
0046). Extends [0010](0010-native-macos-look.md) for the Settings window.

## Context

The search in the Settings window narrowed the page list by page label, nav group and the group titles a page
registered by hand (`sections`). A setting was found only when its group's title matched, one hit per page, and
nothing checked that the hand-kept titles matched the pages.

## Decision

- Each page registers its rows (`registerSettingsPage` `rows`): `id` is the `anchor` of a `Group` or `Row`,
  `title` its heading or label as shown, `keywords` for synonyms, `under` the row to reveal while the row itself
  is hidden. Pages may add `keywords` too. Rows that show data (a project, a status, a person, a diagnostic
  value) have no anchor and no entry.
- Typing replaces the page list with one list of matching pages and rows, ranked like the palette
  (`lib/rank.ts`); a row hit shows the page, then the row's title. ↵ or a click opens the page scrolled to the
  row and washes it once in the accent (`--motion-highlight`), again on every pick; the `under` row is revealed
  only while the row isn't on screen. `open_settings` and `app:settings:<id>` take a row id too.
- A test renders every page and fails on an anchor without an entry, an entry without its row, or a title that
  isn't the label (`components/settings/searchIndex.test.tsx`).
- No headline repeats the one above it: groups that repeated their page's title lost their heading.
- Rejected: searching group titles only (one hit per page hides the rest; the hand-kept list drifted); reading
  rows from the rendered pages at runtime (a page renders only when shown); searching descriptions (keywords
  hold the synonyms).

## Consequences

- A new setting row needs an anchor and an entry in its page's `rows` (`pages.tsx`), or the test fails.
- A Space's own settings are found by the Spaces page's keywords; their groups show only inside one Space.
