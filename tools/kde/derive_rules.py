#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Derive a visual-contract rule set from two directories of real gate captures.

The expected-difference rules in expected-visual-diff.json are never guessed and never hand-drawn:
each set is derived from an actual before/after capture pair, and a changed rule is a reviewed change
(the diff of that file is where a reviewer sees which pixels a change is now allowed to move).

usage: derive_rules.py --before DIR --after DIR --set NAME --note TEXT [--baseline-sha SHA] [--json FILE]
       derive_rules.py --set NAME --stamp-baseline SHA [--json FILE]

Replaces the named set, keeps every other set and the noise profiles, and prints a family summary
(commonest before->after colours) so the reviewer can see at once what kinds of change were recorded.

A set whose `before` is the acceptance baseline records the commit it was derived against as `baseline_sha`; the visual
stage refuses to apply it to captures that were not taken at that commit. The commit comes from the `before`
directory's own provenance.json (the native stage writes it) and, if --baseline-sha is also given, must agree with it;
a `before` directory with no provenance.json needs --baseline-sha. A set derived between two states of one tree
(accent-vs-default) names no baseline.

--stamp-baseline SHA only records `baseline_sha` on an existing set (metadata; no coordinate is touched), for a set
that was derived before provenance existed. It refuses a set that is missing or a sha that is not 40 hex digits.
"""

import argparse
import json
import sys
import time
from collections import Counter
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

from kdeacc import provenance, visual  # noqa: E402

HERE = Path(__file__).resolve().parent


def baseline_of(before, given):
    """The baseline commit for a set: the `before` directory's provenance, checked against --baseline-sha; None for a same-tree set."""
    record = provenance.read(before)
    recorded = record["commit"] if record else None
    if given and not provenance.SHA_RE.match(given):
        raise SystemExit(f"--baseline-sha {given!r} is not a 40-hex commit")
    if recorded and given and recorded != given:
        raise SystemExit(f"--baseline-sha {given[:7]} disagrees with the captures' own provenance ({recorded[:7]}): {before}")
    if recorded and record.get("dirty"):
        raise SystemExit(f"the captures in {before} were taken from a dirty tree: rules must not be derived from them")
    return recorded or given


def stamp(json_path, set_name, sha):
    if not provenance.SHA_RE.match(sha):
        raise SystemExit(f"{sha!r} is not a 40-hex commit")
    data = json.loads(json_path.read_text())
    if set_name not in data["sets"]:
        raise SystemExit(f"no set {set_name!r} in {json_path}: have {sorted(data['sets'])}")
    data["sets"][set_name] = {"baseline_sha": sha, **{k: v for k, v in data["sets"][set_name].items() if k != "baseline_sha"}}
    json_path.write_text(json.dumps(data, indent=1) + "\n")
    print(f"{set_name}: baseline_sha {sha}")
    return 0


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    p.add_argument("--before", type=Path)
    p.add_argument("--after", type=Path)
    p.add_argument("--set", dest="set_name", required=True)
    p.add_argument("--note", help="what the pair is (trees, host), kept beside the set")
    p.add_argument("--baseline-sha", help="the acceptance baseline commit the `before` captures were taken at (checked against their provenance.json)")
    p.add_argument("--stamp-baseline", metavar="SHA", help="record baseline_sha on an existing set without deriving anything")
    p.add_argument("--json", type=Path, default=HERE / "expected-visual-diff.json")
    args = p.parse_args(argv)
    if args.stamp_baseline:
        return stamp(args.json, args.set_name, args.stamp_baseline)
    if not (args.before and args.after and args.note):
        p.error("--before, --after and --note are required to derive a set")
    cd = visual.load_capture_diff(HERE.parents[1])
    derived = visual.derive_set(args.before, args.after, cd)
    data = json.loads(args.json.read_text()) if args.json.exists() else {"schema": 1, "noise_profiles": {}, "sets": {}}
    sha = baseline_of(args.before, args.baseline_sha)
    if sha:
        derived = {"baseline_sha": sha, **derived}
    derived["derived"] = {"note": args.note, "before": args.before.name, "after": args.after.name, "date": time.strftime("%Y-%m-%d"),
                          "captures_with_a_change": len(derived["captures"]),
                          "captures_compared": len(list(args.after.glob("*.ppm")))}
    data["sets"][args.set_name] = derived
    args.json.write_text(json.dumps(data, indent=1) + "\n")
    families = Counter()
    for name, rule in derived["captures"].items():
        top = rule["colours_observed"][0][:2] if rule["colours_observed"] else None
        families[(tuple(top) if top else None, bool(rule.get("bands")))] += 1
    print(f"{args.set_name}: {len(derived['captures'])} of {derived['derived']['captures_compared']} captures carry a rule")
    for (colours, ring), n in families.most_common():
        print(f"  {n:3} captures, commonest change {colours[0]} -> {colours[1]}" + (" (includes a ring band)" if ring else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
