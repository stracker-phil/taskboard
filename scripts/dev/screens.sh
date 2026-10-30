#!/bin/zsh
# Captures the README screenshots, light and dark, from the debug app on a fresh demo seed: screens.sh [dir] (default docs/screens).
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
# The app's own Appearance setting, so macOS keeps its look.
appearance() {
  "$HERE/js.sh" "(async () => { const i = window.__TAURI_INTERNALS__.invoke; const s = await i('get_settings'); await i('update_settings', { settings: { ...s, appearance: '$1' } }); })()"
  sleep 1
}
# A view's id by its name, from the CLI's JSON.
view_id() { "$BIN" views "$1" --json | python3 -I -c 'import json, sys; print(next(v["id"] for v in json.load(sys.stdin)["views"] if v["name"] == sys.argv[1]))' "$2"; }
# The open level and view as the store's narrowing key, "<listId>:<viewId>".
narrowing_key() { "$BIN" status --json | python3 -I -c 'import json, sys; p = json.load(sys.stdin)["place"]; print(p["listId"], p["viewId"], sep=":")'; }

appearance dark
# Changes on the demo site since the first sync fill Unread.
"$BIN" dev drift >/dev/null
"$BIN" sync >/dev/null
for _ in {1..20}; do sleep 1; "$BIN" status | grep -q "sync: idle" && break; done
"$BIN" go recent >/dev/null
shot unread-dark

# The List with subtasks under their parents, in both appearances.
"$HERE/js.sh" "window.__TAURI_INTERNALS__.invoke('view_set_subtasks', { id: $(view_id NW "All issues"), mode: 'hierarchy' })"
"$BIN" go NW "All issues" >/dev/null
shot list-dark
"$BIN" go NW Board >/dev/null
shot board-dark

# A filter list's Filter popover: its own conditions, a drafted one for the view, and Save View.
"$BIN" go "NW/Open bugs" >/dev/null
KEY=$(narrowing_key)
"$HERE/js.sh" "window.taskboard.getState().setNarrowing('$KEY', { view: [{ field: 'priority', op: 'in', value: ['Highest', 'High'] }] })"
sleep 0.5
"$HERE/js.sh" 'window.__TAURI_INTERNALS__.invoke("run_command", { id: "page:toolbar.filter" })'
shot filter-dark
"$HERE/js.sh" "window.taskboard.getState().clearDrafts('$KEY')"

"$BIN" go NW "All issues" >/dev/null
"$HERE/js.sh" 'window.taskboard.getState().setPaletteOpen(true)'
sleep 0.5
"$HERE/js.sh" '(() => { const input = document.querySelector(".palette input"); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set; set.call(input, "cart"); input.dispatchEvent(new Event("input", { bubbles: true })); })()'
shot palette-dark
"$HERE/js.sh" 'window.taskboard.getState().setPaletteOpen(false)'

appearance light
shot list-light
# A view tab's in-app context menu.
"$HERE/js.sh" '(() => { const tab = [...document.querySelectorAll("[role=tab]")].find((e) => e.textContent.trim() === "Board"); const r = tab.getBoundingClientRect(); tab.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: r.left + r.width / 2, clientY: r.bottom - 4 })); })()'
shot menu-light
"$HERE/js.sh" 'window.taskboard.getState().closeMenu()'
# The Issue view, scrolled to the inline image, subtasks and attachments, finding a word.
"$BIN" open NW-1042 >/dev/null
sleep 2
"$HERE/js.sh" '(() => { const h = [...document.querySelectorAll("h2, h3")].find((e) => e.textContent.trim() === "Web links"); h?.scrollIntoView({ block: "start" }); })()'
# Find in this issue, highlighting its matches.
"$HERE/js.sh" '(() => { const input = document.querySelector(".issue-header input"); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set; set.call(input, "checkout"); input.dispatchEvent(new Event("input", { bubbles: true })); })()'
shot issue-light
"$BIN" go NW Table >/dev/null
shot table-light

appearance dark
# The Settings window, alone on screen so the capture picks it.
"$HERE/js.sh" 'window.__TAURI_INTERNALS__.invoke("run_command", { id: "app:settings:space:10001" })'
sleep 1
"$BIN" hide >/dev/null
shot space-settings-dark
