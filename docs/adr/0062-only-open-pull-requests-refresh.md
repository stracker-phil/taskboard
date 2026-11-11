# 0062. Only open and draft pull requests refresh

## Status

Accepted (2026-11-11). Owner's request. Amends [0061](0061-pull-request-status-comes-from-the-gh-cli.md): "repeats
every 5 minutes while a pull request isn't merged".

## Context

A closed pull request was asked about every 5 minutes like an open one, though closing, like merging, is the
end of its life. The owner wants a refresh only when it can show something new.

## Decision

An opened issue asks `gh` again every 5 minutes only for pull requests that are open or drafts, and for those
whose lookup failed. Merged and closed ones are fetched when the issue is opened, at most once per 5 minutes,
and then left alone (`pull_requests::due_at`).

## Consequences

- A closed pull request that is reopened shows as closed until the issue is opened again.
