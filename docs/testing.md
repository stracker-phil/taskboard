# Testing and dev tooling

## Commands

Coding agents don't run these themselves. They hand test writing to the `write-ts-unit-test` and
`write-rust-unit-test` agents and runs to the `ci` agent (`.claude/agents/`), so test code and test output
stay out of the main context ([`.claude/rules/testing.md`](../.claude/rules/testing.md)).

| Command | Runs |
|---|---|
| `bun run test` | Vitest UI tests (`test:ui`), then the Rust tests (`test:rust`) |
| `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored` | the slow `#[ignore]` tests on temp folders or a temp database; the `ci` agent's integration suite |
| `bun run lint` | regenerates the IPC types and fails when `src/api/types.gen.ts` changed, then `tsc --noEmit`, `eslint .` (type-checked `typescript-eslint`, `react-hooks`) and `cargo clippy --lib --bins -- -D warnings` (production code only) |
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | Rust formatting (run before lint) |

Unit tests and lint never run at the same time: both regenerate `src/api/types.gen.ts`.

## Agents

| Agent | Does |
|---|---|
| `write-rust-unit-test` | writes Rust unit tests (`mod tests`) and integration tests; never runs them |
| `write-ts-unit-test` | writes Vitest tests next to components, mocking only `src/api`; never runs them |
| `ci` | runs one suite and replies with a pass/fail summary |
| `review-code-cleanliness` | reviews a changed file for structure, comment noise, boundaries and over-engineering |

The `ship-step` skill chains cleanup, review, `cargo fmt`, `ci`, build, launch and commit at the end of every
step.

## Rust

- **Unit tests** sit next to the code in `#[cfg(test)] mod tests`, named as behavior sentences, with exact
  and table-driven assertions.
- **Helpers** live in `src/test_support/`: `temp_config_dir`, builders, fakes and the demo seed. They compile
  under `cfg(any(test, feature = "test-util", debug_assertions))`
  ([ADR 0006](adr/0006-test-doubles-behind-test-util.md)).
- **Contracts:** `lib.rs` tests compare `IDENTIFIER` and `LINK_SCHEME` with both Tauri configs; each
  `#[ts(export)]` type has an `export_bindings_*` test (`bun run types`).
- **Performance**, once there is a database: a seeded integration test with median limits per view, as
  TimeTracker's `tests/performance.rs`.

## Frontend

- `*.test.ts(x)` next to their code; `src/test-setup.ts` stubs `ResizeObserver`, `matchMedia` and the
  clipboard, and pretends to be a Mac (`Mod` is ⌘).
- Only `src/api` is mocked, with `createApiMock()` (`src/test/apiMock.ts`); fixtures in `src/test/`.
- Testing Library queries by role or text, `userEvent` for interactions.
- Guards: `src/test/boundary.test.ts` (Tauri imports only in `api.ts` files), `src/styles/colors.test.ts`
  (no color literal outside the palettes, matching palettes), `src/styles/contrast.test.ts`.

## Demo data

The debug app, the tests and the README screenshots share one made-up data set in `src-tauri/src/test_support/`
([ADR 0011](adr/0011-debug-build-is-a-separate-app-with-demo-data.md),
[`demo-data.md`](../.claude/rules/demo-data.md)). The debug app loads it on its first launch, into its own
data folder. To start over, quit the debug app and delete
`~/Library/Application Support/com.philippstracker.taskboard.debug/`.

## Driving the app

The dev scripts work without stealing focus, so the app can run on a second screen while you or an agent
keep typing in the terminal. They are macOS-only (zsh, Swift, `screencapture`, `python3`).

| Script | Does |
|---|---|
| `bun run build:app` | builds the debug bundle `src-tauri/target/debug/bundle/macos/Taskboard (Debug).app` (`tauri.debug.conf.json`, `VITE_DEV_HOOKS=1`) |
| `scripts/dev/launch.sh [cli args]` | refuses a bundle older than the sources, signs it, restarts only the debug app next to the terminal window, without focus. Log: `$TMPDIR/taskboard-dev.log`; terminal app: `$TASKBOARD_DEV_TERMINAL` (default `Terminal`) |
| `scripts/dev/js.sh '<js>'` | runs JavaScript in the running debug app over its socket. Fire-and-forget; wrap code in an IIFE; the store is `window.taskboard`; a first line `@<label>` targets another window |
| `scripts/dev/screenshot.sh out.png [--overlay]` | captures the debug app's largest window, also in the background |
| `scripts/dev/screenshot-looks.sh <dir>` | captures the window in each scheme of the current macOS mode ([ux.md](ux.md#scheme-audit)) |
| `bun run release` | builds the release `.app` and `.dmg`, then `scripts/install.sh`: signs it, links `~/Applications/Taskboard.app` and `~/.local/bin/taskboard`, restarts a running Taskboard on it |

Never use `bun run app`, `open` or AppleScript clicks and keystrokes to drive the app: they take focus.

### Signing

macOS keys privacy grants and Keychain trust to identifier + certificate
([ADR 0015](adr/0015-signed-with-a-local-certificate.md)). Run `scripts/create-signing-cert.sh` once per Mac;
`scripts/sign-app.sh` signs with `TASKBOARD_SIGN_IDENTITY`, else that certificate, else ad hoc.
