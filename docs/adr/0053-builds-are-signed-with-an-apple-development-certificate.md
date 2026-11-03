# 0053. Builds are signed with an Apple Development certificate

## Status

Accepted (2026-11-03). Owner's report. Supersedes [ADR 0015](0015-signed-with-a-local-certificate.md).

## Context

With the self-signed "Taskboard Local Signing" certificate (ADR 0015), every new release build asked for the
login password before it read its token from the Keychain (ADR 0021). The Keychain item's partition list
admits an app without a Team ID only by its code hash, which changes with every build, so "Always Allow"
covered one build. The debug build never asked because it keeps its demo token in memory (ADR 0026).

## Decision

`scripts/sign-app.sh` signs with `$TASKBOARD_SIGN_IDENTITY`, else the first valid "Apple Development"
certificate (made in Xcode → Settings → Accounts → Manage Certificates; a free Apple ID is enough), picked by
its hash because an expired certificate may share its name, else the local certificate of
`scripts/create-signing-cert.sh`, else ad hoc. The certificate carries a Team ID, so the Keychain admits
every build of that team after one "Always Allow".

Rejected: keeping the local certificate and re-adding each build's hash to the item (needs the password every
time); the data-protection Keychain (needs a keychain-access-groups entitlement and a provisioning profile);
storing the token outside the Keychain (ADR 0021).

## Consequences

- New builds read the token without a prompt; switching certificates asks once more, and privacy grants
  (Accessibility) are given once more.
- The certificate expires after a year; Xcode makes a new one, then the Keychain asks once.
- No notarization: a Development certificate is for this Mac, not for distribution.
- The local certificate stays as the fallback for a Mac without an Apple ID in Xcode.
