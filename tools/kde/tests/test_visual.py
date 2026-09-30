# SPDX-License-Identifier: GPL-3.0-or-later
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from kdeacc import visual  # noqa: E402

CD = visual.load_capture_diff(HERE.parents[2])  # the repository's real capture-diff.py

W, H = 200, 120


def canvas(colour=(240, 240, 240)):
    return bytearray(bytes(colour) * (W * H))


def paint(buf, rect, colour):
    x0, y0, x1, y1 = rect
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            i = (y * W + x) * 3
            buf[i:i + 3] = bytes(colour)


def ring(buf, rect, t, colour):
    x0, y0, x1, y1 = rect
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            if x - x0 < t or x1 - x < t or y - y0 < t or y1 - y < t:
                i = (y * W + x) * 3
                buf[i:i + 3] = bytes(colour)


def cap(buf):
    return (W, H, bytes(buf))


class RegionRules(unittest.TestCase):
    def test_change_inside_region_passes(self):
        a, b = canvas(), canvas()
        paint(b, (50, 50, 80, 53), (61, 174, 233))
        rule = {"regions": [[48, 48, 82, 55]], "must_change": True}
        r = visual.classify("phase-row-catch", cap(a), cap(b), rule, CD)
        self.assertEqual((r.status, r.kind, r.outside, r.inside), ("PASS", "expected-change", 0, 31 * 4))

    def test_one_stray_pixel_outside_fails(self):
        a, b = canvas(), canvas()
        paint(b, (50, 50, 80, 53), (61, 174, 233))
        paint(b, (150, 100, 150, 100), (0, 0, 0))
        rule = {"regions": [[48, 48, 82, 55]], "must_change": True}
        r = visual.classify("phase-row-catch", cap(a), cap(b), rule, CD)
        self.assertEqual((r.status, r.outside), ("FAIL", 1))
        self.assertIn("outside the allowed area", r.detail)
        self.assertIn("[150, 100, 150, 100]", r.detail)

    def test_missing_expected_change_fails(self):
        a = canvas()
        rule = {"regions": [[48, 48, 82, 55]], "must_change": True}
        r = visual.classify("phase-row-catch", cap(a), cap(canvas()), rule, CD)
        self.assertEqual(r.status, "FAIL")
        self.assertIn("expected change is missing", r.detail)

    def test_must_change_false_allows_no_change(self):
        rule = {"regions": [[0, 0, 5, 5]], "must_change": False}
        r = visual.classify("dashboard", cap(canvas()), cap(canvas()), rule, CD)
        self.assertEqual(r.status, "PASS")

    def test_a_changes_own_edge_pixels_do_not_count_as_noise(self):
        # 20x20 with a soft 3-delta halo inside the region: 400+ quiet px, all allowed.
        a, b = canvas(), canvas()
        paint(b, (50, 50, 90, 90), (243, 243, 243))
        paint(b, (60, 60, 62, 62), (0, 0, 0))
        rule = {"regions": [[48, 48, 92, 92]], "must_change": True}
        self.assertEqual(visual.classify("dashboard", cap(a), cap(b), rule, CD).status, "PASS")
        # The same halo outside the region exceeds the 2D noise bound (400 px).
        rule2 = {"regions": [[59, 59, 63, 63]], "must_change": True}
        r = visual.classify("dashboard", cap(a), cap(b), rule2, CD)
        self.assertEqual(r.status, "FAIL")
        self.assertIn("sub-threshold", r.detail)

    def test_size_mismatch_and_missing_side_fail(self):
        small = (10, 10, bytes(300))
        self.assertEqual(visual.classify("x", cap(canvas()), small, None, CD).status, "FAIL")
        self.assertEqual(visual.classify("x", None, cap(canvas()), None, CD).status, "FAIL")


class BandRules(unittest.TestCase):
    RULE = {"bands": [{"outer": [10, 10, 150, 100], "thickness": 5}], "must_change": True}

    def test_ring_passes(self):
        b = canvas()
        ring(b, (10, 10, 150, 100), 4, (54, 139, 183))
        r = visual.classify("detail-nostrokes", cap(canvas()), cap(b), self.RULE, CD)
        self.assertEqual((r.status, r.outside), ("PASS", 0))

    def test_interior_change_fails(self):
        b = canvas()
        ring(b, (10, 10, 150, 100), 4, (54, 139, 183))
        paint(b, (70, 50, 72, 52), (0, 0, 0))  # the ring's interior
        r = visual.classify("detail-nostrokes", cap(canvas()), cap(b), self.RULE, CD)
        self.assertEqual((r.status, r.outside), ("FAIL", 9))

    def test_filled_rectangle_is_not_a_ring(self):
        b = canvas()
        paint(b, (10, 10, 150, 100), (54, 139, 183))
        r = visual.classify("detail-nostrokes", cap(canvas()), cap(b), self.RULE, CD)
        self.assertEqual(r.status, "FAIL")
        self.assertGreater(r.outside, 10000)


