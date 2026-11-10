# Sourced by scripts that start or stop Taskboard. `app_pids path/to/taskboard` lists the app process ids of
# the release or the debug build, whichever the path names, wherever it runs from: its binary without a
# subcommand (the CLI has one).
app_pids() {
  local bundle='Taskboard'
  [[ "$1" == *"Taskboard (Debug).app/"* ]] && bundle='Taskboard \(Debug\)'
  pgrep -f "${bundle}\.app/Contents/MacOS/taskboard( --hidden)?$"
}
