# AGENTS.md

Instructions for a coding agent that clones, builds or adapts Taskboard. Humans: see [README.md](README.md).

Taskboard is one developer's personal tool, published as source. There are no releases, binaries or support.
As shipped it builds and runs on **macOS on Apple Silicon** only; everything else is a port.

## Repository facts

- Tauri 2 app: Rust backend in `src-tauri/`, React 19 + TypeScript frontend in `src/`, bun as package manager.
- Rust owns all logic; React only renders. Read [docs/architecture.md](docs/architecture.md) before changing
  code, and the page of the layer you touch ([docs/README.md](docs/README.md) is the index).
- [CLAUDE.md](CLAUDE.md) and [.claude/rules/](.claude/rules/) are the maintainer's conventions (tests,
  commits, demo data). Follow them in this repo; in a fork, keep what serves you.
- IPC types in `src/api/types.gen.ts` are generated from Rust with ts-rs (`bun run types`); never edit them.

## Prerequisites (macOS)

| Tool | Check | Notes |
|---|---|---|
| Xcode Command Line Tools | `xcode-select -p` | `xcode-select --install` |
| rustup, stable toolchain | `rustup show` | `rust-toolchain.toml` pins `stable` with rustfmt and clippy. A Homebrew rustup is keg-only: put `/opt/homebrew/opt/rustup/bin` on `PATH` |
| bun | `bun --version` | |

## Clone and build

```sh
git clone <this repository>
cd taskboard
bun install
scripts/create-signing-cert.sh   # once per Mac; keeps privacy grants across rebuilds
bun run build:app                # debug bundle: src-tauri/target/debug/bundle/macos/Taskboard (Debug).app
bun run release                  # release bundle and .dmg, installed to ~/Applications and ~/.local/bin
```

Debug builds are never installed; call them by path or start them with `scripts/dev/launch.sh`. They keep
their data in `~/Library/Application Support/com.philippstracker.taskboard.debug/`, apart from the release
build's, and start with made-up demo data.

## Verify

| Command | Expect |
|---|---|
| `bun run test` | Vitest and Rust unit tests pass |
| `bun run lint` | exits 0: types drift check, `tsc`, ESLint, `clippy -D warnings` |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | no differences |
| `bun run release` | `Taskboard.app` and a `.dmg` in `src-tauri/target/release/bundle/` |

## Adapt

[docs/adapting.md](docs/adapting.md) lists every place that is specific to macOS or to the name, and the
Identity table for renaming the app. When done, run the Verify table on the target platform and update the
docs pages your change affects.
