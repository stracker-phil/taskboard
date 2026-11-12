# Taskboard technical documentation

These docs are for developers and coding agents working on the code. For what the app does and how to
use it, see the [project README](../README.md).

| Page | Covers |
|---|---|
| [architecture.md](architecture.md) | Design rules, process model, data, repository map, webview hardening |
| [ux.md](ux.md) | Design language: the mockup's window, tokens, type, metrics, Settings, keys |
| [data.md](data.md) | The two SQLite files, their tables, the write path, migration rules |
| [sync.md](sync.md) | The sync engine: jobs, states, storing pages, change records, events |
| [notifications.md](notifications.md) | macOS notifications after a change sync: when, what, the buttons |
| [scopes.md](scopes.md) | The API token scopes the app needs and what each one is for |
| [links.md](links.md) | `taskboard://` links: what each opens, Copy Taskboard Link, the command allowlist |
| [writes.md](writes.md) | Changes to Jira: the two locks, pending writes, the flow, "Change back to …" |
| [jira.md](jira.md) | The Jira client: site and tokens, endpoints, limits, retries, errors, JQL, the fake Jira |
| [testing.md](testing.md) | Unit and integration tests, the test agents, the demo seed, scripts that drive the app, signing |
| [adapting.md](adapting.md) | Identity (names, ids, paths), network hosts, platform-specific code, common changes, conventions to keep |
| [adr/](adr/README.md) | Architecture decision records: why the design is the way it is |

Pages for the other layers (`backend.md`, `frontend.md`) and for each feature are added with the code
they describe, in the same commit.

Suggested reading order: architecture, then the layer you are changing, then adapting.

The rules in [`.claude/rules/`](../.claude/rules/) are binding for changes:
- [`testing.md`](../.claude/rules/testing.md)
- [`commits.md`](../.claude/rules/commits.md)
- [`demo-data.md`](../.claude/rules/demo-data.md)
- [`stable-controls.md`](../.claude/rules/stable-controls.md)
- [`rust-test-conventions.md`](../.claude/rules/rust-test-conventions.md)
- [`ts-test-conventions.md`](../.claude/rules/ts-test-conventions.md)

## Sibling apps

Taskboard is one app of the owner's family and shares their stack, process and look (family catalog: the
user-level `app-blueprint` skill). When this repo has no answer yet, look there (read-only):

| App | Path | Take from it |
|---|---|---|
| Daybook | `../daybook` | the Rust command bus (`docs/commands.md`), extra windows on a native layer, menu-bar app, global shortcuts, local HTTP API, JSON stores with repair |
| TimeTracker | `../timetracker` | SQLite through sqlx, undo as ops, the seeds and showcase, performance tests, socket `watch`, the MCP server, `PopUpButton` |
| DataBee | `../databee` | optional features as inline plugins, ports with fakes (`Database`, `SecretStore`), the `Secret` type |
| Launcher | `../project-launcher` | the Settings search over every row, Settings → Shortcuts (Reassign, Reset), the accent choice |
