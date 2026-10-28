# 0015. Builds are signed with a local certificate

## Status

Accepted (2026-10-28). Inherited from the signing steps of DataBee, TimeTracker and Daybook.

## Context

macOS keys privacy grants (Accessibility, Automation, Calendars, Reminders) and Keychain trust to the app's
designated requirement. Signed ad hoc, that is the build's hash, so every rebuild asks again.

## Decision

`scripts/create-signing-cert.sh` creates the self-signed "Taskboard Local Signing" code-signing certificate
in the login keychain, once per Mac. `scripts/sign-app.sh` signs with `$TASKBOARD_SIGN_IDENTITY`, else that
certificate, else ad hoc (with a warning). `scripts/dev/launch.sh` signs the debug bundle and
`scripts/install.sh` the release bundle, each with its own identifier. Tauri builds with
`signingIdentity: "-"` and the scripts re-sign.

## Consequences

- A grant survives rebuilds; the debug and release apps are granted once each.
- A running app is quit before its files are re-signed (re-signing can get it killed), and the reopen is
  retried (Launch Services may answer -600 right after a quit).
