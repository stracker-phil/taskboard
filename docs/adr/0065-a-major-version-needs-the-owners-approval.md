# 0065. A major version needs the owner's approval

## Status

Accepted (2026-11-11). Supersedes in part 0054 (the major bump). Inherited from the family catalog
(release-versions; reference: TimeTracker ADR 0047).

## Context

ADR 0054 made a change that breaks existing use (CLI, links, stored data without migration, a removed feature)
a major bump, chosen from `git log v<last>..HEAD`. Taskboard is a personal tool with one user, who also writes
the scripts that use it, so a major version signals breakage to consumers it doesn't have.

## Decision

- **Changes that break existing use are minor**, like new features and visible behavior.
- **A major bump needs the owner's explicit approval**, asked before releasing; never chosen from the log alone.
- Patch stays as it was: fixes, polish, performance and docs only.
- Rejected: semver's strict major for breaking changes (it signals breakage to other consumers).

## Consequences

- Versions grow by minor and patch; a major marks what the owner calls one.
- `scripts/release.sh` is unchanged: it takes the bump as an argument.
