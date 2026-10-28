# 0020. The app talks to the configured Jira site and api.atlassian.com

## Status

Accepted (2026-10-28). Amends [0013](0013-network-only-to-listed-hosts.md): the host list is per connection,
not a constant.

## Context

ADR 0013 lists every host in a constant `HOSTS`. Taskboard's main host is the owner's own Jira site
(`<site>.atlassian.net`), known only at runtime. Scoped API tokens call a second host, the Atlassian gateway
`api.atlassian.com`, with the site's cloud id in the path. Tests and the debug build need a Jira that holds
demo data and never reaches the internet.

## Decision

- `net::Http` takes its host list when it is built. `JiraClient` builds it from `Site::hosts()`: the configured
  site and `api.atlassian.com`. Nothing else is reachable, and only over `https`.
- The site URL must be a Jira Cloud address (`https://<name>.atlassian.net`); `jira::site` refuses others.
- Plain `http` to `127.0.0.1` is allowed only under `cfg(any(test, debug_assertions))`, for the built-in fake
  Jira ([ADR 0022](0022-the-debug-build-syncs-from-a-built-in-fake-jira.md)). Release builds can't reach it.
- The webview loads nothing remote: avatars are initials, images in Jira's HTML become links.
- Rejected: an HTTP plugin with a capability scope in the webview (the UI would hold the token); a fixed host
  list (the site is the owner's).

## Consequences

- What leaves the Mac is still reviewable in one place: `Site::hosts()` and `net::Http::allows`.
- `docs/adapting.md` lists the hosts.
