#!/bin/zsh
# Quits the running release or debug Taskboard, whichever `path/to/taskboard` names (each has its own socket
# and data), and waits up to 5 seconds until it is gone.
# Usage: scripts/stop-app.sh path/to/taskboard
source "$(dirname "$0")/app-pids.sh"
app_pids "$1" >/dev/null || exit 0
"$1" quit >/dev/null 2>&1
for _ in {1..50}; do
  app_pids "$1" >/dev/null || exit 0
  sleep 0.1
done
echo "Taskboard did not quit; quit it from the menu bar and try again" >&2
exit 1
