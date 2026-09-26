# SPDX-License-Identifier: GPL-3.0-or-later
"""The environment export's pure helpers, without Blender."""

import json
import math
from pathlib import Path
import unittest

import export_environment as ee

ASSETS = Path(__file__).resolve().parents[2] / "assets/replay"


class ExportEnvironmentTest(unittest.TestCase):
    def test_structure_footprints_come_from_the_dressing(self):
        footprints = ee._dressing_footprints(ASSETS)
        self.assertIn("finish-tower", footprints)
        self.assertIn("wetland-boardwalk", footprints)
        r0, r1, a0, a1 = footprints["wetland-boardwalk"]
        self.assertTrue(38.0 < r0 < 39.0 < 40.5 < r1 < 41.5)
        self.assertTrue(302.0 < a0 < 304.0 and 351.0 < a1 < 353.0)
        # The finish tower stands at the quay, 52 degrees round the loop.
        r0, r1, a0, a1 = footprints["finish-tower"]
        self.assertTrue(r0 < 37.0 < r1 and a0 < 52.0 < a1)
        # The island is a ring round the whole basin.
        self.assertEqual(footprints["island"][2:], (0.0, 360.0))

    def test_footprints_keep_their_margin(self):
        deck = ee._dressing_footprints(ASSETS)["wetland-boardwalk"]
        inside = (39.6 * math.sin(math.radians(327)), 39.6 * math.cos(math.radians(327)))
        beside = (37.7 * math.sin(math.radians(327)), 37.7 * math.cos(math.radians(327)))
        self.assertTrue(ee._inside(deck, *inside, 0.3))
        self.assertFalse(ee._inside(deck, *beside, 0.3))
        self.assertTrue(ee._inside(deck, *beside, 2.0))
        # A sector past 360 degrees straddles the loop's 0.
        self.assertTrue(ee._inside((39.0, 41.0, 350.0, 370.0), 40.0 * math.sin(math.radians(5.0)), 40.0 * math.cos(math.radians(5.0)), 0.3))
        self.assertFalse(ee._inside((39.0, 41.0, 350.0, 370.0), 40.0 * math.sin(math.radians(20.0)), 40.0 * math.cos(math.radians(20.0)), 0.3))
        self.assertTrue(ee._inside((0.0, 16.5, 0.0, 360.0), 3.0, -4.0, 0.3))

    def test_the_waterline_holds_between_vertices(self):
        # A sunken vertex inside 36.2 m joined to dry ones outside it: every
        # vertex passes, but the face surfaces at 35.75 m.
        surfacing = [(34.0, 0.0, -1.0), (37.5, 0.0, 1.0), (37.5, 1.0, 1.0)]
        self.assertTrue(all(math.hypot(x, y) >= 36.2 or z < 0.0 for x, y, z in surfacing))
        self.assertTrue(ee._above_water_within(surfacing, 36.2))
        # The same kind of slope surfacing at 37 m is clear.
        self.assertFalse(ee._above_water_within([(34.0, 0.0, -1.0), (40.0, 0.0, 1.0), (40.0, 1.0, 1.0)], 36.2))
        # Wholly sunken; dry across the centre; wet but for one vertex outside.
        self.assertFalse(ee._above_water_within([(0.0, 0.0, -1.0), (5.0, 0.0, -1.0), (0.0, 5.0, -0.5)], 36.2))
        self.assertTrue(ee._above_water_within([(-5.0, -5.0, 0.1), (5.0, -5.0, 0.1), (0.0, 5.0, 0.1)], 36.2))
        self.assertFalse(ee._above_water_within([(37.0, 0.0, 0.0), (40.0, 0.0, -1.0), (40.0, 2.0, -1.0)], 36.2))

    def test_distance_from_the_basin_centre(self):
        self.assertAlmostEqual(ee._distance_to_axis([(3.0, -1.0), (3.0, 1.0), (5.0, 0.0)]), 3.0)
        self.assertEqual(ee._distance_to_axis([(-1.0, -1.0), (1.0, -1.0), (0.0, 1.0)]), 0.0)
        # Collapsed onto a line through the centre, without crossing it.
        self.assertAlmostEqual(ee._distance_to_axis([(2.0, 0.0), (4.0, 0.0)]), 2.0)
        self.assertAlmostEqual(ee._distance_to_axis([(3.0, 4.0)]), 5.0)

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
