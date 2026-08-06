#!/usr/bin/env bash
set -euo pipefail

echo "==> Building dyland AppImage..."

APPDIR="dist/dyland.AppDir"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/icons/hicolor/128x128/apps"

cp target/release/dyd-daemon "${APPDIR}/usr/bin/dyd-daemon"
if [ -f target/release/dyd-ui-tauri ]; then
    cp target/release/dyd-ui-tauri "${APPDIR}/usr/bin/dyd-ui"
elif [ -f target/release/dyland ]; then
    cp target/release/dyland "${APPDIR}/usr/bin/dyd-ui"
fi
cp resources/dyland.desktop "${APPDIR}/dyland.desktop"
cp resources/dyland.desktop "${APPDIR}/usr/share/applications/dyland.desktop"
cp dyd-ui/src-tauri/icons/128x128.png "${APPDIR}/dyd.png"
cp dyd-ui/src-tauri/icons/128x128.png "${APPDIR}/usr/share/icons/hicolor/128x128/apps/dyd.png"

# Create AppRun launcher
cat << 'EOF' > "${APPDIR}/AppRun"
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
if ! pgrep -x "dyd-daemon" >/dev/null 2>&1; then
    "${HERE}/usr/bin/dyd-daemon" &
fi
exec "${HERE}/usr/bin/dyd-ui" "$@"
EOF
chmod +x "${APPDIR}/AppRun"

if command -v appimagetool >/dev/null 2>&1; then
    appimagetool "${APPDIR}" "dist/dyland-0.1.0-x86_64.AppImage"
    echo "==> Created dist/dyland-0.1.0-x86_64.AppImage"
else
    echo "==> AppDir structure prepared in ${APPDIR} (install appimagetool to package into .AppImage binary)"
fi
