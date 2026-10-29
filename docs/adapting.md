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
| Product name | `Taskboard` | `Taskboard (Debug)` | `tauri.conf.json` `productName` and window `title`; `tauri.debug.conf.json`; `lib.rs` `APP_NAME`, which is also the Keychain service (`secrets.rs`); `index.html` |
| Bundle | `Taskboard.app` | `Taskboard (Debug).app` | `scripts/install.sh`, `scripts/stop-app.sh`, `scripts/dev/launch.sh`; the window owner in `scripts/dev/screenshot.sh` |
| Bundle identifier | `com.philippstracker.taskboard` | `com.philippstracker.taskboard.debug` | both Tauri configs; `lib.rs` `IDENTIFIER` (tested against both); `scripts/install.sh`, `scripts/dev/launch.sh`; the socket path in `scripts/dev/js.sh`. It names the config dir and logs |
| Binary and CLI | `taskboard` | the same name inside the debug bundle, run by path | Cargo package `name` (also the self dev-dependency); `cli/`; the scripts |
| Installed links | `~/Applications/Taskboard.app`, `~/.local/bin/taskboard` | none | `scripts/install.sh` |
| URL scheme | `taskboard://` | `taskboard-debug://` | both Tauri configs; `lib.rs` `LINK_SCHEME` (tested) |
| Socket, lock | `taskboard.sock`, `taskboard.lock` in the config dir | the same in the debug config dir | `lib.rs`, `instance.rs`; `scripts/dev/js.sh` |
| Config dir | `~/Library/Application Support/com.philippstracker.taskboard/` | `….taskboard.debug/` | `lib.rs` `config_dir()` |
| Logs | `~/Library/Logs/com.philippstracker.taskboard/` | `….taskboard.debug/` | Tauri's `app_log_dir` |
| Signing certificate | `Taskboard Local Signing` or `TASKBOARD_SIGN_IDENTITY` | the same | `scripts/create-signing-cert.sh`, `scripts/sign-app.sh` |
| Env vars | `TASKBOARD_SIGN_IDENTITY` | also `TASKBOARD_WINDOW_FRAME`, `TASKBOARD_BACKGROUND`, `TASKBOARD_DEV_TERMINAL` | `scripts/sign-app.sh`; `dev/`; `scripts/dev/launch.sh` |
| Store on `window` | none | `window.taskboard` (`VITE_DEV_HOOKS=1`) | `src/main.tsx`, `src/vite-env.d.ts`, the dev scripts |
| Author, license | Philipp Stracker, MIT | | `Cargo.toml`, `package.json`, `LICENSE` |

## Network hosts

Only `jira/gateway/http.rs` talks to the network, and only to the hosts of the current Jira connection
([ADR 0020](adr/0020-jira-site-and-atlassian-gateway-are-the-hosts.md)) and, for profile pictures, Atlassian's
avatar host ([ADR 0032](adr/0032-avatars-come-from-atlassians-avatar-host-through-rust.md)). A new host needs an
ADR and a row here.

| Host | Why | Where |
|---|---|---|
| `<site>.atlassian.net` (the configured site, `https`) | Classic tokens, `_edge/tenant_info`, browse links | `jira/site.rs` `Site::hosts`, `normalize_site_url` |
| `api.atlassian.com` (`https`) | Scoped tokens: `/ex/jira/<cloudId>/rest/api/3/` | `net.rs` `GATEWAY_HOST` |
| `avatar-management--avatars.<region>.prod.public.atl-paas.net` (`https`, default port, region `[a-z0-9-]+`) | People's profile pictures, `GET` without credentials, PNG/JPEG/GIF/WebP up to 512 KB | `net.rs` `AvatarUrl`, `jira/gateway/avatar.rs`, `Http::get_image` |
| `127.0.0.1` (`http`, debug builds and tests only) | The fake Jira ([ADR 0022](adr/0022-the-debug-build-syncs-from-a-built-in-fake-jira.md)) | `net.rs` `is_fake_jira`, `jira/gateway/http.rs` `Http::allows`, `jira/site.rs` |

## Platform-specific code

macOS on Apple Silicon only: the config path, Unix sockets, window vibrancy and the overlay title bar, the
Dock app model (ADR 0016), codesigning, the Quick Look panel for attachments (`quicklook.rs`, `objc2-quick-look-ui`,
ADR 0035; the app cache folder `~/Library/Caches/<bundle identifier>/attachments/`), and the dev scripts (zsh,
Swift, `screencapture`).

## Conventions to keep

- Rust owns the logic; the page renders. IPC only in `api.ts` files; types generated with ts-rs.
- Small feature modules; split files over ~400 lines.
- No color literal outside the palettes; the native look of `ux.md`.
- Tests through public interfaces only; test code written and run by the agents.
- Debug is a separate app on demo data; nothing in the repo carries real data.
- Docs by file and symbol, never by line number; an ADR for every decision, never edited once accepted.
