# 0032. Avatars come from Atlassian's avatar host, through Rust

## Status

Accepted (2026-10-29). Owner's request. Supersedes the "avatars are initials" line of
[0020](0020-jira-site-and-atlassian-gateway-are-the-hosts.md); keeps [0025](0025-all-jira-traffic-goes-through-one-allowlisted-gateway.md)
and [0027](0027-jira-html-is-sanitised-and-shows-no-remote-content.md).

## Context

Every person showed as initials on a color. The owner wants people's real Jira profile pictures. Jira's user
objects carry `avatarUrls`; uploaded pictures live on Atlassian's public CDN
(`avatar-management--avatars.<region>.prod.public.atl-paas.net`), Jira's default ones on Gravatar. Letting the
webview load them would make it reach the network, from any address a Jira user object names.

## Decision

- **One new host, by pattern.** `net::AvatarUrl` accepts only `https` on the default port, without credentials
  or fragment, with the host exactly `avatar-management--avatars.<region>.prod.public.atl-paas.net` and the
  region of lowercase letters, digits and dashes. Gravatar, the site, loopback and lookalikes are refused; the
  people they belong to keep their initials. The owner approved this host only.
- **Only Rust downloads.** `jira::gateway::AvatarClient` (over the one HTTP client, `gateway/http.rs`
  `Http::get_image`) sends a plain `GET` with no `Authorization` and no cookie, follows no redirect, and accepts
  only PNG, JPEG, GIF or WebP up to 512 KB whose `Content-Type` and magic bytes agree (never SVG).
- **The page gets a `data:` URL.** The sync stores the 48 px address in `cache.user.avatar_url` only when it
  passes the rule. `user_avatar(account_id)` answers from `cache.avatar` and fetches when the picture is
  missing, older than 7 days or the address changed; a failure is remembered for a day and the last picture
  stays. The CSP already allows `img-src data:`; the webview still loads nothing remote.
- **The demo keeps initials.** The demo people's addresses are on `avatar.example.com`, which the rule
  refuses, and loopback is never allowed for avatars, not even in the debug build.
- Rejected: loading the URLs in the webview (remote content, any host a user object names); proxying through
  the Jira site with the token (credentials for a public picture); Gravatar (a third party for a default
  picture we draw ourselves).

## Consequences

- What leaves the Mac grows by one host pattern, without credentials: `net::AvatarUrl` and `docs/adapting.md`.
- Pictures show offline once fetched; a changed picture shows after the next sync and the next ask.
- Atlassian's CDN learns which pictures the Mac asks for, as a browser on the Jira site would.
