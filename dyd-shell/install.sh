#!/usr/bin/env bash
# Installs the dyland GNOME Shell extension into the user extension directory
# and enables it. The extension does not draw a second island. It hides the
# top-bar clock and stacks the dyland window above the panel.
set -euo pipefail

UUID="dyland@dyland.app"
SRC="$(cd "$(dirname "$0")" && pwd)"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/$UUID"

mkdir -p "$DEST"
cp "$SRC/metadata.json" "$SRC/extension.js" "$SRC/stylesheet.css" "$DEST/"

# GNOME 45+ only scans extension directories when the shell starts. Enabling
# through the running shell fails for a copy that just appeared. Writing the
# setting here makes the next login load it.
remember_extension() {
    command -v gsettings >/dev/null 2>&1 || return 0
    command -v python3 >/dev/null 2>&1 || return 0
    python3 - "$UUID" << 'PY'
import ast, subprocess, sys
uuid = sys.argv[1]

def get(key):
    raw = subprocess.check_output(
        ["gsettings", "get", "org.gnome.shell", key], text=True
    ).strip()
    if raw in ("@as []", "@as[]"):
        return []
    value = ast.literal_eval(raw)
    return list(value) if isinstance(value, list) else []

def put(key, values):
    body = ", ".join("'" + item.replace("'", "") + "'" for item in values)
    subprocess.check_call(["gsettings", "set", "org.gnome.shell", key, f"[{body}]"])

enabled = get("enabled-extensions")
if uuid not in enabled:
    enabled.append(uuid)
    put("enabled-extensions", enabled)
disabled = [item for item in get("disabled-extensions") if item != uuid]
if len(disabled) != len(get("disabled-extensions")):
    put("disabled-extensions", disabled)
PY
}

remember_extension

if command -v gnome-extensions >/dev/null 2>&1 && gnome-extensions enable "$UUID"; then
    echo "Dyland shell integration enabled."
    echo "If the clock is still visible, log out and back in once so GNOME loads the extension."
    exit 0
fi

echo "Dyland extension installed to $DEST"
echo "GNOME loads new extensions only when the shell starts."
echo "It is marked to enable at the next login. Log out and back in once."
