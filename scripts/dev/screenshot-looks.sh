#!/bin/zsh
# Captures the running debug app's window in the current macOS mode (docs/ux.md, Palette checks): the app
# follows macOS with one palette per mode, so switch the system's appearance and run it again for the other.
# screenshot-looks.sh <out-dir>  writes <out-dir>/<mode>.png.
DIR=${1:?usage: screenshot-looks.sh <out-dir>}
HERE=${0:A:h}
MODE=light
[[ $(defaults read -g AppleInterfaceStyle 2>/dev/null) == Dark ]] && MODE=dark
mkdir -p "$DIR"
"$HERE/screenshot.sh" "$DIR/$MODE.png"
