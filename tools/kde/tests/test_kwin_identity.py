# SPDX-License-Identifier: GPL-3.0-or-later
"""Which windows are RowPlay's, as the KWin scripts and the harness decide it.

The first version matched `rowplay` anywhere in a window's caption. A browser tab titled
".../shenghaoc/rowplay-qt" (a person reviewing these pull requests) was then a RowPlay window: the identity
stage refused to start, and the raiser, which activates and un-minimizes every window it matches, took the tab
for the app. Windows are RowPlay's only by KWin's own identity properties, compared exactly.
"""

import re
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from kdeacc import kwin  # noqa: E402

REPO = Path(__file__).resolve().parents[3]

# (what it is, KWin's properties for it, is it RowPlay's?)
WINDOWS = [
    ("a browser tab about this repository", dict(desktopFileName="google-chrome", resourceClass="google-chrome", resourceName="chrome",
        caption="tools: add the Plasma acceptance harness foundation by shenghaoc · Pull Request #148 · shenghaoc/rowplay-qt - Google Chrome"), False),
    ("a terminal in the repository", dict(desktopFileName="org.kde.konsole", resourceClass="org.kde.konsole", resourceName="konsole",
        caption="rowplay-qt : claude - Konsole"), False),
    ("another application with rowplay in its name", dict(desktopFileName="org.example.rowplay-viewer", resourceClass="org.example.rowplay-viewer",
        resourceName="rowplay-viewer", caption="Viewer"), False),
    ("an editor with a file named like the app", dict(desktopFileName="org.kde.kate", resourceClass="org.kde.kate", resourceName="kate",
        caption="rowplay.md - Kate"), False),
    ("a window with no identity at all", dict(desktopFileName="", resourceClass="", resourceName="", caption=""), False),
    ("a window that is missing the properties", dict(caption="rowplay"), False),
    ("the branch build, by desktop-entry ID", dict(desktopFileName="io.github.shenghaoc.rowplay", resourceClass="io.github.shenghaoc.rowplay",
        resourceName="rowplay-app", caption="rowplay"), True),
    ("the AppImage, by desktop-entry ID", dict(desktopFileName="io.github.shenghaoc.rowplay", resourceClass="io.github.shenghaoc.rowplay",
        resourceName="rowplay-qt", caption="rowplay"), True),
    ("the baseline build, by executable name", dict(desktopFileName="rowplay-app", resourceClass="rowplay-app", resourceName="rowplay-app",
        caption="rowplay"), True),
    ("an older AppImage, by executable name", dict(desktopFileName="rowplay-qt", resourceClass="rowplay-qt", resourceName="rowplay-qt",
        caption="rowplay"), True),
    ("RowPlay whose caption is something else entirely", dict(desktopFileName="io.github.shenghaoc.rowplay", resourceClass="x", resourceName="y",
        caption="a different title"), True),
]


class Rule(unittest.TestCase):
    def test_every_case(self):
        for what, props, want in WINDOWS:
            self.assertEqual(kwin.is_rowplay_window(props), want, what)

    def test_the_caption_never_decides(self):
        base = dict(desktopFileName="google-chrome", resourceClass="google-chrome", resourceName="chrome")
        for caption in ("rowplay", "RowPlay", "io.github.shenghaoc.rowplay", "rowplay-qt", "rowplay-app"):
            self.assertFalse(kwin.is_rowplay_window({**base, "caption": caption}), caption)

    def test_a_substring_never_decides(self):
        for name in ("rowplay", "RowPlay", "xrowplay-app", "rowplay-app2", "io.github.shenghaoc.rowplay.extra", "org.rowplay-qt"):
            self.assertFalse(kwin.is_rowplay_window({"desktopFileName": name, "resourceClass": name, "resourceName": name}), name)

    def test_the_identity_list_names_the_apps_desktop_entry(self):
        """If the app's ID changes in main.rs, the finder must follow, or it finds nothing."""
        main = (REPO / "crates/rowplay-app/src/main.rs").read_text()
        app_id = re.search(r'const APP_ID: &str = "([^"]+)"', main).group(1)
        self.assertIn(app_id, kwin.ROWPLAY_IDENTITIES)
        self.assertEqual(kwin.IDENTITY_PROPERTIES, ("desktopFileName", "resourceClass", "resourceName"))


