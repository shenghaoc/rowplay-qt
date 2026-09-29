# SPDX-License-Identifier: GPL-3.0-or-later
import os
import signal
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from kdeacc import host, plasma, qtprobe  # noqa: E402


class HostParsing(unittest.TestCase):
    def test_rpm_versions(self):
        self.assertEqual(host.parse_rpm_version("plasma-workspace-6.7.5-1.fc44.x86_64"), "6.7.5")
        self.assertEqual(host.parse_rpm_version("kf6-kcoreaddons-6.30.0-1.fc44.x86_64"), "6.30.0")
        self.assertEqual(host.parse_rpm_version("mesa-dri-drivers-26.2.3-1.fc44.x86_64"), "26.2.3")
        self.assertIsNone(host.parse_rpm_version("package ksecretd is not installed"))

    def test_tool_versions_and_os_release(self):
        self.assertEqual(host.parse_tool_version("plasmashell 6.7.5\n"), "6.7.5")
        self.assertEqual(host.parse_tool_version("kwin 6.7.5"), "6.7.5")
        osr = host.parse_os_release('NAME="Fedora Linux"\nVERSION_ID=44\nPRETTY_NAME="Fedora Linux 44 (KDE Plasma Desktop Edition)"\n')
        self.assertEqual((osr["VERSION_ID"], osr["PRETTY_NAME"]), ("44", "Fedora Linux 44 (KDE Plasma Desktop Edition)"))

    def test_glxinfo_and_session(self):
        text = "    Accelerated: yes\nOpenGL renderer string: Mesa Intel(R) UHD Graphics 630 (CFL GT2)\nOpenGL version string: 4.6 (Compatibility Profile) Mesa 26.2.3\n"
        g = host.parse_glxinfo(text)
        self.assertEqual((g["renderer"], g["accelerated"]), ("Mesa Intel(R) UHD Graphics 630 (CFL GT2)", "yes"))
        self.assertEqual(host.parse_session("Type=wayland\nDesktop=KDE\n"), {"Type": "wayland", "Desktop": "KDE"})

    def test_gaps_are_named(self):
        full = {"fedora": {"version_id": "44"}, "session": {"type": "wayland"}}
        for k in ("kernel", "plasma", "kwin", "kde_frameworks", "host_qt", "bundled_qt", "mesa", "gpu_renderer",
                  "xdg_desktop_portal", "xdg_desktop_portal_kde"):
            full[k] = "x"
        self.assertEqual(host.host_gaps(full), [])
        full["plasma"] = None
        full["session"]["type"] = "x11"
        self.assertEqual(host.host_gaps(full), ["plasma", "session is not Wayland"])


class PlasmaParsing(unittest.TestCase):
    def test_current_scheme(self):
        text = "You have the following color schemes on your system:\n * BreezeClassic\n * BreezeDark\n * BreezeLight (current color scheme)\n"
        self.assertEqual(plasma.parse_current_scheme(text), "BreezeLight")
        self.assertIsNone(plasma.parse_current_scheme(" * BreezeDark\n"))

    def test_portal_appearance(self):
        text = ("({'org.freedesktop.appearance': {'contrast': <uint32 0>, 'color-scheme': <uint32 2>, "
                "'reduced-motion': <uint32 0>, 'accent-color': <(0.23921568691730499, 0.68235296010971069, 0.91372549533843994)>}},)")
        self.assertEqual(plasma.parse_portal_appearance(text),
                         {"contrast": 0, "color-scheme": 2, "accent-color": (0.2392, 0.6824, 0.9137)})

    def test_font_scaling(self):
        self.assertEqual(plasma.scale_font("Noto Sans,10,-1,5,50,0,0,0,0,0", 1.5), "Noto Sans,15,-1,5,50,0,0,0,0,0")
        self.assertEqual(plasma.scale_font("Noto Sans,11,-1,5,400,0,0,0,0,0,0,0,0,0,0,1", 1.5).split(",")[1], "16")  # 16.5 -> 16 (round half even)
        self.assertEqual(plasma.font_point_size("Noto Sans,10,-1,5,50"), 10.0)
        with self.assertRaises(ValueError):
            plasma.scale_font("nonsense", 1.5)

    def test_the_loudest_accent_is_far_from_the_current_one(self):
        self.assertNotEqual(plasma.loudest_accent("#f67400"), "#f67400")
        self.assertEqual(plasma.loudest_accent("#3daee9", "#eff0f1"), "#f67400")  # orange is farthest from blue and pale grey

    def test_snapshot_comparison_names_every_difference(self):
        def snap(sha="a", scheme="BreezeLight", **qt):
            return plasma.Snapshot(b"x", sha, scheme, {"color-scheme": 2}, "Noto Sans,10", {"highlight": "#3daee9", **qt})
        self.assertEqual(plasma.compare_snapshots(snap(), snap()), [])
        diffs = plasma.compare_snapshots(snap(), snap(sha="b", scheme="BreezeDark", highlight="#f89d4c"))
        self.assertEqual(len(diffs), 3)
        self.assertTrue(any("kdeglobals differs" in d for d in diffs))
        self.assertTrue(any("colour scheme BreezeLight -> BreezeDark" in d for d in diffs))
        self.assertTrue(any("Qt highlight" in d for d in diffs))


