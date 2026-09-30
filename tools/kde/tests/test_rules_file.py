# SPDX-License-Identifier: GPL-3.0-or-later
"""The committed expected-visual-diff.json is data a reviewer trusts: audit its shape."""

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.dont_write_bytecode = True

from kdeacc import baseline, provenance, visual  # noqa: E402

RULES = Path(__file__).resolve().parents[1] / "expected-visual-diff.json"


class RulesFile(unittest.TestCase):
    def setUp(self):
        self.data = json.loads(RULES.read_text())

    def test_the_noise_profile_records_its_calibration(self):
        profile = self.data["noise_profiles"]["native-hardware"]
        self.assertEqual(profile["max_delta"], 1)
        self.assertGreater(profile["max_pixels_2d"], 450)   # the measured same-tree maximum
        self.assertGreater(profile["max_pixels_3d"], 76)
        self.assertIn("calibration", profile)
        self.assertIn("unchanged", profile["calibration"])  # says the global bound is untouched

    def test_both_sets_load_with_the_profile(self):
        for name in ("baseline-vs-branch", "accent-vs-default"):
            rules, profile = visual.load_rules(RULES, name, "native-hardware")
            self.assertTrue(rules["captures"], name)
            self.assertEqual(profile["max_delta"], 1)
            self.assertTrue(rules["derived"]["note"])

    def test_the_baseline_relative_set_names_the_acceptance_baseline_and_the_same_tree_set_names_none(self):
        rules, _ = visual.load_rules(RULES, "baseline-vs-branch")
        self.assertRegex(rules["baseline_sha"], provenance.SHA_RE.pattern)
        self.assertEqual(rules["baseline_sha"], baseline.BASELINE_SHA)   # one definition of the baseline, shared with the harness
        self.assertIn("046c2e3", rules["derived"]["note"])                # ... and the prose still says the same thing
        self.assertNotIn("baseline_sha", visual.load_rules(RULES, "accent-vs-default")[0])

    def test_every_rule_is_a_well_formed_area_with_a_change_it_must_show(self):
        for name, rset in self.data["sets"].items():
            for capture, rule in rset["captures"].items():
                w, h = rule["size"]
                self.assertTrue(rule["must_change"], (name, capture))
                self.assertTrue(rule["regions"] or rule.get("bands"), (name, capture))
                for x0, y0, x1, y1 in rule["regions"] + [b["outer"] for b in rule.get("bands", [])]:
                    self.assertTrue(0 <= x0 <= x1 < w and 0 <= y0 <= y1 < h, (name, capture, (x0, y0, x1, y1), (w, h)))
                for band in rule.get("bands", []):
                    self.assertGreaterEqual(band["thickness"], 1)
                    x0, y0, x1, y1 = band["outer"]
                    self.assertLess(band["thickness"] * 2, min(x1 - x0, y1 - y0))  # a ring, not a filled box

    def test_the_baseline_vs_branch_set_is_the_three_expected_families_only(self):
        caps = self.data["sets"]["baseline-vs-branch"]["captures"]
        self.assertEqual(len(caps), 40)
        families = {"scrubber": 0, "settings icon": 0, "ring": 0}
        for name, rule in caps.items():
            top = rule["colours_observed"][0][:2]
            if rule.get("bands"):
                families["ring"] += 1
            elif top == ["000000", "3daee9"]:
                families["scrubber"] += 1
                self.assertTrue(name.startswith(("phase-", "replay-gap-")), name)   # the replay's scrubber fill
            else:
                families["settings icon"] += 1
                self.assertEqual(top[0], "232629", name)                           # a dark glyph replaced by the surface
                (x0, y0, x1, y1), = rule["regions"]
                self.assertLessEqual((x1 - x0, y1 - y0), (20, 20), name)            # one 16 px icon and its margin
        self.assertEqual(families, {"scrubber": 30, "settings icon": 9, "ring": 1})


if __name__ == "__main__":
    unittest.main()
