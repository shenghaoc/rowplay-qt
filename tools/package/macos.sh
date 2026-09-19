#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Build rowplay-qt.app and a .dmg on macOS (Phase 9).
#
#   dist/rowplay-qt.app                                the deployed bundle
#   dist/rowplay-qt-<version>-macos-<arch>.dmg         compressed disk image
#   dist/rowplay-qt-<version>-macos-<arch>.dmg.sha256
#
# Needs a Qt 6.11 framework install with `macdeployqt` (found through
# `qmake` on PATH, or `QMAKE=/path/to/qmake`), cargo, and Xcode's command
# line tools (`codesign`, `hdiutil`, `otool`). Signing: ad-hoc unless
# `ROWPLAY_MAC_SIGN_IDENTITY` names a Developer ID certificate in the
# keychain (then hardened runtime + secure timestamp, notarisation-ready).
#
# What "done" means here, in order:
#   1. cargo build --release
#   2. assemble the bundle skeleton (Info.plist, icon, PkgInfo, licences)
#   3. macdeployqt copies the frameworks, plugins and QML modules the shell
#      imports (scanned from qml/) and rewrites the load commands
#   4. no Mach-O in the bundle may reference the Qt install by absolute
#      path (otool scan) — the build machine's Qt must not be a hidden input
#   5. the deployed binary starts from a clean environment, renders 30
#      frames and exits 0 (tools/package/launch-check.py)
#   6. hdiutil packs the .dmg and its SHA-256 is written beside it
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"

# An absolute path, like linux.sh: qtbridge's build script treats a set
# QMAKE as a file path (not exported here, but a caller's bare value would
# already have broken cargo).
QMAKE=$(command -v "${QMAKE:-qmake}") || { echo "qmake not found; put it on PATH or set QMAKE" >&2; exit 1; }
QT_BINS=$("$QMAKE" -query QT_INSTALL_BINS)
QT_LIBS=$("$QMAKE" -query QT_INSTALL_LIBS)
QT_PREFIX=$("$QMAKE" -query QT_INSTALL_PREFIX)
MACDEPLOYQT="$QT_BINS/macdeployqt"
[ -x "$MACDEPLOYQT" ] || { echo "macdeployqt not found at $MACDEPLOYQT" >&2; exit 1; }

VERSION=$(cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="rowplay-app"))')
MIN_MACOS=$(sed -n 's/^QMAKE_MACOSX_DEPLOYMENT_TARGET *= *//p' "$QT_PREFIX/mkspecs/qconfig.pri")
[ -n "$MIN_MACOS" ] || { echo "QMAKE_MACOSX_DEPLOYMENT_TARGET not found in $QT_PREFIX/mkspecs/qconfig.pri" >&2; exit 1; }
ARCH=$(uname -m)
DIST="$ROOT/dist"
APP="$DIST/rowplay-qt.app"
DMG="$DIST/rowplay-qt-$VERSION-macos-$ARCH.dmg"

echo "== rowplay-qt $VERSION, macOS $ARCH, Qt $("$QMAKE" -query QT_VERSION), minimum macOS $MIN_MACOS"

# 1. release build (build.rs emits the @executable_path/../Frameworks rpath)
cargo build --release -p rowplay-app

# 2. bundle skeleton
rm -rf "$APP" && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/licenses"
cp target/release/rowplay-app "$APP/Contents/MacOS/rowplay-qt"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@MIN_MACOS@/$MIN_MACOS/g" \
    packaging/macos/Info.plist > "$APP/Contents/Info.plist"
cp assets/icon/rowplay-qt.icns "$APP/Contents/Resources/"
printf 'APPL????' > "$APP/Contents/PkgInfo"
cp LICENSE LICENSES/*.txt ASSET_PROVENANCE.md "$APP/Contents/Resources/licenses/"

# 3. deploy Qt into the bundle
SIGN_ARGS=()
if [ -n "${ROWPLAY_MAC_SIGN_IDENTITY:-}" ]; then
    SIGN_ARGS=(-codesign="$ROWPLAY_MAC_SIGN_IDENTITY" -hardened-runtime -timestamp)
fi
# (the `+` expansion keeps macOS's bash 3.2 happy about an empty array under set -u)
"$MACDEPLOYQT" "$APP" -qmldir="$ROOT/qml" -libpath="$QT_LIBS" -always-overwrite -verbose=1 ${SIGN_ARGS[@]+"${SIGN_ARGS[@]}"}

# macdeployqt copies the whole QtQuick QML tree, and with it QtQmlLocalStorage
# and every QtSql driver plugin — including the ODBC, PostgreSQL and Mimer
# drivers, which link libraries that exist on nobody's machine (they are
# what the "can't open file: /usr/local/lib/libmimerapi.dylib" errors above
# are about). Nothing in rowplay-qt uses QtSql (the cache is rusqlite), so
# the drivers go. Everything else stays: trimming unused QML modules is an
# optimisation with a runtime-load failure mode and is tracked in the Phase
# 9 spec, not done blind here.
rm -rf "$APP/Contents/PlugIns/sqldrivers"

# 4. nothing in the bundle may still point at the Qt install
leaks=0
while IFS= read -r -d '' macho; do
    if otool -l "$macho" 2>/dev/null | grep -F "$QT_LIBS" >/dev/null; then
        echo "still references the Qt install: ${macho#"$APP"/}" >&2
        leaks=$((leaks + 1))
    fi
done < <(find "$APP" -type f \( -perm -u+x -o -name '*.dylib' \) -print0)
if [ "$leaks" -ne 0 ]; then
    echo "$leaks Mach-O file(s) reference $QT_LIBS; the bundle is not self-contained" >&2
    exit 1
fi
echo "== otool scan: no Mach-O references $QT_LIBS"

# 5. it starts on its own — under cocoa, the only platform plugin the bundle
# carries (macdeployqt deploys no offscreen plugin), so a window flashes up
# for a couple of seconds. ROWPLAY_LAUNCH_PLATFORM overrides for experiments.
python3 tools/package/launch-check.py "$APP/Contents/MacOS/rowplay-qt" \
    ${ROWPLAY_LAUNCH_PLATFORM:+--platform "$ROWPLAY_LAUNCH_PLATFORM"}

# 6. disk image + checksum
rm -f "$DMG"
hdiutil create -quiet -volname "rowplay $VERSION" -srcfolder "$APP" -ov -format UDZO "$DMG"
(cd "$DIST" && shasum -a 256 "$(basename "$DMG")" > "$(basename "$DMG").sha256")
echo "== $(du -sh "$APP" | cut -f1) bundle, $(du -h "$DMG" | cut -f1) dmg"
cat "$DMG.sha256"
