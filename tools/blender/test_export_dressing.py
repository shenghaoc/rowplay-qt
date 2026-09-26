# SPDX-License-Identifier: GPL-3.0-or-later
"""The dressing export's pure helpers, without Blender."""

import json
import unittest

import export_dressing as ed


class ExportDressingTest(unittest.TestCase):
    def test_a_compact_structure_gets_its_own_sector(self):
        # A box at 52 degrees, 38 m out: Blender points (x, y, z), z up.
        points = [(38.0 * 0.788 + dx, -38.0 * 0.616 + dy, z) for dx in (-1.5, 1.5) for dy in (-1.5, 1.5) for z in (0.0, 6.0)]
        r0, r1, a0, a1 = ed._sector(points)
        self.assertTrue(35.5 < r0 < 38.0 < r1 < 40.5)
        self.assertTrue(a0 < 52.0 < a1 and a1 - a0 < 8.0)

    def test_a_sector_across_zero_degrees_keeps_its_span(self):
        import math
        points = [(40.0 * math.sin(math.radians(a)), -40.0 * math.cos(math.radians(a)), 0.0) for a in (350.0, 355.0, 0.0, 5.0, 10.0)]
        r0, r1, a0, a1 = ed._sector(points)
        self.assertAlmostEqual(a0, 350.0, places=3)
        self.assertAlmostEqual(a1, 370.0, places=3)
        self.assertTrue(ed.inside_sector((r0, r1, a0, a1), 40.0 * math.sin(math.radians(2.0)), 40.0 * math.cos(math.radians(2.0)), 0.3))
        self.assertFalse(ed.inside_sector((r0, r1, a0, a1), 40.0 * math.sin(math.radians(20.0)), 40.0 * math.cos(math.radians(20.0)), 0.3))

    def test_the_island_is_a_ring(self):
        import math
        points = [(r * math.cos(a), r * math.sin(a), 0.0) for r in (5.0, 15.0) for a in (0.0, 1.5, 3.0, 4.5)]
        self.assertEqual(ed._sector(points)[2:], [0.0, 360.0])

    def test_the_lane_band_holds_the_buoy_rings(self):
        self.assertTrue(ed.in_lane_band(23.1))
        self.assertTrue(ed.in_lane_band(33.3))
        self.assertFalse(ed.in_lane_band(22.25, 0.365))
        self.assertTrue(ed.in_lane_band(22.7, 0.365))
        self.assertFalse(ed.in_lane_band(34.2, 0.365))

    def test_primitives_are_matched_to_slots_by_their_triangles(self):
        a = [((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)), ((5.0, 0.0, 0.0), (6.0, 0.0, 0.0), (5.0, 1.0, 0.0))]
        b = [((0.0, 0.0, 2.0), (1.0, 0.0, 2.0), (0.0, 1.0, 2.0))]
        slots = [("paint", ed._centroids(a)), ("glass", ed._centroids(b))]
        # Canonicalisation reorders triangles and rotates their corners: the signature holds.
        rotated = [(a[1][1], a[1][2], a[1][0]), a[0]]
        self.assertEqual(ed.match_classes(slots, [ed._centroids(b), ed._centroids(rotated)]), ["glass", "paint"])
        with self.assertRaises(ed.ContractError):
            ed.match_classes(slots, [ed._centroids(b)])
        with self.assertRaises(ed.ContractError):
            ed.match_classes(slots, [ed._centroids(b), ed._centroids(b)])
        with self.assertRaises(ed.ContractError):
            ed.match_classes([("paint", ed._centroids(a)), ("timber", ed._centroids(a))], [ed._centroids(a), ed._centroids(a)])

    def test_one_instance_per_line(self):
        plan = {"triangles": {"dressing:row:finish-tower": 2000, "dressing:row:bollard": 88},
                "footprints": {"finish-tower": [36.9, 40.1, 49.0, 55.0]},
                "instances": [{"variant": "bollard", "tier": 1, "name": "bollard.000.1", "position": [1.0, 0.18, 36.5],
                               "yaw": 12.5, "scale": 1.0, "color": "#ffffff"}]}
        structures, variants = ed.STRUCTURES, ed.VARIANTS
        try:
            ed.STRUCTURES = {"finish-tower": ("land", True, True)}
            ed.VARIANTS = ("bollard",)
            text = ed.dressing_json(plan, {"dressing:row:finish-tower": ["paint", "metal"], "dressing:row:bollard": ["metal"]})
        finally:
            ed.STRUCTURES, ed.VARIANTS = structures, variants
        parsed = json.loads(text)
        self.assertEqual(parsed["structures"][0]["classes"], ["paint", "metal"])
        self.assertEqual(parsed["structures"][0]["footprint"], [36.9, 40.1, 49.0, 55.0])
        self.assertEqual(parsed["variants"][0]["name"], "bollard")
        self.assertNotIn("name", parsed["instances"][0])
        lines = text.splitlines()
        self.assertEqual(sum(1 for line in lines if line.startswith('    {"variant"')), 1)
        self.assertEqual(sum(1 for line in lines if line.startswith('    {"name"')), 2)


if __name__ == "__main__":
    unittest.main()
