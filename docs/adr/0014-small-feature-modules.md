# 0014. Small feature modules with narrow seams

## Status

Accepted (2026-10-28). Distilled from DataBee ADRs 0010, 0011 and 0013, TimeTracker ADR 0008 and the module
layout of all three apps.

## Context

The three earlier apps stayed easy to change where each feature was a folder that owned its logic, its
commands and its tests, and got hard to change where files grew past a few hundred lines or one module
reached into another's state.

## Decision

- **By feature, not by layer.** A feature is a folder in `src-tauri/src/<feature>/` (its domain types and
  pure logic, a `commands.rs` that only plumbs arguments, its `mod tests`) and, when it has UI, a folder in
  `src/`. The core (startup, state, store, IPC, windows) never imports a feature; a feature registers itself
  (handlers, commands, settings pages) in one install list (`handlers.rs`, `src/modules.ts`). Removing a
  feature is deleting its folders and its lines.
- **Pure cores.** Rules are pure functions over values; I/O sits at the edge. Most tests need no files.
- **Ports only where a swap is real.** A trait (repository, clock, other app, OS API) exists when there are
  two implementations today, usually the real one and a test double. Platform code (objc2, AppKit) stays in
  one module behind such a port. No speculative interfaces, builders or registries with one case.
- **Small files.** Over ~400 lines of production code, or hard to follow, a file is split into focused
  modules in the same step, without asking.

## Consequences

- Each feature can be read, tested and replaced on its own.
- A few more files and an install list to keep in step.