class NoiseRules(unittest.TestCase):
    def test_unlisted_noise_within_the_bound_passes(self):
        b = canvas()
        for x in range(0, 150, 10):  # 15 scattered px, delta 3
            paint(b, (x, 5, x, 5), (243, 243, 243))
        r = visual.classify("dashboard", cap(canvas()), cap(b), None, CD)
        self.assertEqual((r.status, r.kind, r.changed), ("PASS", "noise", 0))

    def test_unlisted_real_change_fails(self):
        b = canvas()
        paint(b, (20, 20, 60, 40), (0, 0, 0))
        r = visual.classify("dashboard", cap(canvas()), cap(b), None, CD)
        self.assertEqual((r.status, r.kind), ("FAIL", "unlisted-change"))
        self.assertGreater(r.outside, 400)

    def test_3d_captures_have_the_tighter_bound(self):
        b = canvas()
        paint(b, (0, 0, 20, 20), (244, 244, 244))  # 441 px, delta 4
        self.assertEqual(visual.classify("dashboard", cap(canvas()), cap(b), None, CD).status, "FAIL")  # 2D bound 400
        b2 = canvas()
        paint(b2, (0, 0, 15, 15), (245, 245, 245))  # 256 px, delta 5 > 3D delta bound 4
        self.assertEqual(visual.classify("phase-row-catch", cap(canvas()), cap(b2), None, CD).status, "FAIL")


