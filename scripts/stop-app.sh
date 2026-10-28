#!/bin/zsh
# Quits the running Taskboard of the build that `path/to/taskboard` belongs to (release or debug; each has its
# own socket and data) and waits until it is gone. `running` lists that build's app process ids: its
# binary without a subcommand (the CLI has one).
# Usage: scripts/stop-app.sh path/to/taskboard
case "$1" in
  *"Taskboard (Debug).app/"*) BUNDLE='Taskboard \(Debug\)' ;;
  *) BUNDLE='Taskboard' ;;
esac
running() { pgrep -f "${BUNDLE}\.app/Contents/MacOS/taskboard( --hidden)?$"; }
running >/dev/null || exit 0
"$1" quit >/dev/null 2>&1
for _ in {1..50}; do
  running >/dev/null || exit 0
  sleep 0.1
done
echo "Taskboard did not quit; quit it from the menu bar and try again" >&2
exit 1
