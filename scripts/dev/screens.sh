#!/bin/zsh
# Captures the README screenshots from the debug app on a fresh demo seed: screens.sh [dir] (default docs/screens).
# Moves the debug data folder aside, starts the debug build at a fixed size, drives it with its own CLI and
# restores the folder afterwards. The window comes to the front: macOS doesn't draw a covered web view, so a
# capture of it is blank. Build first: bun run build:app.
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${1:-$ROOT/docs/screens}
HERE="$ROOT/scripts/dev"
APP="$ROOT/src-tauri/target/debug/bundle/macos/Taskboard (Debug).app"
BIN="$APP/Contents/MacOS/taskboard"
DATA="$HOME/Library/Application Support/com.philippstracker.taskboard.debug"
ASIDE="$DATA.screens-backup"
[[ -x "$BIN" ]] || { echo "Build first: bun run build:app"; exit 1; }
[[ ! -e "$ASIDE" ]] || { echo "$ASIDE exists from an earlier run; restore or remove it first"; exit 1; }
mkdir -p "$OUT"

"$ROOT/scripts/stop-app.sh" "$BIN"
[[ -e "$DATA" ]] && mv "$DATA" "$ASIDE"
restore() {
  "$ROOT/scripts/stop-app.sh" "$BIN"
  rm -rf "$DATA"
  [[ -e "$ASIDE" ]] && mv "$ASIDE" "$DATA"
}
trap restore EXIT

"$ROOT/scripts/sign-app.sh" "$APP" com.philippstracker.taskboard.debug
TASKBOARD_WINDOW_FRAME="80,60,1440,900" nohup "$BIN" >/dev/null 2>&1 &
disown

# The first sync fills the cache from the fake Jira.
for _ in {1..60}; do
  sleep 1
  "$BIN" status 2>/dev/null | grep -q "sync: idle" && "$BIN" status | grep -qE ": [1-9][0-9]* cached" && break
done
"$BIN" focus >/dev/null
sleep 2

shot() { sleep 1.5; "$HERE/screenshot.sh" "$OUT/$1.png"; echo "$OUT/$1.png"; }

# Changes on the demo site since the first sync fill Recently changed.
"$BIN" dev drift >/dev/null
"$BIN" sync >/dev/null
for _ in {1..20}; do sleep 1; "$BIN" status | grep -q "sync: idle" && break; done
"$BIN" go recent >/dev/null
shot recent
"$BIN" go NW "All issues" >/dev/null
shot list
"$BIN" go NW Board >/dev/null
shot board
"$BIN" open NW-1042 >/dev/null
shot issue
"$BIN" go NW "All issues" >/dev/null
"$HERE/js.sh" 'window.taskboard.getState().setPaletteOpen(true)'
sleep 0.5
"$HERE/js.sh" '(() => { const input = document.querySelector(".palette input"); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set; set.call(input, "cart"); input.dispatchEvent(new Event("input", { bubbles: true })); })()'
shot palette
"$HERE/js.sh" 'window.taskboard.getState().setPaletteOpen(false)'
"$BIN" go NW Table >/dev/null
shot table
# The Settings window, alone on screen so the capture picks it.
"$HERE/js.sh" 'window.__TAURI_INTERNALS__.invoke("run_command", { id: "app:settings:space:10001" })'
sleep 1
"$BIN" hide >/dev/null
shot space-settings
