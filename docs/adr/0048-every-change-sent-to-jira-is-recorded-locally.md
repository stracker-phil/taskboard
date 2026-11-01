# 0048. Every change sent to Jira is recorded locally

## Status

Accepted (2026-11-01).

## Context

Taskboard changes company data in Jira on the owner's behalf: assignees, statuses, priorities, comments, web
links. Jira's own history shows these changes per issue, but there was no single place to see what this app
sent, or what Jira refused, without opening every issue.

## Decision

- **One local table.** `local.write_log` keeps every change Jira was asked for, with Jira's refusal, for 180 days
  (`writes::sent`). It is written where the request is sent, so it can't drift from what the app does.
- **What, not the content.** A record says what changed in plain words; a comment is recorded by its length,
  not its text, so no new copy of company content is kept.
- **Seen and exported in Settings.** Settings → Status lists the newest 50; Export… writes all of them as a CSV
  into Downloads (cells that start like a formula are defused).

## Consequences

- A change refused before it is sent (changes off, a switched-off kind) is not recorded: Jira never saw it.
- Resetting the cache keeps the record: it is in the local file.
