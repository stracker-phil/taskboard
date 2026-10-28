# 0013. The app talks only to listed hosts

## Status

Accepted (2026-10-28). Inherited from Daybook ADR 0020.

## Context

A task manager holds private plans. Each integration (an issue tracker, a calendar, a chat) is a reason to
send data out, and a dependency that pulls in a client of its own can talk to anything.

## Decision

The app is local-first and works offline. `net.rs` is the only HTTP client and talks only to the hosts in
its `HOSTS` list; any other host is refused. Adding a host needs an ADR and a row in `docs/adapting.md`.
Tokens live in the Keychain (`secrets.rs`), never in a JSON file or a log.

## Consequences

- What leaves the Mac is reviewable in one list.
- Opening a link in the browser is not network access by the app; it goes through `tauri-plugin-opener`.