class NativeNoiseProfile(unittest.TestCase):
    PROFILE = {"max_delta": 1, "max_pixels_2d": 700, "max_pixels_3d": 250}

    def dithered(self, n):
        b = canvas()
        for k in range(n):
            paint(b, (k % W, k // W, k % W, k // W), (241, 241, 241))  # delta 1
        return b

    def test_delta_one_dithering_is_noise_only_under_the_profile(self):
        b = self.dithered(600)  # over the 2D bound (400), under the profile's 700
        self.assertEqual(visual.classify("dashboard", cap(canvas()), cap(b), None, CD).status, "FAIL")
        self.assertEqual(visual.classify("dashboard", cap(canvas()), cap(b), None, CD, self.PROFILE).status, "PASS")

    def test_the_profile_does_not_excuse_a_bigger_delta_or_more_pixels(self):
        many = self.dithered(800)
        self.assertEqual(visual.classify("dashboard", cap(canvas()), cap(many), None, CD, self.PROFILE).status, "FAIL")
        b = self.dithered(20)
        paint(b, (100, 100, 100, 100), (245, 245, 245))  # delta 5: over 1
        self.assertEqual(visual.classify("dashboard", cap(canvas()), cap(b), None, CD, self.PROFILE).status, "PASS")  # 2D delta bound is 8
        b2 = self.dithered(20)
        paint(b2, (100, 100, 100, 100), (245, 245, 245))
        self.assertEqual(visual.classify("phase-row-catch", cap(canvas()), cap(b2), None, CD, self.PROFILE).status, "FAIL")  # 3D bound is 4

    def test_a_listed_capture_may_carry_the_dithering_outside_its_region(self):
        b = self.dithered(600)
        paint(b, (150, 100, 190, 110), (61, 174, 233))
        rule = {"regions": [[148, 98, 192, 112]], "must_change": True}
        self.assertEqual(visual.classify("dashboard", cap(canvas()), cap(b), rule, CD).status, "FAIL")
        self.assertEqual(visual.classify("dashboard", cap(canvas()), cap(b), rule, CD, self.PROFILE).status, "PASS")


class Derivation(unittest.TestCase):
    def test_a_bar_becomes_a_padded_region_and_passes_its_own_rule(self):
        a, b = canvas(), canvas()
        paint(b, (50, 60, 120, 63), (61, 174, 233))
        rule = visual.derive_rule("phase-row-catch", cap(a), cap(b), CD)
        self.assertEqual(rule["regions"], [[48, 58, 122, 65]])
        self.assertNotIn("bands", rule)
        self.assertEqual(visual.classify("phase-row-catch", cap(a), cap(b), rule, CD).status, "PASS")

    def test_a_ring_becomes_a_band_and_interior_then_fails(self):
        a, b = canvas(), canvas()
        ring(b, (10, 10, 150, 100), 4, (54, 139, 183))
        rule = visual.derive_rule("detail-nostrokes", cap(a), cap(b), CD)
        self.assertEqual(rule["bands"], [{"outer": [10, 10, 150, 100], "thickness": 5}])
        self.assertEqual(rule["regions"], [])
        self.assertEqual(visual.classify("detail-nostrokes", cap(a), cap(b), rule, CD).status, "PASS")
        paint(b, (70, 50, 70, 50), (0, 0, 0))
        self.assertEqual(visual.classify("detail-nostrokes", cap(a), cap(b), rule, CD).status, "FAIL")

    def test_interior_outliers_do_not_widen_a_ring_and_stay_exact(self):
        # A large ring, 98 %+ of whose changed pixels lie on its border band, plus a few pixels deeper inside. They must
        # touch the ring's 16 px cell grid to be part of its cluster at all, so they sit 8 px in from the left edge.
        a, b = canvas(), canvas()
        outer = (5, 5, 194, 114)
        ring(b, outer, 4, (54, 139, 183))
        paint(b, (13, 60, 14, 61), (0, 0, 0))                    # 4 pixels, dist 8: past RING_BAND, well under 2 % of the ring
        rule = visual.derive_rule("detail-nostrokes", cap(a), cap(b), CD)
        (band,) = rule["bands"]
        self.assertEqual(band["outer"], list(outer))
        self.assertEqual(band["thickness"], 5, "the border pixels make the band 4 + 1 = 5 thick; an interior outlier must not widen it")
        self.assertEqual(rule["regions"], [[11, 58, 16, 63]], "the outliers are a padded region of their own")
        self.assertEqual(visual.classify("detail-nostrokes", cap(a), cap(b), rule, CD).status, "PASS")
        # ... and the interior is still not allowed anywhere else: a pixel near the centre, and one just inside the band
        # (with the old, widened band this second one would have been allowed)
        stray = bytearray(b)
        paint(stray, (100, 60, 100, 60), (0, 0, 0))
        result = visual.classify("detail-nostrokes", cap(a), cap(stray), rule, CD)
        self.assertEqual((result.status, result.outside), ("FAIL", 1))
        near = bytearray(b)
        paint(near, (20, 30, 20, 30), (0, 0, 0))     # 15 px in from the left edge, past the band's 5 px and outside the outlier region
        result = visual.classify("detail-nostrokes", cap(a), cap(near), rule, CD)
        self.assertEqual((result.status, result.outside), ("FAIL", 1))
        # the ring is not a filled rectangle: the band's own interior is still outside it
        self.assertLess(band["thickness"] * 2, min(outer[2] - outer[0], outer[3] - outer[1]))

    def test_a_ring_with_no_outliers_derives_as_before(self):
        a, b = canvas(), canvas()
        ring(b, (5, 5, 194, 114), 4, (54, 139, 183))
        rule = visual.derive_rule("detail-nostrokes", cap(a), cap(b), CD)
        self.assertEqual((rule["bands"], rule["regions"]), ([{"outer": [5, 5, 194, 114], "thickness": 5}], []))

    def test_two_far_apart_changes_are_two_regions_and_a_noise_only_pair_has_no_rule(self):
        a, b = canvas(), canvas()
        paint(b, (5, 5, 20, 20), (0, 0, 0))
        paint(b, (150, 90, 170, 110), (0, 0, 0))
        self.assertEqual(len(visual.derive_rule("dashboard", cap(a), cap(b), CD)["regions"]), 2)
        c = canvas()
        paint(c, (5, 5, 6, 6), (243, 243, 243))
        self.assertIsNone(visual.derive_rule("dashboard", cap(a), cap(c), CD))


class Directories(unittest.TestCase):
    def write_ppm(self, path, buf):
        path.write_bytes(b"P6\n%d %d\n255\n" % (W, H) + bytes(buf))

    def test_compare_dirs_end_to_end_with_a_stale_rule(self):
        import tempfile
        with tempfile.TemporaryDirectory() as d:
            d = Path(d)
            (d / "a").mkdir()
            (d / "b").mkdir()
            base, changed = canvas(), canvas()
            paint(changed, (50, 60, 120, 63), (61, 174, 233))
            for name in ("phase-row-catch", "dashboard"):
                self.write_ppm(d / "a" / f"{name}.ppm", base)
            self.write_ppm(d / "b" / "phase-row-catch.ppm", changed)
            self.write_ppm(d / "b" / "dashboard.ppm", base)
            self.write_ppm(d / "a" / "smoke.ppm", base)
            self.write_ppm(d / "b" / "smoke.ppm", changed)  # animated: skipped
            rules = {"captures": {"phase-row-catch": {"regions": [[48, 58, 122, 65]], "must_change": True},
                                  "gone": {"regions": [[0, 0, 1, 1]]}}}
            results = visual.compare_dirs(d / "a", d / "b", rules, CD)
            by = {r.name: r for r in results}
            self.assertEqual(by["phase-row-catch"].status, "PASS")
            self.assertEqual(by["dashboard"].status, "PASS")
            self.assertEqual(by["gone"].status, "FAIL")
            self.assertNotIn("smoke", by)
            summary = visual.summarize(results)
            self.assertEqual((summary["expected_changes"], summary["failed"]), (1, ["gone"]))


if __name__ == "__main__":
    unittest.main()
