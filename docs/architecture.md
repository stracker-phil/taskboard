# Architecture

Taskboard is the owner's task and project manager for macOS: a local-first Jira Cloud client that mirrors
issues into a local SQLite cache and reads only from it. It is a Tauri 2 app: a Rust process owns every piece
of logic (Jira traffic, sync, storage, filtering, grouping), a React 19 webview renders state and forwards
intent. This page holds the rules every feature follows and grows with the code.

## Design rules

These rules hold everywhere in the code. Keep them when you change things.

1. **Rust owns the logic; React only renders** ([ADR 0001](adr/0001-rust-owns-the-logic.md)). Validation,
   ordering, filtering, grouping, dates and the CLI are Rust. The page asks, renders and forwards intent.
2. **IPC only in `api.ts` files** ([ADR 0002](adr/0002-ipc-in-api-files.md)). Only `src/api/index.ts` (and
   `src/<module>/api.ts`) imports `@tauri-apps/*`. Everything else imports the `api` object, which is also
   what UI tests mock. `src/test/boundary.test.ts` enforces it.
3. **Errors are data** ([ADR 0003](adr/0003-errors-as-kinds.md)). Every command returns
   `Result<T, AppError>`, serialized as `{ kind, message, field? }`. The UI branches on `kind` only.
4. **Types are generated** ([ADR 0004](adr/0004-generated-ipc-types.md)). Every type crossing IPC derives
   `ts_rs::TS` and exports to `src/api/types.gen.ts` (camelCase). Never edit the generated file.
5. **Panics unwind** ([ADR 0005](adr/0005-panic-unwind.md)). A bug fails one command, not the app. Locks
   are taken with the poison-tolerant helpers in `lock.rs`.
6. **No color literal outside `src/styles/palettes/`** ([ADR 0007](adr/0007-color-palettes.md),
   [ADR 0009](adr/0009-a-scheme-per-system-mode.md)). A Vitest scan enforces it.
