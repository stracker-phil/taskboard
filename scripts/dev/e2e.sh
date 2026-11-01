#!/bin/zsh
# Checks that nothing jumps in the running debug app: e2e.sh [level] [view] (default NW "All issues").
# Opens the view, then clicks Me and Unread only twice each and drags the first row, measuring the content panel,
# the toolbar, the first row and the clicked control after every step. Exits 1 on any drift of 1px or more.
# Start the app first: bun run build:app && scripts/dev/launch.sh
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/src-tauri/target/debug/bundle/macos/Taskboard (Debug).app/Contents/MacOS/taskboard"
LOG="${TMPDIR:-/tmp}/taskboard-dev.log"
MARK="E2E-$(date +%s)-$$"
"$BIN" status >/dev/null 2>&1 || { echo "The debug app isn't running: scripts/dev/launch.sh"; exit 1; }
"$BIN" go "${1:-NW}" "${2:-All issues}" >/dev/null
sleep 1.5

"$ROOT/scripts/dev/js.sh" "(async () => {
  const report = (text) => window.__TAURI_INTERNALS__.invoke('log_frontend_error', { message: '$MARK ' + text });
  const wait = (ms) => new Promise((done) => setTimeout(done, ms));
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return [r.x, r.y, r.width, r.height].map(Math.round).join(',');
  };
  const firstRow = () => document.querySelector('[role=row][data-issue-id]');
  const parts = (control) => ({
    panel: rect(document.querySelector('.content-panel')),
    toolbar: rect(document.querySelector('.toolbar')),
    row: rect(firstRow()),
    control: rect(control),
  });
  const drift = [];
  const compare = (step, before, after) => {
    for (const name of Object.keys(before)) {
      // A filter may leave no rows; only a row that shows on both sides is compared.
      if (name === 'row' && (before.row === null || after.row === null)) continue;
      if (before[name] !== after[name]) drift.push(step + ': ' + name + ' ' + before[name] + ' -> ' + after[name]);
    }
  };
  const clickTwice = async (name, selector) => {
    const control = document.querySelector(selector);
    if (!control) return drift.push(name + ': not found (' + selector + ')');
    const before = parts(control);
    control.click();
    await wait(700);
    compare(name + ' on', before, parts(control));
    control.click();
    await wait(700);
    compare(name + ' off', before, parts(control));
  };
  const drag = async () => {
    const row = firstRow();
    if (!row) return drift.push('drag: no row');
    const before = parts(row);
    const r = row.getBoundingClientRect();
    const at = (dx, dy) => ({ clientX: r.x + 40 + dx, clientY: r.y + r.height / 2 + dy, button: 0, pointerId: 1, bubbles: true });
    row.dispatchEvent(new PointerEvent('pointerdown', at(0, 0)));
    for (const dy of [4, 12, 30, 60]) {
      window.dispatchEvent(new PointerEvent('pointermove', at(20, dy)));
      await wait(60);
    }
    await wait(500);
    compare('dragging', before, parts(row));
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await wait(500);
    compare('drag cancelled', before, parts(row));
  };
  try {
    await clickTwice('Me', '.me-split__toggle');
    await clickTwice('Unread only', '[aria-label=\"Unread only\"]');
    await drag();
    report(drift.length === 0 ? 'OK' : 'DRIFT ' + drift.join(' | '));
  } catch (error) {
    report('ERROR ' + error);
  }
})()"

for _ in {1..30}; do
  sleep 0.5
  line=$(grep -a -o "$MARK [^\"]*" "$LOG" | head -1) && [[ -n "$line" ]] && break
done
result=${line#"$MARK "}
case "$result" in
  OK) echo "No drift"; exit 0 ;;
  "") echo "No result in $LOG"; exit 1 ;;
  *) echo "${result//' | '/$'\n'}"; exit 1 ;;
esac
