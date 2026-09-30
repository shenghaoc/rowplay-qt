# SPDX-License-Identifier: GPL-3.0-or-later
"""Where a directory of gate captures came from: the commit, and whether the tree was clean.

The visual rules for "the baseline against the branch" describe a change made on top of one commit (ADR 0018:
046c2e3). Applied to captures taken at any other commit they would fail with dozens of misleading "the expected change
is missing" results, or worse pass wrongly. So the native stage writes `provenance.json` beside the captures it takes,
and the visual stage checks it against the rule set's `baseline_sha` before it applies a single rule. It reads
the captures' own record, never a git worktree: captures copied to a machine with one checkout can be classified.
"""

from __future__ import annotations

import json
import re
import time
from pathlib import Path

PROVENANCE_FILE = "provenance.json"
SHA_RE = re.compile(r"^[0-9a-f]{40}$")


class EvidenceError(ValueError):
    """The evidence directory does not say what it must."""


def write(directory, role, state, gate_profile="full", note=""):
    """Record what `directory` holds. `state` is stages.git_state's dict for the tree the captures were taken from."""
    record = {"schema": 1, "role": role, "commit": state["sha"], "branch": state.get("branch"), "dirty": bool(state["dirty"]),
              "gate_profile": gate_profile, "written_by": "tools/kde native stage", "written": time.strftime("%Y-%m-%dT%H:%M:%S%z")}
    if note:
        record["note"] = note
    Path(directory).mkdir(parents=True, exist_ok=True)
    (Path(directory) / PROVENANCE_FILE).write_text(json.dumps(record, indent=2) + "\n")
    return record


def read(directory):
    """The provenance record, or None when the directory has none. A malformed record is an EvidenceError."""
    path = Path(directory) / PROVENANCE_FILE
    if not path.exists():
        return None
    try:
        record = json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        raise EvidenceError(f"{path} is not valid JSON: {exc}") from exc
    if not isinstance(record, dict) or not SHA_RE.match(str(record.get("commit", ""))):
        raise EvidenceError(f"{path} has no 40-hex `commit`")
    return record


def check_baseline_captures(directory, rules, expected_sha):
    """[(name, ok, detail)] proving that the baseline captures in `directory` are the ones `rules` were derived against.

    `expected_sha` is the baseline this run compares with (baseline.BASELINE_SHA unless --baseline-sha says otherwise)."""
    checks = []
    derived = rules.get("baseline_sha")
    checks.append(("the rule set names the baseline it was derived against", bool(derived and SHA_RE.match(derived)),
                   f"baseline_sha {str(derived)[:7]}",
                   "the rule set has no `baseline_sha`: it cannot be applied to a baseline it does not name"))
    if not (derived and SHA_RE.match(derived)):
        return checks
    checks.append(("the rules' baseline is this run's acceptance baseline", derived == expected_sha,
                   f"both are {derived[:7]}",
                   f"baseline mismatch: the rules were derived against {derived[:7]} but this run compares with {expected_sha[:7]}"))
    try:
        record = read(directory)
    except EvidenceError as exc:
        checks.append(("the baseline captures carry readable provenance", False, "", f"{exc}"))
        return checks
    if record is None:
        checks.append(("the baseline captures carry provenance", False, "",
                       f"the baseline captures in {directory} carry no {PROVENANCE_FILE}, so it cannot be shown they were taken at {derived[:7]} "
                       f"(the native stage writes one beside every capture directory; captures from an older harness need a hand-written "
                       f'{{"role": "baseline", "commit": "<40-hex sha>", "dirty": false}})'))
        return checks
    checks.append(("the baseline captures were taken at the rules' baseline commit", record["commit"] == derived,
                   f"captured at {record['commit'][:7]}",
                   f"baseline mismatch: these captures were taken at {record['commit'][:7]}, the rules were derived against {derived[:7]}"))
    checks.append(("the baseline captures come from a clean tree", not record.get("dirty", False),
                   "clean", "the baseline tree had uncommitted changes when the captures were taken"))
    if record.get("role") not in (None, "baseline"):
        checks.append(("the captures are the baseline's", False, "", f"the directory's provenance says role {record.get('role')!r}, not 'baseline'"))
    return checks