class FakePlasma(plasma.Plasma):
    """Plasma with no desktop: kdeglobals is a temp file, the 'tools' are Python."""

    def __init__(self, path, fail_restore=False):
        super().__init__(runner=None, probe_fn=lambda: {"highlight": "#3daee9"}, kdeglobals=path, settle=0)
        self.scheme = "BreezeLight"
        self.applied = []
        self.fail_restore = fail_restore
        self.font = "Noto Sans,10"

    def read_scheme(self): return self.scheme
    def read_portal(self): return {"color-scheme": 2}
    def read_font(self): return self.font

    def apply_scheme(self, name, accent=None):
        self.scheme = name
        self.applied.append((name, accent))
        # the real tool leaves an explicit key behind: not what the file held before
        with open(self.kdeglobals, "a") as h:
            h.write("ColorScheme=%s\n" % name)

    def restore(self, snap):
        if self.fail_restore:
            return ["kdeglobals differs (simulated)"]
        return super().restore(snap)


class Transaction(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.file = Path(self.dir.name) / "kdeglobals"
        self.file.write_text("[General]\nColorSchemeHash=abc\n")
        self.original = self.file.read_bytes()

    def tearDown(self):
        self.dir.cleanup()

    def test_refuses_without_permission(self):
        with self.assertRaises(PermissionError):
            with plasma.SessionTransaction(FakePlasma(self.file), allow=False):
                pass
        self.assertEqual(self.file.read_bytes(), self.original)

    def test_restores_exact_bytes_after_success(self):
        p = FakePlasma(self.file)
        with plasma.SessionTransaction(p, allow=True) as tx:
            p.apply_scheme("BreezeDark", "#f67400")
            self.assertNotEqual(self.file.read_bytes(), self.original)
        self.assertTrue(tx.restored, tx.left)
        self.assertEqual(self.file.read_bytes(), self.original)
        self.assertEqual(p.scheme, "BreezeLight")

    def test_restores_after_an_exception_and_still_raises_it(self):
        p = FakePlasma(self.file)
        tx = plasma.SessionTransaction(p, allow=True)
        with self.assertRaises(RuntimeError):
            with tx:
                p.apply_scheme("BreezeDark")
                raise RuntimeError("a test blew up")
        self.assertTrue(tx.restored)
        self.assertEqual(self.file.read_bytes(), self.original)

    def test_restores_on_sigterm_and_sigint(self):
        for sig in (signal.SIGTERM, signal.SIGINT):
            p = FakePlasma(self.file)
            tx = plasma.SessionTransaction(p, allow=True)
            with self.assertRaises(KeyboardInterrupt):
                with tx:
                    p.apply_scheme("BreezeDark")
                    os.kill(os.getpid(), sig)
                    self.fail("the signal must interrupt the block")
            self.assertTrue(tx.restored, (sig, tx.left))
            self.assertEqual(self.file.read_bytes(), self.original)

    def test_a_failed_restoration_is_reported_not_swallowed(self):
        p = FakePlasma(self.file, fail_restore=True)
        with plasma.SessionTransaction(p, allow=True) as tx:
            pass
        self.assertFalse(tx.restored)
        self.assertEqual(tx.left, ["kdeglobals differs (simulated)"])

    def test_signal_handlers_are_put_back(self):
        before = signal.getsignal(signal.SIGTERM)
        with plasma.SessionTransaction(FakePlasma(self.file), allow=True):
            self.assertNotEqual(signal.getsignal(signal.SIGTERM), before)
        self.assertEqual(signal.getsignal(signal.SIGTERM), before)


class ProbeParsing(unittest.TestCase):
    BASE = {"accent": "#000000", "highlight": "#3daee9", "themeAccentColor": "#3daee9",
            "themeSystemAccentAvailable": True, "colorScheme": 1, "contrast": 0,
            "buttonBackground": "ButtonPanel_QMLTYPE_4(0x1)"}

    def out(self, **over):
        import json
        info = {**self.BASE, **over}
        return ('qt.qpa.theme: Adding platform integration\'s theme names to list of theme names: QList("kde", "generic")\n'
                'qt.qpa.theme: Successfully created platform theme "kde" via createPlatformTheme\n'
                'qt.gui.icon.loader: Initialized icon loader with system theme "breeze" and SVG support true\n'
                "qml: PROBE " + json.dumps(info) + "\n")

    def test_theme_and_icon_facts_come_from_qts_own_log(self):
        info = qtprobe.parse_probe_output(self.out())
        self.assertEqual((info["platformThemeCreated"], info["platformThemeNames"], info["iconTheme"]),
                         ("kde", ["kde", "generic"], "breeze"))
        self.assertEqual((info["colorSchemeName"], info["contrastName"], info["fusion"]), ("Light", "NoPreference", True))

    def test_no_probe_line_is_an_error(self):
        with self.assertRaises(ValueError):
            qtprobe.parse_probe_output("nothing useful")

    def test_accent_condition_branches_and_names_no_desktop(self):
        ok, story = qtprobe.accent_condition(qtprobe.parse_probe_output(self.out()))
        self.assertTrue(ok)
        self.assertIn("unset default", story)
        # Qt reports a usable Accent: Theme must use it as is.
        good = qtprobe.parse_probe_output(self.out(accent="#12ab34", themeAccentColor="#12ab34"))
        self.assertTrue(qtprobe.accent_condition(good)[0])
        # ... and a Theme that ignored it is a failure.
        bad = qtprobe.parse_probe_output(self.out(accent="#12ab34", themeAccentColor="#3daee9"))
        self.assertFalse(qtprobe.accent_condition(bad)[0])
        # Nothing usable anywhere: no platform accent, brand blue.
        none = qtprobe.parse_probe_output(self.out(accent="#308cc6", highlight="#308cc6", themeAccentColor="#0066cc", themeSystemAccentAvailable=False))
        self.assertTrue(qtprobe.accent_condition(none)[0])
        self.assertNotIn("KDE", story)
        self.assertNotIn("Plasma", story)


if __name__ == "__main__":
    unittest.main()
