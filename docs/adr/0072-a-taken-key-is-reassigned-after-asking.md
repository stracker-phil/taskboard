# 0072. A taken key is reassigned after asking; the menu's keys stay fixed for now

## Status

Accepted (2026-11-12). Inherited from the family catalog (keyboard-first; reference: Launcher ADRs 0032 and 0045).

## Context

Settings → Shortcuts saved any recorded key and then listed the clash below the list ("⌘⇧M is used by both …"),
so a command could silently lose the key it had, and the owner had to find and fix the other one. Reset showed
for every override, even one equal to the default. The menu bar's keys are fixed and listed for reference.

## Decision

- One binding per command where both work: recording a key (or resetting to a default) that another command holds
  asks first in a sheet ("⌘⇧M is used by “Only My Issues”. Reassign it to “…”?"). Reassign moves it in one save,
  leaving the other command without a key; Cancel keeps both (`commands/registry.ts` `holdersOf`, `withBinding`).
- A key of the menu bar is refused with a message; those keys can't be taken.
- Reset shows only while a command's key differs from its default; recording the default removes the override.
- The menu bar's keys stay fixed (owner, 2026-11-12: not a friction point today); the catalog's rebindable menu
  keys are deferred, not rejected.
- Rejected: saving and warning afterwards (the clash outlives the moment it was made); refusing a taken key (the
  owner has to unbind the other command first).

## Consequences

- No new conflicts can be stored; the conflict list remains for files from before.
