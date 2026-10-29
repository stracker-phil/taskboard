# Jira client

`src-tauri/src/jira/` talks to Jira Cloud's REST API v3. It reads only; writes arrive in M4. Every request goes
through `jira/gateway/`, which sends only the operations on its allowlist
([ADR 0025](adr/0025-all-jira-traffic-goes-through-one-allowlisted-gateway.md)) to the hosts of the connection
([ADR 0020](adr/0020-jira-site-and-atlassian-gateway-are-the-hosts.md)).

## Setup

Settings → Connection (`connection/`):

1. Site URL and email, an API token from id.atlassian.com (Account settings → Security → API tokens), the
   token kind and the token's expiry date. **Save** stores the token in the Keychain under the email; the
   page never gets it back, only `hasToken`. Changing the email moves the token to the new account.
2. **Test Connection** asks `/myself` and stores the account id, name and time zone. A scoped token first
   needs the site's cloud id (`/_edge/tenant_info`); when the site doesn't answer it (`not_found`), the page
   asks for it.
3. **Settings → Projects → Refresh** loads every project into `cache.space`; **Track** creates the project's Space
   (`local.space`). Untracking keeps the Space's settings.

Scopes a scoped token needs for v1: `read:jira-work`, `read:jira-user` (writes later add `write:jira-work`).
The expiry date warns 14 days ahead (`connection::token_expiry`).

Settings → Sync sets the scope JQL (validated by Jira's `jql/parse` before it is saved; empty = the default
over the tracked projects), the intervals and the days done issues stay, and resets the cache.

In the debug build the fake Jira runs at `http://127.0.0.1:47470` (`dev/fake.rs` `FAKE_JIRA_PORT`) with the demo site
dated to today; any email and token connect to it.

## Site and tokens

`jira/site.rs` `Site` holds the site URL, the email, the token kind and the cloud id.

- `normalize_site_url` accepts `example`, `example.atlassian.net` or any link on the site and returns
  `https://example.atlassian.net/`. Anything that isn't `https://<name>.atlassian.net` fails as
  `InvalidField` on `siteUrl`. Debug builds and tests also accept the fake Jira's `http://127.0.0.1:<port>/`.
- **Classic** (unscoped) tokens call `https://<site>.atlassian.net/rest/api/3/`. **Scoped** tokens call
  `https://api.atlassian.com/ex/jira/<cloudId>/rest/api/3/`; the cloud id comes from
  `https://<site>.atlassian.net/_edge/tenant_info`, asked without credentials (`JiraClient::tenant_info`).
- `browse_url(key)` is the issue's page on the site.

The token lives in the Keychain (`secrets.rs`, service `APP_NAME`, account = the email;
[ADR 0021](adr/0021-the-token-lives-in-the-keychain-through-secrets-rs.md)); the debug build keeps the demo token
in memory instead ([ADR 0026](adr/0026-the-debug-build-keeps-its-demo-token-in-memory.md)). `JiraClient` reads it for every
request and sends `Authorization: Basic base64(email:token)` (never to `_edge/tenant_info`). No command returns it; `net::redact` removes
`Authorization` values, `Basic`/`Bearer` tokens and the email from the lines the client logs.

## Gateway and allowlist

`Gateway::send(Endpoint)` (`jira/gateway/mod.rs`) is the only way a request leaves the app:

1. `endpoint.rs` `Endpoint::plan` turns one allowlisted operation into method, URL, body and whether it is
   authenticated. Path segments are pushed one by one, so a value can't add a segment, a query or a fragment.
2. `params.rs` validates every value first: `ProjectKey` (`[A-Z][A-Z0-9_]*`), `IssueRef` (digits, or a key),
   `IssueRefs` (1 to `BULK_FETCH_MAX`), `Jql` (non-empty, at most 10,000 characters, body only), `FieldIds`
   (explicit ids, never `*all`; only `FieldUsage` asks for `*all`, from a fixed list in `endpoint.rs`), `Expand` (`renderedFields`), `PageToken`. A refused value fails as
   `invalid_input` and nothing is sent.
