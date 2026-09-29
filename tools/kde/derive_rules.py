#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Derive a visual-contract rule set from two directories of real gate captures.

The expected-difference rules in expected-visual-diff.json are never guessed and never hand-drawn:
each set is derived from an actual before/after capture pair, and a changed rule is a reviewed change
(the diff of that file is where a reviewer sees which pixels a change is now allowed to move).

usage: derive_rules.py --before DIR --after DIR --set NAME --note TEXT [--json FILE]

Replaces the named set, keeps every other set and the noise profiles, and prints a family summary
(commonest before->after colours) so the reviewer can see at once what kinds of change were recorded.
"""

import argparse
import json
import sys
import time
from collections import Counter
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

from kdeacc import visual  # noqa: E402

HERE = Path(__file__).resolve().parent


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    p.add_argument("--before", type=Path, required=True)
    p.add_argument("--after", type=Path, required=True)
    p.add_argument("--set", dest="set_name", required=True)
    p.add_argument("--note", required=True, help="what the pair is (trees, host), kept beside the set")
    p.add_argument("--json", type=Path, default=HERE / "expected-visual-diff.json")
    args = p.parse_args(argv)
    cd = visual.load_capture_diff(HERE.parents[1])
    derived = visual.derive_set(args.before, args.after, cd)
    data = json.loads(args.json.read_text()) if args.json.exists() else {"schema": 1, "noise_profiles": {}, "sets": {}}
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
