# Adapting Taskboard

Taskboard is written so that a coding agent can take it over. `CLAUDE.md` is loaded automatically; it points
the agent to the rules in `.claude/rules/` and to these docs. The `ship-step` skill runs after every step, and
subagents in `.claude/agents/` write the tests, run the suites and review changed files. This page lists what
such changes touch. Keep it current: every new name-, path- or platform-specific place gets a row here.

## Identity

The app's name and ids live in a few places. Change them together. Debug builds are a second app with their
own name, identifier, data, socket and URL scheme
([ADR 0011](adr/0011-debug-build-is-a-separate-app-with-demo-data.md)).

| What | Release | Debug | Where |
|---|---|---|---|
| Product name | `Taskboard` | `Taskboard (Debug)` | `tauri.conf.json` `productName` and window `title`; `tauri.debug.conf.json`; `lib.rs` `APP_NAME`; `index.html` |
| Bundle | `Taskboard.app` | `Taskboard (Debug).app` | `scripts/install.sh`, `scripts/stop-app.sh`, `scripts/dev/launch.sh`; the window owner in `scripts/dev/screenshot.sh` |
| Bundle identifier | `com.philippstracker.taskboard` | `com.philippstracker.taskboard.debug` | both Tauri configs; `lib.rs` `IDENTIFIER` (tested against both); `scripts/install.sh`, `scripts/dev/launch.sh`; the socket path in `scripts/dev/js.sh`. It names the config dir, logs and Keychain service |
| Binary and CLI | `taskboard` | the same name inside the debug bundle, run by path | Cargo package `name` (also the self dev-dependency); `cli/`; the scripts |
| Installed links | `~/Applications/Taskboard.app`, `~/.local/bin/taskboard` | none | `scripts/install.sh` |
| URL scheme | `taskboard://` | `taskboard-debug://` | both Tauri configs; `lib.rs` `LINK_SCHEME` (tested) |
| Socket, lock | `taskboard.sock`, `taskboard.lock` in the config dir | the same in the debug config dir | `lib.rs`, `instance.rs`; `scripts/dev/js.sh` |
| Config dir | `~/Library/Application Support/com.philippstracker.taskboard/` | `….taskboard.debug/` | `lib.rs` `config_dir()` |
| Logs | `~/Library/Logs/com.philippstracker.taskboard/` | `….taskboard.debug/` | Tauri's `app_log_dir` |
| Signing certificate | `Taskboard Local Signing` or `TASKBOARD_SIGN_IDENTITY` | the same | `scripts/create-signing-cert.sh`, `scripts/sign-app.sh` |
| Env vars | `TASKBOARD_SIGN_IDENTITY` | also `TASKBOARD_WINDOW_FRAME`, `TASKBOARD_BACKGROUND`, `TASKBOARD_DEV_TERMINAL` | `scripts/sign-app.sh`; `dev.rs`; `scripts/dev/launch.sh` |
| Store on `window` | none | `window.taskboard` (`VITE_DEV_HOOKS=1`) | `src/main.tsx`, `src/vite-env.d.ts`, the dev scripts |
| Author, license | Philipp Stracker, MIT | | `Cargo.toml`, `package.json`, `LICENSE` |

## Network hosts

None. A host is added with an ADR ([0013](adr/0013-network-only-to-listed-hosts.md)) and a row here.

## Platform-specific code

macOS on Apple Silicon only: the config path, Unix sockets, window vibrancy and the overlay title bar, the
tray, the LaunchAgent, codesigning, and the dev scripts (zsh, Swift, `screencapture`).

## Conventions to keep

- Rust owns the logic; the page renders. IPC only in `api.ts` files; types generated with ts-rs.
- Small feature modules; split files over ~400 lines.
- No color literal outside the palettes; the native look of `ux.md`.
- Tests through public interfaces only; test code written and run by the agents.
- Debug is a separate app on demo data; nothing in the repo carries real data.
- Docs by file and symbol, never by line number; an ADR for every decision, never edited once accepted.
