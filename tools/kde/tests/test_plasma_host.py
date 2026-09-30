# SPDX-License-Identifier: GPL-3.0-or-later
import os
import signal
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from kdeacc import host, qtprobe  # noqa: E402


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
        full = {"fedora": {"version_id": "44"}, "session": {"type": "wayland", "desktop": "KDE", "xdg_current_desktop": "KDE"}}
        for k in ("kernel", "plasma", "kwin", "kde_frameworks", "host_qt", "bundled_qt", "mesa", "gpu_renderer",
                  "xdg_desktop_portal", "xdg_desktop_portal_kde"):
            full[k] = "x"
        self.assertEqual(host.host_gaps(full), [])
        full["plasma"] = None
        full["session"]["type"] = "x11"
        self.assertEqual(host.host_gaps(full), ["plasma", "session is not Wayland"])


class PlasmaSessionGate(unittest.TestCase):
    """Plasma's packages can be installed under GNOME or Sway: only a KDE session is Plasma evidence."""

    def host(self, desktop, xdg, type_="wayland"):
        h = {"fedora": {"version_id": "44"}, "session": {"type": type_, "desktop": desktop, "xdg_current_desktop": xdg}}
        for k in ("kernel", "plasma", "kwin", "kde_frameworks", "host_qt", "bundled_qt", "mesa", "gpu_renderer",
                  "xdg_desktop_portal", "xdg_desktop_portal_kde"):
            h[k] = "x"
        return h

    def test_kde_sessions_are_plasma(self):
        self.assertEqual(host.host_gaps(self.host("KDE", "KDE")), [])
        self.assertEqual(host.host_gaps(self.host(None, "KDE")), [])
        self.assertEqual(host.host_gaps(self.host("", "plasma:KDE")), [])

    def test_gnome_sway_and_unknown_sessions_are_not(self):
        for desktop, xdg in (("gnome", "GNOME"), ("sway", "sway"), (None, None), ("", "ubuntu:GNOME")):
            gaps = host.host_gaps(self.host(desktop, xdg))
            self.assertEqual(len(gaps), 1, (desktop, xdg, gaps))
            self.assertIn("not KDE Plasma", gaps[0])

    def test_an_x11_plasma_session_is_still_not_wayland(self):
        self.assertEqual(host.host_gaps(self.host("KDE", "KDE", type_="x11")), ["session is not Wayland"])


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
