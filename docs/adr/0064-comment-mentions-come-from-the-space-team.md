# 0064. Comment mentions come from the Space's team

## Status

Accepted (2026-11-11). Owner's request.

## Context

Tagging someone in a comment ("@Marta please review") meant switching to Jira's web page. Jira offers a people
search, but the owner wants to pick only people from the issue's Space team as Taskboard knows it (Settings →
Team).

## Decision

Typing `@` and a few letters in the comment box lists the Space's active team without the connected account
(`writes::mentionable`, `team::active`), filtered by the start of the name or of any word in it. Picking someone
inserts `@Full Name`. Rust resolves names, not the page: `adf::from_markdown` turns `@Full Name` of anyone on that
list into an ADF mention when the comment is sent, so Jira notifies them. The `@` must start a word, the name
must end at a word boundary, the case must match and the longest name wins. Nothing new is asked from Jira, and
no new scope is needed.

## Consequences

- Someone outside the Space's team can't be mentioned from Taskboard; their name stays text.
- A typed full name works without the list; a name edited after picking stays text.
- Two team members with the same name: the mention goes to one of them.
