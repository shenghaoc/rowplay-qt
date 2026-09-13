#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# One-command venue bake (Phase 6a, ADR 0010).
#
#   tools/bake-venues/bake.sh            # bake, Blender pass, finalize
#   tools/bake-venues/bake.sh --verify   # also re-bake one variant byte-for-byte
#
# Requires: Node >= 24 and `pnpm install` in reference/rowplay (for three.js);
# Blender on PATH or BLENDER_BIN set to the executable.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO"

BLENDER="${BLENDER_BIN:-blender}"
if ! command -v "$BLENDER" >/dev/null 2>&1; then
  echo "bake: Blender not found; set BLENDER_BIN to the blender executable" >&2
  exit 1
fi
if [ ! -d reference/rowplay/node_modules/three ]; then
  echo "bake: run 'pnpm install' in reference/rowplay first (three.js is required)" >&2
  exit 1
fi

STAGED=build/venues-staging
CLEANED=build/venues-cleaned
VERIFY=()
if [ "${1:-}" = "--verify" ]; then VERIFY=(--verify); fi

echo "==> bake (Node, per-variant processes)"
rm -rf "$STAGED" "$CLEANED"
node --experimental-transform-types --import ./tools/bake-venues/register.mjs \
  tools/bake-venues/bake.mjs --all "${VERIFY[@]}"

echo "==> Blender hygiene pass"
mkdir -p "$CLEANED"
PYTHONHASHSEED=0 "$BLENDER" --background --threads 1 \
  --python tools/bake-venues/cleanup.py -- \
  --input "$STAGED" --output "$CLEANED" --report build/venues-cleanup-report.json

echo "==> finalize (re-derive inventory + bounds from the cleaned GLBs)"
cp "$CLEANED"/*.glb "$STAGED"/
node --experimental-transform-types --import ./tools/bake-venues/register.mjs \
  tools/bake-venues/bake.mjs --finalize --out "$STAGED"

echo "==> done: $STAGED"
