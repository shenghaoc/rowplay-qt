# SPDX-License-Identifier: GPL-3.0-or-later
"""The Phase 3 water field: tiling, slope, spectrum and no dominant lattice."""

import math
import unittest

import numpy as np

import water


class WaterFieldTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.comps = water.components()
        cls.normals, cls.record = water.normals()

    def test_regeneration_is_identical(self):
        again, _ = water.normals()
        self.assertTrue(np.array_equal(self.normals, again))

    def test_integer_wave_vectors_tile(self):
        # Integer wave vectors make the field periodic on the tile, so the
        # texture has no seam: the slope past the last column is the first.
        terms = water.gusts()
        n = water.WATER["size"]
        v = np.arange(n) / n * 2 * math.pi
        first = water.slopes(np.zeros(n), v, self.comps, terms)
        wrapped = water.slopes(np.full(n, 2 * math.pi), v, self.comps, terms)
        for a, b in zip(first, wrapped):
            np.testing.assert_allclose(a, b, atol=1e-9)
        self.assertTrue(all(isinstance(m, int) and isinstance(q, int) for m, q in self.comps))

    def test_slope_is_scaled_to_its_rms(self):
        n = self.normals
        sx, sy = -n[..., 0] / n[..., 2], -n[..., 1] / n[..., 2]
        self.assertAlmostEqual(math.sqrt((sx * sx + sy * sy).mean()), water.WATER["rmsSlope"], places=9)
        self.assertEqual(water.WATER["rmsSlope"], 0.11)

    def test_spectrum_spans_chop_ripples_and_swell(self):
        tile = water.WATER["tile"]
        lams = sorted(tile / math.hypot(m, q) for m, q in self.comps)
        self.assertEqual(len(lams), 360)
        self.assertLess(lams[0], 0.08)
        self.assertGreater(lams[-1], 2.0)
        # Every octave from 7.5 cm to 2.4 m holds several components.
        for low in (0.075, 0.15, 0.3, 0.6, 1.2):
            self.assertGreaterEqual(sum(low <= l < 2 * low for l in lams), 8, low)

    def test_no_component_dominates(self):
        # The Phase 1 map had 16 components, so each carried 6 % or more of
        # the slope variance and their beats read as a lattice.
        variance = [slope * slope for slope, _ in self.comps.values()]
        self.assertLess(max(variance) / sum(variance), 0.02)

    def test_slope_is_not_one_directional(self):
        # The Phase 1 map's RMS slope was 0.070 along the tile and 0.029
        # across it (2.4x): parallel bands. The wind still leans the field.
        n = self.normals
        sx, sy = -n[..., 0] / n[..., 2], -n[..., 1] / n[..., 2]
        ratio = math.sqrt((sx * sx).mean() / (sy * sy).mean())
        self.assertLess(max(ratio, 1 / ratio), 1.5)


if __name__ == "__main__":
    unittest.main()
