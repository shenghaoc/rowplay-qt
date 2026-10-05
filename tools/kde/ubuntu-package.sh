#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Runs tools/package/linux.sh unchanged inside ubuntu:24.04 (the release runner's OS), rootless podman.
# The container's target dir is bind-mounted over /src/target, apart from the host's and from every other tree's.
# usage: ubuntu-package.sh <repo-or-worktree-dir> <container-target-dir> <log> <qt-dir>
set -euo pipefail
SRC=$1 TGT=$2 LOG=$3
QT=$(cd "$4" && pwd -P)
mkdir -p "$TGT" "$HOME/.cache/rowplay-ubuntu2404-apt"
GITDIR=$(git -C "$SRC" rev-parse --path-format=absolute --git-common-dir)   # a worktree's .git file points here
podman run --rm --security-opt label=disable -v "$GITDIR":"$GITDIR":ro \
  -v "$SRC":/src -v "$TGT":/src/target -v "$QT":"$QT":ro \
  -v "$HOME/.cargo":"$HOME/.cargo" -v "$HOME/.rustup":"$HOME/.rustup" \
  -v "$HOME/.cache/rowplay-ubuntu2404-apt":/var/cache/apt/archives \
  -e "HOME=$HOME" -e "QTDIR=$QT" -e LANG=C.UTF-8 -w /src docker.io/library/ubuntu:24.04 bash -c '
set -e
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends \
  xvfb libgl1-mesa-dri libegl1 libxkbcommon-x11-0 libxcb-cursor0 libxcb-icccm4 \
  libxcb-image0 libxcb-keysyms1 libxcb-randr0 libxcb-render-util0 libxcb-shape0 \
  libxcb-xinerama0 libxcb-xfixes0 libxcb-xkb1 libfontconfig1 \
  libdbus-1-dev pkg-config file build-essential curl ca-certificates python3 xauth git \
  libglib2.0-0t64 libfreetype6 libx11-6 libx11-xcb1 libxcb1 libxkbcommon0 libwayland-client0 libwayland-cursor0 libwayland-egl1 libgl1 libopengl0 libxrender1 libxext6 libsm6 libice6 libgssapi-krb5-2 libssl3t64 libpng16-16t64 libxcb-shm0 libxcb-sync1 libxcb-util1 libxcb-glx0 libgl-dev libegl-dev >/dev/null
export PATH="$HOME/.cargo/bin:$QTDIR/bin:/usr/bin:/bin"
export QMAKE="$QTDIR/bin/qmake"
export LD_LIBRARY_PATH="$QTDIR/lib"
git config --global --add safe.directory "*"
grep PRETTY /etc/os-release; rustc --version; qmake -query QT_VERSION
env -u DISPLAY -u WAYLAND_DISPLAY tools/package/linux.sh
' > "$LOG" 2>&1
