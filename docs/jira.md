# Jira client

`src-tauri/src/jira/` talks to Jira Cloud's REST API v3. It reads only; writes arrive in M4. Everything goes
through `net::Http`, which reaches only the hosts of the connection
([ADR 0020](adr/0020-jira-site-and-atlassian-gateway-are-the-hosts.md)).

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
[ADR 0021](adr/0021-the-token-lives-in-the-keychain-through-secrets-rs.md)). `JiraClient` reads it for every
request and sends `Authorization: Basic base64(email:token)`. No command returns it; `net::redact` removes
`Authorization` values, `Basic`/`Bearer` tokens and the email from the lines the client logs.

## Endpoints

One method on `JiraClient` each (`jira/api.rs`); the answers are typed in `jira/dto/`.

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
| `check_jql` | `POST jql/parse?validation=strict` | Jira's messages; empty when valid |

`Issue.fields` stays raw JSON, as the cache stores it; accessors read what the sync needs
(`summary`, `project_id`, `status_id`, `updated`). `parse_jira_time` reads Jira's `2026-10-20T09:14:03.120+0200`.

**Search pages** come one after another: `SearchPages::next_page` sends the previous page's `nextPageToken`
and stops after a page with `isLast` (or without a token). A failed page is returned once, then the cursor
ends; the caller decides whether to repeat the job.

## Limits and retries

- `jira/limiter.rs` `Limiter`: a burst of 5, then 5 requests per second, shared by every client.
- `jira/retry.rs`: `429` is retried, and `503` only with `Retry-After`; at most `MAX_RETRIES` (4). A
  `Retry-After` in seconds or as an HTTP date is honoured (capped at 15 minutes); without one, the wait is
  exponential with full jitter (base 1 s, cap 30 s). `RateLimit-Reason` is logged. Writes will pass
  `Retry::Never`: Jira may have applied the first attempt.

## Errors

`JiraClient` maps answers to `AppError` ([ADR 0003](adr/0003-errors-as-kinds.md)):

| Answer | Kind |
|---|---|
| no token in the Keychain | `not_connected` |
| 401, 403 | `auth` |
| no connection, DNS failure, timeout | `offline` |
| 400 on `search/jql`, `approximate-count`, `jql/parse` | `invalid_jql`, with Jira's messages |
| 404 | `not_found` |
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
  changes: `edit_issue`, `move_issue` (same id, new key), `delete_issue`, `revoke_access`.
- `requests()` and `paths()` return what it was asked, with the `Authorization` header and the JSON body.

The demo site (`test_support/demo/`) is made up and matches the mockup; `synthetic(n)` builds `n` issues on the
same site for performance tests.
