# 0008. The CLI is a second process that asks the running app over the socket

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0008 (TimeTracker ADR 0017).

## Context

Scripts, other apps of the owner (Daybook drives TimeTracker this way) and coding agents need to read and
change the app's state. A CLI that opens the data itself would race the running app and bypass its rules,
undo and events.

## Decision

The app's binary is also its CLI. The first process binds `taskboard.sock` in the config dir and holds
`taskboard.lock`. A second `taskboard …` parses argv (clap), sends one JSON line over the socket
(`ipc.rs`; the server checks the peer's uid), prints the reply (`--json` for scripts) and exits. The app
answers through the same core functions as the UI. `show`, `focus` and a bare `taskboard` start the app when
it isn't running; everything else exits 2 with "Taskboard isn't running". `quit` is a socket command, so
scripts never need `pkill`.

## Consequences

- One code path per action, whether it comes from the UI, the CLI, a link or a script.
- The socket is also the dev hook: debug builds accept an `eval` message (`scripts/dev/js.sh`).
- A read-only MCP server, if added, is the only other path to the data.
