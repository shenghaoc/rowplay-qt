#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# CI change scope: does a pull request touch code, or only documentation?
# Prints `code=true` or `code=false` (the CI workflow appends it to
# $GITHUB_OUTPUT); AGENTS.md, "Continuous integration", states the policy.
#
#   tools/ci/change-scope.sh --diff <base-ref>   classify <base-ref>...HEAD
#   tools/ci/change-scope.sh < paths             classify one path per line
#
# A path is documentation only if no build, test or tool reads it. The
# allow-list names those paths; everything else is code, and so is an empty
# change list. A path on the deny-list is code even where the allow-list
# matches it: ASSET_PROVENANCE.md is a root *.md, but asset_hashes.rs reads
# it as the asset contract. Markdown under assets/, tests/fixtures/, i18n/ or
# anywhere else not named here is code, since the asset and fixture manifest
# tests police those directories. Tool READMEs are documentation: no script
# or test reads them (checked 2026-09-26; widen or narrow this only with the
# same check). A path git quotes (non-ASCII, say) matches nothing, so it
# counts as code. tools/ci/test-change-scope.sh pins every case.
set -euo pipefail

docs='^(docs/|\.kiro/|LICENSES/|LICENSE$|\.github/pull_request_template\.md$|[^/]+\.md$|tools/([^/]+/)*README\.md$)'
deny='^ASSET_PROVENANCE\.md$'

if [ "${1:-}" = "--diff" ]; then
  [ -n "${2:-}" ] || { echo "usage: $0 --diff <base-ref>" >&2; exit 2; }
  # --no-renames: a rename must count its old path too. With rename
  # detection, moving a code file under docs/ lists only the new path and
  # would pass as docs-only.
  changed=$(git diff --no-renames --name-only "$2...HEAD")
else
  changed=$(cat)
fi
printf '%s\n' "$changed" >&2

code=false
if [ -z "$changed" ]; then
  code=true
else
  while IFS= read -r path; do
    if grep -qE "$deny" <<< "$path" || ! grep -qE "$docs" <<< "$path"; then
      echo "code: $path" >&2
      code=true
    fi
  done <<< "$changed"
fi
echo "code=$code"