class Scripts(unittest.TestCase):
    SCRIPTS = {"list": kwin.LIST_JS, "close": kwin.CLOSE_JS, "raise": kwin.RAISE_JS}

    def rendered(self, name):
        return kwin.render(self.SCRIPTS[name], "MARK")

    def test_all_three_select_windows_by_the_one_shared_predicate(self):
        for name in self.SCRIPTS:
            text = self.rendered(name)
            self.assertEqual(text.count("function isRow("), 1, name)
            self.assertIn("const ROWPLAY_IDS = " + str(list(kwin.ROWPLAY_IDENTITIES)).replace("'", '"'), text, name)
            self.assertNotIn("@IDS@", text)
            self.assertNotIn("@MARK@", text)
            self.assertIn("isRow(w)", text, f"{name} must call the predicate")

    def test_the_predicate_reads_no_caption_and_uses_no_pattern(self):
        for name in self.SCRIPTS:
            text = self.rendered(name)
            body = re.search(r"function isRow\(w\) \{(.*?)\n\}", text, re.S).group(1)
            self.assertNotIn("caption", body, name)
            self.assertNotRegex(body, r"\.test\(|/[a-z]+/i|\.match\(|\.includes\(|indexOf\([^)]*\)\s*<", name)  # exact membership only
            self.assertNotRegex(text, r"/rowplay/i", name)                                                      # the old, loose pattern
            self.assertNotRegex(text, r"konsole|claude", name)                                                  # no list of windows to exempt

    def test_no_script_in_the_module_matches_a_caption_by_pattern(self):
        source = (Path(kwin.__file__)).read_text()
        for number, line in enumerate(source.splitlines(), 1):
            if line.lstrip().startswith("#"):
                continue
            self.assertNotRegex(line, r"/rowplay/i|\.caption\)\.|caption\)\s*;?\s*$.*test", f"kwin.py:{number}: {line.strip()}")
            self.assertNotRegex(line, r"test\([^)]*caption", f"kwin.py:{number}: {line.strip()}")

    def test_only_the_list_script_prints_window_data_and_never_for_other_applications(self):
        text = self.rendered("list")
        # every console.info that carries window properties sits inside the isRow guard
        self.assertRegex(text, r"for \(const w of workspace\.windowList\(\)\) if \(isRow\(w\)\) \{\s*console\.info\(")
        value = r"\bw\.caption\b(?!Changed)"          # the caption's value, not a signal's name
        self.assertEqual(len(re.findall(value, text)), 1)   # printed once, for matched windows only
        self.assertNotRegex(self.rendered("close"), value)
        self.assertNotRegex(self.rendered("raise"), value)
        self.assertNotIn("captionChanged", self.rendered("raise"))   # identity cannot change with a caption


class FinderIgnoresOtherWindows(unittest.TestCase):
    def line(self, marker, props, pid):
        return (f'{marker} caption="{props["caption"]}" resourceClass="{props["resourceClass"]}" resourceName="{props["resourceName"]}" '
                f'desktopFileName="{props["desktopFileName"]}" pid={pid} wayland=true width=100 height=100')

    def test_a_foreign_line_in_the_scripts_output_is_dropped(self):
        """Defence in depth: the script already filters, and the parsed list is filtered again."""
        chrome, row = WINDOWS[0][1], WINDOWS[6][1]
        marker = "ROWPLAY-KWIN-test"
        text = "\n".join([self.line(marker, chrome, 3468), self.line(marker, row, 4242), f"{marker} end"])
        with mock.patch.object(kwin, "run_script", return_value=(marker, text)):
            found = kwin.rowplay_windows(mock.MagicMock())
        self.assertEqual([w["pid"] for w in found], [4242])

    def test_a_browser_alone_is_no_rowplay_window_so_the_identity_precheck_passes(self):
        marker = "ROWPLAY-KWIN-test"
        text = "\n".join([self.line(marker, WINDOWS[0][1], 3468), f"{marker} end"])
        with mock.patch.object(kwin, "run_script", return_value=(marker, text)):
            self.assertEqual(kwin.rowplay_windows(mock.MagicMock()), [])

    def test_kwin_that_cannot_be_asked_is_none_not_an_empty_list(self):
        with mock.patch.object(kwin, "run_script", return_value=("M", "")):
            self.assertIsNone(kwin.rowplay_windows(mock.MagicMock()))


class IdentityPrecheck(unittest.TestCase):
    """The identity stage refuses to start with a RowPlay window already open, and says which problem it met."""

    def run_stage(self, windows):
        from unittest import mock as m
        from kdeacc import stages
        import tempfile

        class Args:
            allow_session_changes = True
            appimage = None

        with tempfile.TemporaryDirectory() as d:
            ctx = stages.Ctx(REPO, REPO, Path("/x"), Path(d), m.MagicMock(), Args())
            ctx.appimages["branch"] = Path(d) / "a.AppImage"
            (Path(d) / "a.AppImage").write_text("x")
            with m.patch.object(stages.kwin, "rowplay_windows", return_value=windows):
                return stages.stage_identity(ctx)

    def test_no_window_goes_on(self):
        from unittest import mock as m
        from kdeacc import stages
        with m.patch.object(stages, "key_injection_survey", side_effect=RuntimeError("past the precheck")):
            with self.assertRaises(RuntimeError):
                self.run_stage([])

    def test_a_real_window_is_named(self):
        st = self.run_stage([dict(pid=4242, desktopFileName="io.github.shenghaoc.rowplay", resourceClass="x")])
        self.assertEqual([c.status.value for c in st.checks], ["FAIL"])
        self.assertIn("close RowPlay first", st.checks[0].detail)
        self.assertIn("pid 4242", st.checks[0].detail)
        self.assertIn("io.github.shenghaoc.rowplay", st.checks[0].detail)

    def test_kwin_that_cannot_be_asked_is_said_so_and_not_blamed_on_a_window(self):
        st = self.run_stage(None)
        self.assertEqual([c.status.value for c in st.checks], ["FAIL"])
        self.assertIn("KWin could not be asked", st.checks[0].detail)
        self.assertNotIn("close RowPlay", st.checks[0].detail)


if __name__ == "__main__":
    unittest.main()
