# SPDX-License-Identifier: GPL-3.0-or-later
"""The environment export's pure helpers, without Blender."""

import json
from pathlib import Path
import unittest

import export_environment as ee

ASSETS = Path(__file__).resolve().parents[2] / "assets/replay"


class ExportEnvironmentTest(unittest.TestCase):
    def test_structure_footprints_come_from_the_venue(self):
        footprints = ee._venue_footprints(ASSETS)
        self.assertEqual(set(footprints), set(ee.STRUCTURES))
        r0, r1, a0, a1 = footprints["wetland-boardwalk-deck"]
        self.assertAlmostEqual(r0, 38.6, delta=0.05)
        self.assertAlmostEqual(r1, 40.3, delta=0.05)
        self.assertAlmostEqual(a0, 304.0, delta=0.5)
        self.assertAlmostEqual(a1, 350.0, delta=0.5)
        # The finish tower stands at the quay, 52 degrees round the loop.
        r0, r1, a0, a1 = footprints["finish-tower"]
        self.assertTrue(r0 < 37.0 < r1 and a0 < 52.0 < a1)

    def test_footprints_keep_their_margin(self):
        deck = ee._venue_footprints(ASSETS)["wetland-boardwalk-deck"]
        import math
        inside = (39.4 * math.sin(math.radians(327)), 39.4 * math.cos(math.radians(327)))
        beside = (37.7 * math.sin(math.radians(327)), 37.7 * math.cos(math.radians(327)))
        self.assertTrue(ee._inside(deck, *inside, 0.3))
        self.assertFalse(ee._inside(deck, *beside, 0.3))
        self.assertTrue(ee._inside(deck, *beside, 2.0))

    def test_tints_are_encoded_as_srgb(self):
        self.assertAlmostEqual(ee._srgb(0.0), 0.0)
        self.assertAlmostEqual(ee._srgb(1.0), 1.0)
        self.assertAlmostEqual(ee._srgb(0.214041), 0.5, places=5)

    def test_one_instance_per_line(self):
        entries = [{"variant": "reeds", "tier": 0, "name": "reeds.001", "position": [1.0, 0.0, 36.5],
                    "yaw": 12.5, "scale": 1.0, "color": "#eeeeee"},
                   {"variant": "shrub", "tier": 1, "name": "shrub.002", "position": [2.0, 0.3, 45.0],
                    "yaw": 0.0, "scale": 0.8, "color": "#dddddd"}]
        text = ee.placement_json(entries)
        self.assertEqual(len(text.splitlines()), len(entries) + 2)
        parsed = json.loads(text)
        self.assertEqual([e["variant"] for e in parsed], ["reeds", "shrub"])
        self.assertNotIn("name", parsed[0])


if __name__ == "__main__":
    unittest.main()
