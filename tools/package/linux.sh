#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Build the Linux x86_64 AppImage (Phase 9, ADR 0012).
#
#   dist/rowplay-qt-<version>-linux-x86_64.AppImage (+ .sha256)
#
# Needs the Qt 6.11 `linux_gcc_64` install the README describes (qmake on
# PATH or `QMAKE=/path/to/qmake`), cargo, curl, and — when there is no
# display — `xvfb-run` plus Mesa for the launch check. linuxdeploy and its Qt
# plugin are fetched into tools/package/.cache/ at the pinned releases below
# and verified by SHA-256 before anything runs them.
#
# What "done" means here, in order:
#   1. cargo build --release
#   2. an AppDir with the binary, the desktop entry, the icon, the AppStream
#      metadata and the licence texts
#   3. linuxdeploy + the Qt plugin copy the shared libraries, the platform
#      plugins (xcb and Wayland) and the QML modules the shell imports
#      (scanned from qml/, because the QML itself is compiled into the
#      binary), then appimagetool packs the AppImage
#   4. the AppImage starts from a clean environment, renders 30 frames and
#      exits 0 (tools/package/launch-check.py; under Xvfb + Mesa when headless)
#   5. the SHA-256 is written beside it
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"

# Resolved to an absolute path before it is exported: qtbridge's build
# script treats a set QMAKE as a file path, not a PATH lookup (a bare
# `QMAKE=qmake` made every `cargo build` fail with "could not detect Qt" on
# the first CI run), and linuxdeploy's Qt plugin reads the same variable.
QMAKE=$(command -v "${QMAKE:-qmake}") || { echo "qmake not found; put it on PATH or set QMAKE" >&2; exit 1; }
export QMAKE
ARCH=$(uname -m)
[ "$ARCH" = x86_64 ] || { echo "only x86_64 is packaged (the linuxdeploy pins below are x86_64 builds)" >&2; exit 1; }

VERSION=$(cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="rowplay-app"))')
APP_ID=io.github.shenghaoc.rowplay
DIST="$ROOT/dist"
APPDIR="$DIST/AppDir"
CACHE="$ROOT/tools/package/.cache"
OUTPUT_NAME="rowplay-qt-$VERSION-linux-x86_64.AppImage"

echo "== rowplay-qt $VERSION, Linux $ARCH, Qt $("$QMAKE" -query QT_VERSION)"

# Pinned tools (release tag, SHA-256 of the x86_64 AppImage as measured on
# 2026-09-19). Bump both fields together, deliberately.
# Local-build caveat (measured on RHEL 10, 2026-09-20): the strip bundled in
# this linuxdeploy release cannot parse the `.relr.dyn` sections of
# EL10-era system libraries it deploys (libssl, libsystemd, …) and the run
# aborts loudly. Ubuntu 24.04 (the release runner) is unaffected. On such a
# host, prefix with NO_STRIP=1 for a local, unstripped test build; release
# artifacts still come only from the release workflow.
LINUXDEPLOY_URL=https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage
LINUXDEPLOY_SHA256=c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d
PLUGIN_QT_URL=https://github.com/linuxdeploy/linuxdeploy-plugin-qt/releases/download/1-alpha-20250213-1/linuxdeploy-plugin-qt-x86_64.AppImage
PLUGIN_QT_SHA256=15106be885c1c48a021198e7e1e9a48ce9d02a86dd0a1848f00bdbf3c1c92724

fetch() { # <file name> <url> <sha256>
    local path="$CACHE/$1"
    if [ ! -f "$path" ] || ! echo "$3  $path" | sha256sum -c --status; then
        echo "== fetching $1"
        curl -sSL -o "$path" "$2"
        echo "$3  $path" | sha256sum -c --status || { echo "SHA-256 mismatch for $1" >&2; exit 1; }
    fi
    chmod +x "$path"
}
mkdir -p "$CACHE" "$DIST"
fetch linuxdeploy-x86_64.AppImage "$LINUXDEPLOY_URL" "$LINUXDEPLOY_SHA256"
fetch linuxdeploy-plugin-qt-x86_64.AppImage "$PLUGIN_QT_URL" "$PLUGIN_QT_SHA256"

