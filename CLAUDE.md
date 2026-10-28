# Taskboard

The owner's task and project manager for macOS: a local-first, read-only (for now) Jira Cloud client that
mirrors the owner's issues into SQLite and shows them in a fast ClickUp-style UI. The spec, the mockup and the
step plan live in `docs/plan/` (gitignored); build only what the current step asks for.

Taskboard is the fourth app of the same owner and inherits their architecture, process and look. **Daybook**
(`../daybook`) is the template: the newest, with the most mature design (on-demand scrollbars, the Settings
window, a light native UI) and dev flow. **TimeTracker** (`../timetracker`) is the reference for SQLite,
undo, the demo seed, the CLI and MCP; **DataBee** (`../databee`) for plugins and ports. Read them; never
write to them. Taskboard differs only where its job differs; each deviation has an ADR.

## Rules

- Every session or step ends in a commit, so it can be reverted. Commit each finished part of a step as soon as
  it is green, not only at the end.
- Never push. Only commit locally.
- Write unit tests for every change and run them (through the `ci` agent) before every commit; tests and
  lint must be green.
- Tests pin behavior, not internals: public interfaces only, no log assertions, no call counts.
- Refactor continuously: when a file grows large (over ~400 lines of production code) or complex, simplify it
  and split it into focused modules in the same step, without asking.
- Keep modules small, isolated and replaceable (ADR 0014): by feature, pure cores, a trait only where a second
  implementation exists today. No speculative abstractions.
- Demo data only: code, docs, tests, commits and screenshots never contain the owner's real data
  (`.claude/rules/demo-data.md`). The debug build runs on its own data folder with the demo seed.
- The implementation plan lives in `docs/plan/` (gitignored). Work one step per session; set its status from
  PENDING to DONE when it is committed.

## Docs
Technical docs live in `docs/` (index: `docs/README.md`). Read `docs/architecture.md` and the page of the
layer you change before changing it; `docs/ux.md` is the design language; `docs/adapting.md` lists everything
macOS- or name-specific. Decisions are in `docs/adr/` (never edited once accepted; a new record supersedes).
Update the affected docs page in the same commit when behavior, commands, files or extension points change.
Document by file and symbol, never by line number.

## Stack
- Rust backend in `src-tauri/` (rustup stable, edition 2024; Homebrew rustup is keg-only:
  `/opt/homebrew/opt/rustup/bin`)
- Tauri 2 + React 19 + TypeScript in `src/`, bun as package manager, zustand, lucide-react, Vitest
- macOS, Apple Silicon only

## Architecture
- Rust owns all logic; React only renders (ADR 0001). Only `api.ts` files call `invoke`/`listen` (ADR 0002).
- Errors cross the boundary as `{ kind, message }` (ADR 0003). IPC types are generated with ts-rs (ADR 0004).
- Commands live in a Rust bus; menus, shortcuts, palette, CLI and links run them by id (ADR 0012).
- The CLI is the same binary and asks the running app over a Unix socket (ADR 0008).
- No color literal outside `src/styles/palettes/` (ADR 0007); the look is native (ADR 0010, `docs/ux.md`).
- Debug builds: `Taskboard (Debug)`, `com.philippstracker.taskboard.debug`, own data and socket, demo seed,
  dev hooks; never installed (ADR 0011).
- Repository map: `docs/architecture.md`.

## Commands
Test, lint and format-check commands run through the `ci` agent, never in the main context.
- `bun run test`: Vitest and Rust unit tests
- `bun run lint`: IPC types drift check, tsc, ESLint and clippy on production code (warnings are errors)
- `bun run release`: release `.app` and `.dmg`, signed, then installed (`scripts/install.sh`)
- `bun run build:app` + `scripts/dev/launch.sh`: debug build, restarted next to the terminal without stealing focus
- `scripts/dev/js.sh` / `scripts/dev/screenshot.sh`: drive (store: `window.taskboard`) and capture the running
  debug window in the background
- `scripts/create-signing-cert.sh` (once per Mac) makes the certificate that keeps privacy grants

## Workflow
Finish every step with the `ship-step` skill. Rules in `.claude/rules/`; `testing.md` says which agents in
`.claude/agents/` write and run tests.
