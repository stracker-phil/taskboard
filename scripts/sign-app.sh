#!/bin/zsh
# Signs a built Taskboard bundle so macOS keeps its grants across rebuilds (ADR 0053). Uses
# $TASKBOARD_SIGN_IDENTITY, else a valid "Apple Development" certificate (Xcode → Settings → Accounts →
# Manage Certificates; a free Apple ID is enough), else the "Taskboard Local Signing" certificate of
# scripts/create-signing-cert.sh, else signs ad hoc (the grant then lasts one build).
# Only the Apple certificate carries a Team ID, so the Keychain trusts every build of the team; with the local
# certificate, each new build asks once for the login password before it reads its token.
# Signs with the hardened runtime: no injected libraries or debugger attach; the app needs no entitlements
# (not sandboxed; the Keychain and notifications need none).
# Usage: scripts/sign-app.sh path/to/App.app bundle.identifier
set -e
APP=$1 IDENTIFIER=$2
IDENTITY=${TASKBOARD_SIGN_IDENTITY:-}
if [[ -z "$IDENTITY" ]]; then
  # By hash: an expired certificate of the same name would make the name ambiguous.
  IDENTITY=$(security find-identity -v -p codesigning | awk '/"Apple Development: / { print $2; exit }')
fi
if [[ -z "$IDENTITY" ]] && security find-identity -p codesigning | grep -q '"Taskboard Local Signing"'; then
  IDENTITY="Taskboard Local Signing"
fi
codesign --force --options runtime --sign "${IDENTITY:--}" --identifier "$IDENTIFIER" "$APP" 2>/dev/null
[[ -n "$IDENTITY" ]] || echo "Signed ad hoc: grant Accessibility again after each rebuild (see scripts/sign-app.sh)"
