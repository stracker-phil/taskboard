# Taskboard technical documentation

These docs are for developers and coding agents working on the code. For what the app does and how to
use it, see the [project README](../README.md).

| Page | Covers |
|---|---|
| [architecture.md](architecture.md) | Design rules, process model, data, repository map |
| [ux.md](ux.md) | Design language: the mockup's window, tokens, type, metrics, Settings, keys |
| [data.md](data.md) | The two SQLite files, their tables, the write path, migration rules |
| [sync.md](sync.md) | The sync engine: jobs, states, storing pages, change records, events |
| [notifications.md](notifications.md) | macOS notifications after a change sync: when, what, the buttons |
| [scopes.md](scopes.md) | The API token scopes the app needs and what each one is for |
| [links.md](links.md) | `taskboard://` links: what each opens, Copy Taskboard Link, the command allowlist |
| [writes.md](writes.md) | Changes to Jira: the two locks, pending writes, the flow, "Change back to …" |
| [jira.md](jira.md) | The Jira client: site and tokens, endpoints, limits, retries, errors, JQL, the fake Jira |
| [testing.md](testing.md) | Unit and integration tests, the test agents, the demo seed, scripts that drive the app, signing |
| [adapting.md](adapting.md) | Identity (names, ids, paths), platform-specific code, conventions to keep |
| [adr/](adr/README.md) | Architecture decision records: why the design is the way it is |

Pages for the other layers (`backend.md`, `frontend.md`) and for each feature are added with the code
they describe, in the same commit.

Suggested reading order: architecture, then the layer you are changing, then adapting.

The rules in [`.claude/rules/`](../.claude/rules/) are binding for changes:
- [`testing.md`](../.claude/rules/testing.md)
- [`commits.md`](../.claude/rules/commits.md)
- [`demo-data.md`](../.claude/rules/demo-data.md)
- [`rust-test-conventions.md`](../.claude/rules/rust-test-conventions.md)
- [`ts-test-conventions.md`](../.claude/rules/ts-test-conventions.md)

## Sibling apps

Taskboard is the fourth app of the same owner and inherits their stack, process and look. When this repo has
no answer yet, look there (read-only):

| App | Path | Take from it |
|---|---|---|
| Daybook | `../daybook` | the newest and most mature: the look (`docs/ux.md`, `src/styles/`), Settings window, scrollbars, dev scripts, demo screenshots |
| TimeTracker | `../timetracker` | SQLite through sqlx (migrations, repositories, undo as ops), the demo seed and showcase, the CLI and MCP server, performance tests |
| DataBee | `../databee` | optional features as inline plugins, ports with fakes (`Database`, `SecretStore`), the read-only MCP server |
