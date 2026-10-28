# 0026. The debug build keeps its demo token in memory

## Status

Accepted (2026-10-28). Amends [0021](0021-the-token-lives-in-the-keychain-through-secrets-rs.md) for debug
builds; the release build keeps the token in the Keychain.

## Context

ADR 0021 expected the local signing certificate to keep Keychain access stable across rebuilds. It keeps the
item's access list stable, but macOS also checks the item's partition list, which for an app without an
Apple-issued team id names the exact build (`cdhash:`). Every rebuild therefore asks for the login password
when the app first reads its token. The debug build is rebuilt many times a day and only talks to the fake
Jira, which accepts any token.

## Decision

- Debug builds use an in-memory `SecretStore` holding the demo account's demo token (`state.rs`
  `secret_store`). They never touch the Keychain, so no prompt and no owner token can end up in them
  ([ADR 0011](0011-debug-build-is-a-separate-app-with-demo-data.md)).
- Release builds keep `Keychain` (service `Taskboard`, account = the email), as the spec requires.
- Rejected: storing the debug token in a file (a real token typed into the debug app would land on disk).

## Consequences

- The release build still asks once after each new release build. Only a signing identity with an Apple team
  id (a valid Apple Development certificate via `TASKBOARD_SIGN_IDENTITY`) would remove that prompt.
- A token entered in the debug app's Settings lasts until the debug app quits.