3. `guard.rs` `check` compares the planned request with `RULES` (method, path under the REST base, allowed
   query keys) and the unauthenticated `GET /_edge/tenant_info`. Anything else, a fragment, credentials in the
   URL, a body on a GET, or a write while `WRITES_ENABLED` is `false`, fails as `internal` ("Taskboard doesn't
   call …").
4. `http.rs` `Http` sends it: listed hosts only, no redirects (a 3xx fails), JSON only on success, answers capped
   at `MAX_ANSWER_BYTES` (32 MB). It is the only place `reqwest` may be used (`src-tauri/clippy.toml`).

**Profile pictures** ([ADR 0032](adr/0032-avatars-come-from-atlassians-avatar-host-through-rust.md)) are the one
request that isn't an API call. `gateway/avatar.rs` `AvatarClient::fetch` takes only a `net::AvatarUrl`:
`https`, the default port, no credentials or fragment, and the host exactly
`avatar-management--avatars.<region>.prod.public.atl-paas.net` with a region of lowercase letters, digits and
dashes. `Http::get_image` checks the rule again before the wire and sends a plain `GET` with no
`Authorization` and no cookie (the host is public; the token and email never go there). A redirect, a non-2xx,
an answer over `MAX_AVATAR_BYTES` (512 KB), or one whose `Content-Type` and magic bytes don't both say PNG,
JPEG, GIF or WebP (never SVG) fails; `avatars::user_avatar` logs it without the address and shows initials.
Jira's default pictures (Gravatar, `secure.gravatar.com`) are never fetched. The sync stores only addresses
that pass the rule (`user.avatar_url`, the 48 px one).

**Not API traffic:** the webview has no HTTP, shell or opener permission (`capabilities/default.json`). Links
("Open in Jira", links in descriptions) go through the `open_url` command, which hands an `https` URL without
credentials to the default browser; the app sends nothing itself and the token never leaves the gateway.

**Adding an endpoint:** a variant in `Endpoint` with its plan, a row in `RULES`, a method in `api.rs`, tests for
the exact request and its refusals, and a row in the table below. A write also flips `Access::Write` for that
row and needs the owner's go-ahead (M4).

## Endpoints

One method on `JiraClient` each (`jira/api.rs`), one `Endpoint` each; the answers are typed in `jira/dto/`.

| Method | Endpoint | Answer |
|---|---|---|
| `myself` | `GET myself` | `Myself`: account id, name, time zone |
| `tenant_info` | `GET /_edge/tenant_info` on the site | `TenantInfo`: cloud id |
| `fields` | `GET field` | `Field` with its schema |
| `projects` | `GET project/search`, every page by `startAt` | `Project` |
| `statuses` | `GET status` | `Status` with its category key |
| `versions` | `GET project/{key}/versions` | `Version` |
| `search_jql` | `POST search/jql`, `maxResults=100`, explicit fields | `SearchPages`, a cursor of `SearchPage` |
| `bulk_fetch` | `POST issue/bulkfetch`, up to `BULK_FETCH_MAX` ids | `BulkFetch`: issues and the ids Jira refused |
| `approximate_count` | `POST search/approximate-count` | a count |
| `comments` | `GET issue/{key}/comment`, every page, `expand=renderedBody` | `Comment` with ADF and HTML |
| `remote_links` | `GET issue/{id}/remotelink` | `RemoteLink`: web links (url, title, application, relationship; the icon is read but never loaded) |
| `changelog` | `GET issue/{id}/changelog`, every page by `startAt` (`maxResults=100`; only these two query keys are allowed) | `Changelog`, oldest first: id, author, time and the changed fields (`ChangeItem`: field name and id, `from`/`to` ids, `fromString`/`toString` texts). Asked when an issue is opened, at most every 5 minutes ([ADR 0031](adr/0031-issue-history-comes-from-the-changelog-on-demand.md)) |
| `check_jql` | `POST jql/parse?validation=strict` | Jira's messages; empty when valid |
| `used_custom_fields` | `POST search/jql`: `project = "KEY" ORDER BY updated DESC`, one page of 100, `*all` without the large system fields | the custom field ids with a value |

`Issue.fields` stays raw JSON, as the cache stores it; accessors read what the sync needs
(`summary`, `project_id`, `status_id`, `updated`). Searches ask for explicit fields (`sync/full.rs`
`SEARCH_FIELDS`): the mapped ones, `comment`, `description`, `issuelinks` and `attachment` (id, file name, size,
MIME type, created, author and the `content` address; `dto::Attachment`), plus the shown custom fields.
`issues::attachments_of` reads the attachments from the stored issue; `issue_get` keeps an attachment's address
only when it is `https` on the connected site's host and port (`issues::is_on_site`), and the page opens it with
`open_url`. The app never fetches attachment bytes. `parse_jira_time` reads Jira's `2026-10-20T09:14:03.120+0200`.

**Search pages** come one after another: `SearchPages::next_page` sends the previous page's `nextPageToken`
and stops after a page with `isLast` (or without a token). A failed page is returned once, then the cursor
ends; the caller decides whether to repeat the job.

## Limits and retries

- `jira/limiter.rs` `Limiter`: a burst of 5, then 5 requests per second, shared by every client.
- `jira/retry.rs`: `429` is retried, and `503` only with `Retry-After`; at most `MAX_RETRIES` (4). A
  `Retry-After` in seconds or as an HTTP date is honored (capped at 15 minutes); without one, the wait is
  exponential with full jitter (base 1 s, cap 30 s). `RateLimit-Reason` is logged. Writes will pass
  `Retry::Never`: Jira may have applied the first attempt.

## Errors

The gateway maps answers to `AppError` ([ADR 0003](adr/0003-errors-as-kinds.md)):

| Answer | Kind |
|---|---|
| no token in the Keychain | `not_connected` |
| 401, 403 | `auth` |
| no connection, DNS failure, timeout | `offline` |
| 400 on `search/jql`, `approximate-count`, `jql/parse` | `invalid_jql`, with Jira's messages |
| 404 | `not_found` |
| a redirect, a success that isn't JSON, an answer over 32 MB | `internal` |
| a request outside the allowlist | `internal` ("Taskboard doesn't call …"), nothing sent |
| a value that fails validation (key, JQL, field id) | `invalid_input`, nothing sent |
| anything else, or an answer that doesn't decode | `internal`, with the status |

## JQL

`jira/jql.rs` writes every query:

- `default_scope(keys, days)`: `project in ("NW", "LUM") AND (statusCategory != Done OR updated >= -90d)`.
- `and_project`, `and_updated_since`, `order_by_updated` (`ORDER BY updated ASC`), `quote`.
- `date_literal(instant, zone)`: `yyyy-MM-dd HH:mm` in the Jira user's time zone from `/myself`, rounded down
  to the minute. Jira reads literals in that zone; UTC or the Mac's zone would skip or repeat changes.

## The fake Jira

`test_support/fake_jira/` ([ADR 0022](adr/0022-the-debug-build-syncs-from-a-built-in-fake-jira.md)) serves a
`test_support::demo::JiraData` on `127.0.0.1:<random port>`, for both token kinds' paths. It accepts any token
but answers `401` without one.

- `FakeJira::start(data, now)`, or `start_demo()` for the demo site at `DEMO_NOW`. `site(kind)` and
  `client(kind)` connect the demo user to it (`DEMO_TOKEN`, a limiter tests don't notice).
- Its JQL reader (`fake_jira/jql.rs`) knows `AND`, `OR`, parentheses and the clauses Taskboard writes:
  `project`, `id`, `statusCategory`, `updated` (relative `-90d` against the fake's clock, or a literal in the
  user's zone). A clause on another field matches every issue; a query it can't parse matches every issue in
  a search and is an error in `jql/parse`.
- Knobs: `set_page_size`, `cap_max_results`, `fail_next(count, status, retry_after)`, `answer_next`,
  `set_latency`, `go_offline` / `go_online` (the port refuses connections), `advance` (the clock), and data
  changes: `edit_issue`, `move_issue` (same id, new key), `delete_issue`, `revoke_access`, `set_remote_links`, `set_changelog`. `set_page_size` pages the changelog too.
- `requests()` and `paths()` return what it was asked, with the `Authorization` header and the JSON body.

The demo site (`test_support/demo/`) is made up and matches the mockup (`demo/web_links.rs` gives a few issues
pull request and document links, `demo/attachments.rs` a few issues' files, `demo/history.rs` a few issues' changelogs); `synthetic(n)` builds `n` issues on the
same site for performance tests.
