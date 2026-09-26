#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Pins the CI change scope (tools/ci/change-scope.sh) case by case. Runs in
# the CI workflow's Change scope job before the classifier's verdict is
# used, and locally with no arguments. Needs bash and git only.
set -euo pipefail

scope="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/change-scope.sh"
failures=0

# expect <true|false> <case name> <path>...: the paths, one change list.
expect() {
  local want=$1 name=$2
  shift 2
  local got
  got=$(printf '%s\n' "$@" | "$scope" 2>/dev/null)
  if [ "$got" = "code=$want" ]; then
    echo "ok    code=$want  $name"
  else
    echo "FAIL  $name: expected code=$want, got '$got'"
    failures=$((failures + 1))
  fi
}

# Docs-only: no Rust, Qt or visual gate.
expect false "PR #135's paths (docs/, .kiro/, AGENTS.md, tools/blender/README.md)" \
  docs/roadmap.md docs/decisions/0017-athlete-contact.md docs/blender-audit.md \
  .kiro/specs/blender-05-athlete-contact/tasks.md AGENTS.md tools/blender/README.md
expect false "tools/bake-venues/README.md" tools/bake-venues/README.md
expect false "tools/README.md" tools/README.md
expect false "root README, shims" README.md CLAUDE.md GEMINI.md
expect false "licences" LICENSE LICENSES/MIT.txt LICENSES/GPL-3.0-or-later.txt
expect false "pull request template" .github/pull_request_template.md

# Code: the full CI.
expect true "empty change list" ""
expect true "crates/" crates/rowplay-core/src/lib.rs
expect true "crates/ Markdown" crates/rowplay-core/README.md
expect true "qml/" qml/RowPlay/Main.qml
expect true "qrc" qml/rowplay.qrc
expect true "Python tool" tools/vendor-fixtures.py
expect true "Node tool" tools/convert-locales.mjs
expect true "Blender script" tools/blender/build_all.py
expect true "bake script" tools/bake-venues/bake.mjs
expect true "packaging script" tools/package/linux.sh
expect true "the classifier itself" tools/ci/change-scope.sh
expect true "tool Markdown other than a README" tools/blender/notes.md
expect true "a README-like name" tools/blender/README.md.orig
expect true "a README-like name, prefixed" tools/blender/OLD-README.md
expect true "lower-case readme" tools/blender/readme.md
expect true "workflow" .github/workflows/ci.yml
expect true "other workflow" .github/workflows/release.yml
expect true "other .github file" .github/CODEOWNERS
expect true "Cargo.toml" Cargo.toml
expect true "Cargo.lock" Cargo.lock
expect true "rust-toolchain.toml" rust-toolchain.toml
expect true ".cargo/config.toml" .cargo/config.toml
expect true "ASSET_PROVENANCE.md (asset_hashes.rs reads it)" ASSET_PROVENANCE.md
expect true "tests/fixtures/PROVENANCE.md" tests/fixtures/PROVENANCE.md
expect true "tests/fixtures/Concept2/REDACTION.md" tests/fixtures/Concept2/REDACTION.md
expect true "fixture JSON" tests/fixtures/manifest.json
expect true "assets/README.md" assets/README.md
expect true "assets/replay/venues/README.md" assets/replay/venues/README.md
expect true "assets/replay/environments/README.md" assets/replay/environments/README.md
expect true "venue manifest" assets/replay/venues/MANIFEST.json
expect true "i18n/README.md" i18n/README.md
expect true "i18n catalogue" i18n/rowplay_de.ts
expect true "packaging manifest" packaging/linux/rowplay-qt.desktop
expect true "a directory named like a docs path" crates/docs/x.rs
expect true "LICENSE as a prefix" LICENSE.txt
expect true "mixed: docs plus one code path" \
  docs/roadmap.md AGENTS.md tools/blender/README.md crates/rowplay-app/build.rs
expect true "mixed: docs plus ASSET_PROVENANCE.md" docs/roadmap.md ASSET_PROVENANCE.md
expect true "quoted (non-ASCII) path" '"docs/\303\244.md"'

# Renames, through git itself: --diff must list a rename's old path too.
repo=$(mktemp -d)
trap 'rm -rf "$repo"' EXIT
(
  cd "$repo"
  git init -q -b main
  git config user.email ci@example.invalid
  git config user.name ci
  mkdir -p crates docs tools/blender assets
  echo 'fn main() {}' > crates/main.rs
  echo 'provenance' > assets/NOTES.md
  echo 'docs' > docs/a.md
  git add -A && git commit -qm base
) >/dev/null

# rename_case <true|false> <name> <commands run on a branch off main>
rename_case() {
  local want=$1 name=$2 commands=$3 got
  got=$(
    cd "$repo"
    git checkout -q -B "case-$RANDOM" main
    eval "$commands"
    git commit -qam "$name"
    "$scope" --diff main 2>/dev/null
  )
  if [ "$got" = "code=$want" ]; then
    echo "ok    code=$want  $name"
  else
    echo "FAIL  $name: expected code=$want, got '$got'"
    failures=$((failures + 1))
  fi
}

rename_case true "rename crates/ -> docs/" "git mv crates/main.rs docs/main.rs"
rename_case true "rename crates/ -> tools/blender/README.md" \
  "git mv crates/main.rs tools/blender/README.md"
rename_case true "rename assets/*.md -> docs/" "git mv assets/NOTES.md docs/NOTES.md"
rename_case false "rename within docs/" "git mv docs/a.md docs/b.md"
rename_case false "edit docs/ through git" "echo more >> docs/a.md"

if [ "$failures" -ne 0 ]; then
  echo "$failures change-scope case(s) failed"
  exit 1
fi
echo "all change-scope cases passed"
