#!/usr/bin/env bash
# Installs the Dyland GNOME Shell extension and enables it.
# A session restart (log out / log in) is required for the shell to load a
# newly installed extension.
set -euo pipefail

UUID="dyland@dyland.app"
SRC="$(cd "$(dirname "$0")" && pwd)"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/$UUID"

mkdir -p "$DEST"
cp "$SRC/metadata.json" "$SRC/extension.js" "$SRC/stylesheet.css" "$DEST/"

# Pre-enable for the next session, keeping the existing list intact.
CURRENT="$(gsettings get org.gnome.shell enabled-extensions)"
if [[ "$CURRENT" != *"$UUID"* ]]; then
    CLEAN="${CURRENT//[\[\] ]/}"
    if [[ -z "$CLEAN" ]]; then
        NEW="['$UUID']"
    else
        NEW="[$CLEAN,'$UUID']"
    fi
    gsettings set org.gnome.shell enabled-extensions "$NEW"
fi

echo "Dyland extension installed to $DEST"
echo "Log out and back in (or restart gnome-shell) to activate it."