7. **The look is native and light** ([ADR 0010](adr/0010-native-macos-look.md), [ux.md](ux.md)).
8. **One write path into the data.** Every change goes through the data layer's one write function, which
   validates, saves and announces it as an event (and, with a database, records its inverse for undo, as
   TimeTracker's ops). No other code writes the data.
9. **Commands live in Rust** ([ADR 0012](adr/0012-commands-live-in-rust.md)). Menus, shortcuts, the palette,
   the CLI and links run commands of one bus by id.
10. **Small feature modules** ([ADR 0014](adr/0014-small-feature-modules.md)): by feature, pure cores, ports
    only where a swap is real, files under ~400 lines.
11. **Debug is a separate app on demo data** ([ADR 0011](adr/0011-debug-build-is-a-separate-app-with-demo-data.md));
    nothing in the repo carries real data ([`demo-data.md`](../.claude/rules/demo-data.md)).
12. **Local first** ([ADR 0013](adr/0013-network-only-to-listed-hosts.md)): offline by default, network only
    to listed hosts, tokens in the Keychain.

## Process model

```
┌──────────── Taskboard.app ────────────────────────────────────────────────┐
│  Webview (React)                          Rust core (tokio runtime)       │
│  ┌──────────────────────┐   invoke   ┌─────────────────────────────────┐  │
│  │ components/          │ ─────────▶ │ #[tauri::command]s per feature  │  │
│  │ state/store.ts       │            │ bus/ (commands by id)           │  │
│  │ commands/ (keys)     │ ◀───────── │ AppState, store/, data layer    │  │
│  │ api/index.ts         │   events   └─────────────────────────────────┘  │
│  └──────────────────────┘                                                 │
│  Native menu ◀────────────────────────────────▶ Rust                      │
└───────────────────────────────────────────────┬───────────────────────────┘
                     Unix socket (ipc.rs)       │
  `taskboard …` (2nd process, CLI) ─────────────┘
  taskboard://… (deep link) → tauri-plugin-deep-link → launch queue
```

- **One instance.** The first process binds `taskboard.sock` in the config dir and holds `taskboard.lock`; a
  second `taskboard …` sends its request over the socket and exits ([ADR 0008](adr/0008-cli-asks-the-running-app.md)).
- **Launch requests are pulled, not pushed.** Rust queues launch events and emits a bare `launch` event; the
  page calls `take_launch_events`, so it can't miss the request that started the app.
- **The window** is declared `visible: false` and shown by Rust, so `--hidden` never flashes it. Closing it
  hides it; the app keeps running (and syncing) until Quit, and the Dock icon brings it back
  ([ADR 0016](adr/0016-closing-the-window-keeps-the-app-running.md)). There is no tray, LaunchAgent or global
  shortcut.
- **The CLI**: `taskboard` starts or activates the app; `status [--json]`, `show`, `focus`, `hide`,
  `open <KEY>` and `quit` ask the running app (`cli/`, `remote.rs`). `show` and `focus` start it when it
  isn't running; the others exit 2 with "Taskboard isn't running".
- **Settings** is a window of its own ([ux.md](ux.md#settings)).

## Data

The config dir is `~/Library/Application Support/com.philippstracker.taskboard/` (debug:
`….taskboard.debug/`). Settings, key bindings and window layout are small JSON files there, each a
`JsonStore` over a `JsonFile`, as in Daybook: files 0600 in a 0700 folder, written atomically; a missing file
loads as defaults, unknown keys are ignored and a file that cannot be parsed is moved aside as
`<name>.<timestamp>.corrupt`. A store saves a changed copy before it swaps it in.

Taskboard's data is two SQLite files ([data.md](data.md), [ADR 0019](adr/0019-two-sqlite-files-through-rusqlite.md)):
`local.sqlite` holds the owner's setup (Spaces, filter lists, views, pins), `cache.sqlite` the Jira mirror,
attached as `cache`. Every change goes through `Db::write`, which emits `issues_changed` or `local_changed`
after the commit. Reset cache deletes only the cache file.

Jira is reached only through `jira/` ([jira.md](jira.md)): one `JiraClient` per connection over `net::Http`,
which allows the site and `api.atlassian.com` only. The API token stays in the Keychain (`secrets.rs`);
`connection::JiraAccess` builds clients from the saved connection. The debug build serves the demo site from
the fake Jira on loopback.

## Repository map

Target layout; folders appear with the code that needs them.

```
src-tauri/
  src/main.rs lib.rs         entry; identity (IDENTIFIER, APP_NAME, LINK_SCHEME), argv, then app::start
  src/app.rs handlers.rs     Tauri builder; every registered command (the install list)
  src/commands.rs launch.rs  settings, key bindings, UI state, links, logs; launch events pulled by the page
  src/instance.rs ipc.rs     socket handoff to the running app, instance lock; the socket protocol
  src/cli/ remote.rs         the clap command line and its replies; the app's side of each request
  src/events.rs state.rs     every event name; AppState
  src/error.rs lock.rs       AppError (+ ErrorKind); poison-tolerant lock helpers
  src/store/                 JsonFile, JsonStore, settings/, keybindings, ui_state
  src/bus/                   the command bus
  src/db/                    the two SQLite files: open, migrate, read, the one write path
  migrations/local/ cache/   numbered SQL migrations per file
  src/net.rs secrets.rs      the only HTTP client and its host check, log redaction; the Keychain
  src/jira/                  Jira Cloud: site, client, limiter, retries, paging, endpoints, DTOs, JQL
  src/connection/            the Jira connection: Keychain access, Test Connection, tracked projects, scope
  src/<feature>/             one folder per feature: types, pure logic, commands.rs, tests
  src/window/ menu/          the main and Settings windows, the native app menu
  src/dev.rs                 debug builds only: window frame, background launch, eval
  src/test_support/          fixtures, fakes (cfg test, test-util); the demo Jira site and the fake Jira (also debug)
  tauri.conf.json tauri.debug.conf.json
src/
  main.tsx modules.ts App.tsx
  api/index.ts types.gen.ts types.ts   the only IPC; generated and hand-written types
  queries/                   TanStack Query hooks and the event → query-key map (ADR 0018)
  state/store.ts slices/     UI-only state (zustand)
  shell/                     the main window frame: sidebar and content panel
  commands/ registry/        in-window keys; Settings page registry
  components/ windows/<name>/ lib/ styles/ test/
docs/                        this folder; adr/ holds the decision records; plan/ (gitignored) the steps
scripts/*.sh scripts/dev/    install, stop, signing; the dev loop
.claude/                     rules, agents, skills, settings
```

## Platform

The code base targets **macOS on Apple Silicon** only. The platform-specific parts are listed in
[adapting.md](adapting.md).
