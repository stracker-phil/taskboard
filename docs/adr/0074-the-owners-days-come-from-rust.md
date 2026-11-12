# 0074. The owner's days come from Rust; the page formats with Intl

## Status

Accepted (2026-11-12). Inherited from the family catalog (rust-formats-time-and-dates, the `Intl` variant;
reference: TimeTracker ADRs 0011 and 0025).

## Context

Timestamps are stored as UTC text with milliseconds. Rust already decides local days for filters, pins and the
list's "Yesterday" (`views::rows::format_moment`) in the owner's Jira time zone. The page formatted with the
built-in `Intl`, which the catalog allows, but also did its own day math: "today" and "yesterday" by comparing
`toDateString()` in the Mac's zone, which can disagree with the lists.

## Decision

- Rust reports the owner's day boundaries: `local_days` returns where yesterday, today and tomorrow start (UTC
  text) and the IANA zone (`time::local_days`, in the Jira account's zone, UTC when unknown). The page reads it
  through `useLocalDays()`, again once tomorrow has begun and on `settings_changed`.
- The page keeps no day math: `lib/time.ts` `dayOf` compares a timestamp with those boundaries;
  `formatLocalTime` and `shortWhen` use it, and format with `Intl` in the reported zone (`formatter`).
- Relative times that tick ("5 min ago") stay on the page; no date library.
- Rejected: Rust formatting every label (the status bar's "5 min ago" would need a call per minute); a date
  library (moment, date-fns, dayjs); the Mac's zone for some labels and the Jira zone for others.

## Consequences

- "Today" means the same day in the lists, the status bar, Settings and the Issue view.
- A new label that depends on the day uses `dayOf`, never its own comparison.
