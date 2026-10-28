# 0018. Reads go through TanStack Query, invalidated by events

## Status

Accepted (2026-10-28).

## Context

Daybook's page keeps everything it reads in zustand and re-reads on events by hand. Taskboard reads many
views of one local cache (lists, issues, counts, activity) that change in the background whenever a sync
lands. Each view needs caching, de-duplication and a precise refresh, and a sync that changed nothing must
not re-render the list.

## Decision

- Data the page reads from Rust goes through TanStack Query hooks in `src/queries/`, which call `api`.
  The client never retries and never goes stale on its own (`staleTime: Infinity`): data changes only when
  Rust says so.
- Rust announces changes as events; `src/queries/useEvents.ts` maps each event to the query keys it
  invalidates.
- UI-only state (selection, the sidebar, open popovers) stays in zustand (`src/state/`).

## Consequences

- Structural sharing keeps unchanged rows identical, so a no-op refresh doesn't re-render them.
- Tests mock `api` and render with a fresh query client.