# 1. release build
cargo build --release -p rowplay-app

# 2. AppDir skeleton
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
    "$APPDIR/usr/share/icons/hicolor/512x512/apps" "$APPDIR/usr/share/metainfo" \
    "$APPDIR/usr/share/licenses/rowplay-qt"
cp target/release/rowplay-app "$APPDIR/usr/bin/rowplay-qt"
cp "packaging/linux/$APP_ID.desktop" "$APPDIR/usr/share/applications/"
cp assets/icon/rowplay-icon-512.png "$APPDIR/usr/share/icons/hicolor/512x512/apps/$APP_ID.png"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@DATE@/$(date -u +%F)/g" \
    "packaging/linux/$APP_ID.metainfo.xml" > "$APPDIR/usr/share/metainfo/$APP_ID.metainfo.xml"
cp LICENSE LICENSES/*.txt ASSET_PROVENANCE.md "$APPDIR/usr/share/licenses/rowplay-qt/"

# 3. deploy Qt and pack
# The tools are AppImages themselves; extract-and-run keeps them working on
# FUSE-less hosts (GitHub's runners). The QML lives inside the binary, so the
# Qt plugin scans the sources for imports instead. Wayland platform plugins
# ride along with xcb when the install has them, because the app runs
# natively under Wayland (README): EXTRA_PLATFORM_PLUGINS names the platform
# plugin file(s), and the wayland-* plugin directories it dlopens are staged
# by hand with their dependencies deployed through --deploy-deps-only.
# (EXTRA_QT_PLUGINS is not the tool for it: a deprecated alias of
# EXTRA_QT_MODULES, which takes Qt module names — measured on the second CI
# run, where the deployer then died on the missing libqwayland-egl.so.)
#
# Which Wayland platform plugin exists depends on the Qt version: Qt 6.11
# ships one `libqwayland.so` (measured on the CI runner's aqt install:
# libqeglfs, libqlinuxfb, libqminimal, libqminimalegl, libqoffscreen,
# libqvkkhrdisplay, libqvnc, libqwayland, libqxcb), older 6.x the
# `libqwayland-egl.so` + `libqwayland-generic.so` pair. Whatever is there is
# deployed; the inventory is printed so a log always states which it was,
# and an install with neither yields an xcb-only AppImage (XWayland on
# Wayland desktops).
QT_PLUGINS=$("$QMAKE" -query QT_INSTALL_PLUGINS)
echo "== platform plugins in $QT_PLUGINS/platforms: $(ls "$QT_PLUGINS/platforms" | tr '\n' ' ')"
DEPLOY_DEPS=()
WAYLAND_PLUGINS=""
for candidate in libqwayland.so libqwayland-egl.so libqwayland-generic.so; do
    [ -f "$QT_PLUGINS/platforms/$candidate" ] && WAYLAND_PLUGINS="${WAYLAND_PLUGINS:+$WAYLAND_PLUGINS;}$candidate"
done
if [ -n "$WAYLAND_PLUGINS" ]; then
    export EXTRA_PLATFORM_PLUGINS="$WAYLAND_PLUGINS"
    # The three plugin directories libqwayland dlopens at runtime. The pinned
    # deployer copied none of them (measured on the fourth CI run: only
    # platforms/, xcbglintegrations/ and what was staged here), and without
    # wayland-shell-integration the platform plugin cannot even open a
    # top-level window, so all three are staged by hand.
    STAGED=""
    for dir in wayland-shell-integration wayland-decoration-client wayland-graphics-integration-client; do
        if [ -d "$QT_PLUGINS/$dir" ]; then
            mkdir -p "$APPDIR/usr/plugins"
            cp -R "$QT_PLUGINS/$dir" "$APPDIR/usr/plugins/"
            DEPLOY_DEPS+=(--deploy-deps-only "$APPDIR/usr/plugins/$dir")
            STAGED="$STAGED $dir"
        else
            echo "warning: $QT_PLUGINS/$dir not found; Wayland support in the AppImage will be incomplete" >&2
        fi
    done
    echo "== Wayland platform plugins: $WAYLAND_PLUGINS deployed alongside xcb; staged:${STAGED:- none}"
else
    echo "== Wayland platform plugins: not in this Qt install; the AppImage is xcb-only (XWayland on Wayland desktops)"
fi
# Linux controls use SVG data URLs for their icons. Stage the image-format
# plugin explicitly: a successful launch does not prove icon pixels rendered.
# Deploy its Qt Svg dependency with linuxdeploy before the Qt plugin packs the
# AppImage. The icon-engine plugin is not needed because icon.source wins over
# icon.name for these controls.
SVG_PLUGIN="$QT_PLUGINS/imageformats/libqsvg.so"
[ -f "$SVG_PLUGIN" ] || { echo "Qt Svg image-format plugin not found: $SVG_PLUGIN" >&2; exit 1; }
mkdir -p "$APPDIR/usr/plugins/imageformats"
cp "$SVG_PLUGIN" "$APPDIR/usr/plugins/imageformats/"
DEPLOY_DEPS+=(--deploy-deps-only "$APPDIR/usr/plugins/imageformats")
# The Qt plugin prints "ERROR: Missing qml module: RowPlay / RowPlay.Replay /
# RowPlay.ReplayAssets" while scanning qml/ — those three are compiled into
# the binary (ADR 0012) and it carries on; the launch check is what proves
# they load.
export APPIMAGE_EXTRACT_AND_RUN=1
export QML_SOURCES_PATHS="$ROOT/qml"
export VERSION
# Absolute on purpose: appimagetool resolves a bare OUTPUT name against its
# own process cwd, and the AppImage runtime does not reliably preserve the
# caller's (measured on RHEL 10 with APPIMAGE_EXTRACT_AND_RUN=1: a bare name
# landed in $HOME while the caller sat in $DIST; GitHub's ubuntu runners
# happen to preserve it). $DIST is where the existence check below looks.
export LDAI_OUTPUT="$DIST/$OUTPUT_NAME" OUTPUT="$DIST/$OUTPUT_NAME"
(cd "$DIST" && "$CACHE/linuxdeploy-x86_64.AppImage" --appdir "$APPDIR" \
    ${DEPLOY_DEPS[@]+"${DEPLOY_DEPS[@]}"} --plugin qt --output appimage)
[ -f "$APPDIR/usr/plugins/imageformats/libqsvg.so" ] || { echo "AppImage missing Qt Svg image plugin" >&2; exit 1; }
[ -n "$(find "$APPDIR/usr/lib" -maxdepth 1 -name 'libQt6Svg.so*' -print -quit)" ] || { echo "AppImage missing Qt Svg library" >&2; exit 1; }
[ -f "$DIST/$OUTPUT_NAME" ] || { echo "linuxdeploy produced no $OUTPUT_NAME in $DIST" >&2; ls -la "$DIST" >&2; exit 1; }

# 4. it starts on its own (xcb; under Xvfb + software Mesa when headless)
check=(python3 tools/package/launch-check.py "$DIST/$OUTPUT_NAME" --platform xcb
       --env APPIMAGE_EXTRACT_AND_RUN=1)
if [ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ]; then
    command -v xvfb-run >/dev/null || { echo "no display and no xvfb-run for the launch check" >&2; exit 1; }
    xvfb-run -a -s "-screen 0 1280x800x24" "${check[@]}" \
        --env QSG_RHI_BACKEND=opengl --env LIBGL_ALWAYS_SOFTWARE=1
else
    "${check[@]}"
fi

# 5. checksum
(cd "$DIST" && sha256sum "$OUTPUT_NAME" > "$OUTPUT_NAME.sha256")
echo "== $(du -h "$DIST/$OUTPUT_NAME" | cut -f1) AppImage"
cat "$DIST/$OUTPUT_NAME.sha256"
