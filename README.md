# Taskboard

My task and project manager for macOS: a local-first Jira Cloud client. It mirrors my Jira issues into a
local SQLite cache and shows them in a fast UI with Spaces, filter lists and views, so nothing waits on the
network. Work in progress.

It is a local tool for one person on one Mac. Published as source, as-is: no releases, no binaries, no
support. To build it or port it, point a coding agent at [AGENTS.md](AGENTS.md).

## Install

```sh
bun install
scripts/create-signing-cert.sh   # once per Mac
bun run release
```

This builds `Taskboard.app`, links it into `~/Applications` and the `taskboard` command into `~/.local/bin`.
It runs on macOS on Apple Silicon only.

## Docs

[docs/README.md](docs/README.md) is the index of the technical documentation.

## License

MIT, see [LICENSE](LICENSE).
