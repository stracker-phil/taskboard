#!/bin/zsh
# Installs the release build as the one Taskboard of this machine: ~/Applications/Taskboard.app links to
# it and the `taskboard` command links through that app. A running Taskboard (any build) restarts on it,
# its window shown or hidden as before.
# `bun run release` ends here; debug builds are never installed (scripts/dev/launch.sh runs them by path).
# A ~/.local/bin/taskboard that isn't this script's link stays unless
# `scripts/install.sh --replace-cli`.
set -e
REPLACE_CLI=
[[ "$1" == --replace-cli ]] && REPLACE_CLI=1
ROOT=$(cd "$(dirname "$0")/.." && pwd)
APP="$ROOT/src-tauri/target/release/bundle/macos/Taskboard.app"
[[ -d "$APP" ]] || { echo "Not found: $APP (run 'bun run release')"; exit 1; }
LINK="$HOME/Applications/Taskboard.app"
BIN="$LINK/Contents/MacOS/taskboard"
CLI="$HOME/.local/bin/taskboard"

running() { pgrep -f 'Taskboard( \(Debug\))?\.app/Contents/MacOS/taskboard( --hidden)?$' >/dev/null; }
# Re-signing a running app's files can get it killed, so it quits first; a hidden window stays hidden.
if running; then
  WAS_RUNNING=1
  OPEN_ARGS=()
  "$APP/Contents/MacOS/taskboard" status --json 2>/dev/null | grep -q '"windowVisible": *false' \
    && OPEN_ARGS=(--args --hidden)
  "$ROOT/scripts/stop-app.sh" "$APP/Contents/MacOS/taskboard"
fi
"$ROOT/scripts/sign-app.sh" "$APP" com.philippstracker.taskboard

mkdir -p "$HOME/Applications" "$HOME/.local/bin"
ln -sfn "$APP" "$LINK"
echo "Installed the release build as ~/Applications/Taskboard.app"
if [[ ! -e "$CLI" && ! -L "$CLI" ]] || [[ "$(readlink "$CLI")" == "$BIN" ]] || [[ -n "$REPLACE_CLI" ]]; then
  ln -sfn "$BIN" "$CLI"
  echo "Linked ~/.local/bin/taskboard"
else
  echo "Left ~/.local/bin/taskboard alone (not this script's link); replace it with: scripts/install.sh --replace-cli"
fi

[[ -n "$WAS_RUNNING" ]] || exit 0
# Launch Services can refuse right after the old app quit (error -600), so try a few times.
for _ in {1..10}; do
  open -g "$LINK" "${OPEN_ARGS[@]}" 2>/dev/null || true
  sleep 1
  running && { echo "Restarted Taskboard on the new release build"; exit 0; }
done
echo "Taskboard did not start again; open it from ~/Applications" >&2
exit 1
