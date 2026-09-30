# SPDX-License-Identifier: GPL-3.0-or-later
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.dont_write_bytecode = True

import acceptance  # noqa: E402
from kdeacc import gates, kwin, package, services, stages  # noqa: E402
from kdeacc.results import Run, Stage, Status, render_summary, write_manifest  # noqa: E402

REPO = Path(__file__).resolve().parents[3]


class Aggregation(unittest.TestCase):
    def test_any_failed_check_fails_the_run_and_the_exit_status(self):
        run = Run()
        run.stage("a").passed("fine")
        self.assertEqual((run.failed, run.exit_code()), (False, 0))
        run.stage("b").failed("broken", "why")
        self.assertEqual((run.failed, run.exit_code()), (True, 1))

    def test_a_stage_that_died_fails_even_with_passing_checks(self):
        run = Run()
        st = run.stage("x")
        st.passed("fine")
        st.error = "ValueError: boom"
        self.assertEqual(st.status, Status.FAIL)
        self.assertEqual(run.exit_code(), 1)

    def test_skipped_unavailable_and_manual_never_fail_and_never_look_like_a_pass(self):
        for status in (Status.SKIPPED, Status.UNAVAILABLE, Status.MANUAL_OPTIONAL):
            st = Stage("s")
            st.add("c", status, "why")
            self.assertEqual(st.status, status)
            run = Run()
            run.stages.append(st)
            self.assertEqual(run.exit_code(), 0)
        mixed = Stage("m")
        mixed.passed("p")
        mixed.add("c", Status.SKIPPED, "why")
        self.assertEqual(mixed.status, Status.PASS)
        self.assertEqual(Stage("empty").status, Status.SKIPPED)

    def test_expect_maps_a_condition_to_pass_or_fail(self):
        st = Stage("s")
        self.assertEqual(st.expect("a", True, "ok", "bad").status, Status.PASS)
        self.assertEqual(st.expect("b", False, "ok", "bad").detail, "bad")

    def test_manifest_and_summary_are_generated_from_the_run(self):
        run = Run(started="t0", finished="t1")
        run.context = {"branch": "kde/x", "main_sha": "abc"}
        st = run.stage("visual")
        st.seconds = 2.5
        st.passed("captures", "70 captures", captures=70)
        run.stage("native").failed("gate", "exit 101 | with a pipe")
        with tempfile.TemporaryDirectory() as d:
            write_manifest(run, Path(d) / "manifest.json")
            data = json.loads((Path(d) / "manifest.json").read_text())
        self.assertEqual(data["overall"], "FAIL")
        self.assertEqual([s["status"] for s in data["stages"]], ["PASS", "FAIL"])
        self.assertEqual(data["stages"][0]["checks"][0]["data"], {"captures": 70})
        text = render_summary(run)
        self.assertIn("# KDE Plasma native acceptance: FAIL", text)
        self.assertIn("## FAILED", text)
        self.assertIn("**native: gate**: exit 101 | with a pipe", text)
        self.assertIn("exit 101 \\| with a pipe", text)  # a pipe in a detail cannot break the table
        self.assertIn("| visual | PASS | 1 | 2.5 |", text)


GATE_OUT = """running 4 tests
gate walk (full profile): 115.2 s of app log
  replay entry, step 52 to first frame: 0.7 s (budget 30 s)
replay-row-high: the key light's shadow darkens 2.48 % of the capture (23788 px) by 8.53 % (need at least 1 % and more than 5 %)
accent: palette accent #000000, highlight #3daee9, resolved #3daee9
keyboard contract: 20 lines, all ok, ended by Ctrl+Q
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 116.78s
"""
GATE_LOG = """     0.011 qt.rhi.general: OpenGL VENDOR: Intel RENDERER: Mesa Intel(R) UHD Graphics 630 (CFL GT2) VERSION: 4.6 (Compatibility Profile) Mesa 26.2.3
     6.701 qt.graphs2d.axis.properties: QAbstractAxis::setColor. Tried to use invalid color.
    31.686 qt.graphs2d.axis.properties: QAbstractAxis::setColor. Tried to use invalid color.
     0.412 qt.qpa.theme: Successfully created platform theme "kde" via createPlatformTheme
"""


class GateParsing(unittest.TestCase):
    def parsed(self, out=GATE_OUT, log=GATE_LOG, rc=0, captures=70):
        return gates.parse_gate("g", "cargo test", rc, 117.0, out, log, captures)

    def test_a_healthy_native_full_walk(self):
        r = self.parsed()
        self.assertEqual((r.profile, r.walk_seconds, r.replay_entry_seconds, r.passed_tests, r.failed_tests), ("full", 115.2, 0.7, 4, 0))
        self.assertEqual(r.shadow, {"percent_of_capture": 2.48, "pixels": 23788, "darkening_percent": 8.53})
        self.assertEqual(r.renderer["renderer"], "Mesa Intel(R) UHD Graphics 630 (CFL GT2)")
        self.assertEqual(r.accent, {"accent": "#000000", "highlight": "#3daee9", "resolved": "#3daee9"})
        self.assertEqual(r.keyboard, {"lines": 20, "ended_by": "Ctrl+Q"})
        self.assertEqual(r.warnings, {"qt.graphs2d.axis.properties: QAbstractAxis::setColor. Tried to use invalid color.": 2})
        self.assertEqual(gates.gate_problems(r, hardware=True, expect_full=True), [])

    def test_the_143_signatures_fail_a_run_even_with_exit_zero(self):
        for text in ("Failed to borrow for role_names: BorrowError(BorrowError)", "panic in ffi function", "thread 'main' panicked at x",
                     "panic in a destructor during cleanup", "thread caused non-unwinding panic. aborting."):
            r = self.parsed(out=GATE_OUT + text)
            problems = gates.gate_problems(r, hardware=True, expect_full=True)
            self.assertTrue(any("abort/panic signature" in p for p in problems), text)

    def test_a_software_renderer_is_not_a_native_result(self):
        log = GATE_LOG.replace("Mesa Intel(R) UHD Graphics 630 (CFL GT2)", "llvmpipe (LLVM 21.1.0, 256 bits)")
        problems = gates.gate_problems(self.parsed(log=log), hardware=True)
        self.assertTrue(any("software rasteriser" in p for p in problems))
        self.assertEqual(gates.gate_problems(self.parsed(log=log), hardware=False), [])

    def test_missing_evidence_is_a_problem_not_a_pass(self):
        r = self.parsed(out="test result: ok. 4 passed; 0 failed; 0 ignored")
        problems = gates.gate_problems(r, hardware=True, expect_full=True)
        for word in ("gate walk", "keyboard-contract", "accent-rule", "shadow", "replay-entry"):
            self.assertTrue(any(word in p for p in problems), word)
        self.assertTrue(any("exit status 101" in p for p in gates.gate_problems(self.parsed(rc=101), hardware=True)))
        self.assertTrue(any("captures" in p for p in gates.gate_problems(self.parsed(captures=14), hardware=True, expect_full=True)))

    def test_a_skipped_shortcut_contract_is_fine_without_a_window_manager_and_a_failure_natively(self):
        out = GATE_OUT.replace("keyboard contract: 20 lines, all ok, ended by Ctrl+Q",
                               "keyboard contract: skipped, no active window under xcb (no window manager)")
        r = self.parsed(out=out)
        self.assertEqual(r.keyboard["skipped"], True)
        self.assertEqual(gates.gate_problems(r, hardware=False, expect_full=True), [])      # generic Xvfb run
        problems = gates.gate_problems(r, hardware=True, expect_full=True)                # a native run
        self.assertTrue(any("skipped on a native run" in p for p in problems), problems)

    def test_a_starved_window_is_named_as_the_reason_not_left_to_look_like_an_app_bug(self):
        out = GATE_OUT + "  holds that ran out their tick bound: 11 (steps 2, 4, 13, 49)\n"
        r = self.parsed(out=out)
        self.assertEqual(r.starved_holds, {"count": 11, "steps": "2, 4, 13, 49"})
        problems = gates.gate_problems(r, hardware=True, expect_full=True)
        self.assertTrue(any("starved of frames" in p and "occluded" in p for p in problems), problems)
        self.assertEqual(gates.gate_problems(self.parsed(), hardware=True, expect_full=True), [])

    def test_the_baseline_tree_is_not_asked_for_this_branchs_summaries(self):
        out = GATE_OUT.replace("accent: palette accent #000000, highlight #3daee9, resolved #3daee9\n", "").replace(
            "keyboard contract: 20 lines, all ok, ended by Ctrl+Q\n", "")
        r = self.parsed(out=out)
        self.assertEqual(len(gates.gate_problems(r, hardware=True, expect_full=True)), 2)
        self.assertEqual(gates.gate_problems(r, hardware=True, expect_full=True, baseline=True), [])
        # ... but a baseline that aborted is still a failure
        aborted = self.parsed(out=out + "Failed to borrow for role_names")
        self.assertTrue(gates.gate_problems(aborted, hardware=True, baseline=True))

    def test_release_quick_command_is_1444s(self):
        self.assertEqual(gates.cargo_command(True), ["cargo", "test", "--release", "--config", "profile.release.debug-assertions=true",
                                                     "-p", "rowplay-app", "--test", "qml_runtime_gate", "--", "--nocapture"])


class PackageClassification(unittest.TestCase):
    def test_kde_stack_paths_are_caught_and_qts_own_names_are_not(self):
        bad = ["usr/lib/libKF6ConfigCore.so.6", "usr/qml/org/kde/kirigami/Kirigami.qml", "usr/plugins/platformthemes/KDEPlasmaPlatformTheme6.so",
               "usr/qml/org/kde/desktop/Button.qml", "usr/share/icons/breeze/actions/16/x.svg", "usr/lib/libKF5KIOCore.so.5",
               "usr/plugins/kf6/kio/file.so", "usr/lib/libplasma-framework.so"]
        hits = package.forbidden_in_paths(bad)
        self.assertEqual(len(hits), len(bad), [p for p in bad if p not in [h[1] for h in hits]])
        fine = ["usr/qml/QtQuick/Controls/Basic/CheckDelegate.qml", "usr/qml/QtQuick/Controls/Fusion/Button.qml", "usr/lib/libQt6Core.so.6",
                "usr/plugins/platformthemes/libqxdgdesktopportal.so", "usr/plugins/platforms/libqwayland.so", "usr/lib/libKeyutils.so.1",
                "usr/lib/libkeyutils.so.1", "usr/plugins/iconengines/libqsvgicon.so", "usr/bin/rowplay-qt"]
        self.assertEqual(package.forbidden_in_paths(fine), [])

    def test_needed_libraries(self):
        text = (" 0x0000000000000001 (NEEDED)             Shared library: [libQt6Core.so.6]\n"
                " 0x0000000000000001 (NEEDED)             Shared library: [libKF6ConfigCore.so.6]\n"
                " 0x0000000000000001 (NEEDED)             Shared library: [libc.so.6]\n")
        libs = package.needed_libs(text)
        self.assertEqual(libs, ["libQt6Core.so.6", "libKF6ConfigCore.so.6", "libc.so.6"])
        self.assertEqual(package.forbidden_needed(libs), ["libKF6ConfigCore.so.6"])
        self.assertEqual(package.forbidden_needed(["libQt6Gui.so.6", "libKeyutils.so.1"]), [])

    def test_comparison_and_plugin_inventory(self):
        a = {"bytes": 100, "files": 3, "inventory": {"usr/bin/x": 10, "usr/plugins/platforms/libqxcb.so": 5, "old": 1}}
        b = {"bytes": 104, "files": 3, "inventory": {"usr/bin/x": 14, "usr/plugins/platforms/libqxcb.so": 5, "new": 1}}
        c = package.compare(a, b)
        self.assertEqual((c["bytes"]["delta"], c["files"]["delta"], c["added"], c["removed"], c["size_changed"]),
                         (4, 0, ["new"], ["old"], [("usr/bin/x", 10, 14)]))
        self.assertEqual(package.plugin_inventory(a["inventory"])["plugins"], {"platforms": ["libqxcb.so"]})

    def test_launch_check_line(self):
        self.assertEqual(package.launch_check_line("a\nlaunch-check: ok - x rendered 30 frames\nb"), "launch-check: ok - x rendered 30 frames")
        self.assertIsNone(package.launch_check_line("nothing"))


class KWinAndX11(unittest.TestCase):
    def test_window_lines(self):
        marker = "ROWPLAY-KWIN-abc"
        text = ('noise\n' + marker + ' caption="rowplay" resourceClass="io.github.shenghaoc.rowplay" resourceName="rowplay-qt" '
                'desktopFileName="io.github.shenghaoc.rowplay" pid=74250 wayland=true width=1200 height=800\n'
                + marker + " end\n")
        wins = kwin.parse_windows(text, marker)
        self.assertEqual(len(wins), 1)
        self.assertEqual((wins[0]["desktopFileName"], wins[0]["pid"], wins[0]["wayland"], wins[0]["width"]), ("io.github.shenghaoc.rowplay", 74250, True, 1200))
        self.assertEqual(kwin.parse_windows("nothing", marker), [])

    def test_xprop(self):
        text = ('WM_CLASS(STRING) = "rowplay-app", "rowplay-qt"\n_KDE_NET_WM_DESKTOP_FILE(UTF8_STRING) = "io.github.shenghaoc.rowplay"\n'
                '_GTK_APPLICATION_ID(UTF8_STRING) = "io.github.shenghaoc.rowplay"\n')
        self.assertEqual(stages.parse_xprop(text), {"WM_CLASS": ["rowplay-app", "rowplay-qt"],
                                                    "_KDE_NET_WM_DESKTOP_FILE": "io.github.shenghaoc.rowplay", "_GTK_APPLICATION_ID": "io.github.shenghaoc.rowplay"})


class ServicesAudit(unittest.TestCase):
    def tree(self, files):
        d = tempfile.TemporaryDirectory()
        for rel, text in files.items():
            p = Path(d.name) / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text)
        self.addCleanup(d.cleanup)
        return Path(d.name)

    def test_uses_are_found_and_absences_are_not_applicable(self):
        root = self.tree({"qml/A.qml": "onLinkActivated: Qt.openUrlExternally(link)\n// FileDialog in a comment\n",
                          "crates/x/src/lib.rs": "use keyring::Entry;\n"})
        rows = {r["capability"]: r for r in services.audit(root)}
        self.assertEqual(rows["URL opening"]["verdict"], "Qt/XDG sufficient")
        self.assertEqual(rows["File chooser"]["verdict"], "not applicable")  # only in a comment
        self.assertEqual(rows["Secret Service (token store)"]["verdict"], "Qt/XDG sufficient")

    def test_the_mac_only_menu_bar_is_not_applicable_on_linux(self):
        root = self.tree({"qml/RowPlay/Main.qml": "Instantiator {\n    active: root.isMac\n    delegate: Platform.MenuBar {\n"})
        self.assertEqual({r["capability"]: r for r in services.audit(root)}["Global menu"]["verdict"], "not applicable")

    def test_the_real_repository(self):
        rows = {r["capability"]: r["verdict"] for r in services.audit(REPO)}
        self.assertEqual(rows["URL opening"], "not applicable")
        self.assertEqual(rows["Global menu"], "not applicable")
        self.assertNotIn("needs code", rows.values())


class Leftovers(unittest.TestCase):
    """stage_leftovers formats its failure text only when there is a failure (it once indexed an empty list)."""

    def ctx(self, locked, pgrep=""):
        from kdeacc import session as session_mod
        from kdeacc.shell import Done

        class FakeRunner:
            def run(self, argv, **kw):
                return Done(argv, 0, pgrep, "", 0.0)

        class Inhibitor:
            cookie, note = 7, ""

        self.saved = session_mod.locked_since
        session_mod.locked_since = lambda runner, since: locked
        self.addCleanup(lambda: setattr(session_mod, "locked_since", self.saved))
        c = stages.Ctx(REPO, REPO, Path("/x"), Path("/tmp"), FakeRunner(), None)
        c.run_started, c.inhibitor = 1.0, Inhibitor()
        return c

    def test_no_lock_and_no_processes_passes(self):
        st = stages.stage_leftovers(self.ctx([]))
        self.assertEqual([c.status for c in st.checks], [Status.PASS, Status.PASS])

    def test_a_lock_during_the_run_fails_with_its_first_journal_line(self):
        st = stages.stage_leftovers(self.ctx(["2026-09-30T00:04:21+08:00 fedora kscreenlocker_greet[1]: started"]))
        bad = [c for c in st.checks if c.status == Status.FAIL]
        self.assertEqual(len(bad), 1)
        self.assertIn("kscreenlocker_greet[1]: started", bad[0].detail)

    def test_a_left_behind_process_fails(self):
        st = stages.stage_leftovers(self.ctx([], pgrep="4242 rowplay-app\n"))
        self.assertTrue(any(c.status == Status.FAIL and "rowplay-app" in c.detail for c in st.checks))

    def test_only_a_pid_this_run_launched_counts_and_an_unrelated_apprun_never_does(self):
        import os
        c = self.ctx([])
        c.launched_pids = {2 ** 22 + 12345}      # long gone
        self.assertEqual([x.status for x in stages.stage_leftovers(c).checks], [Status.PASS, Status.PASS])
        c = self.ctx([])
        c.launched_pids = {os.getpid()}          # alive: this test process stands in for a launched app
        bad = [x for x in stages.stage_leftovers(c).checks if x.status == Status.FAIL]
        self.assertEqual(len(bad), 1)
        self.assertIn("launched by this run", bad[0].detail)

    def test_alive_pids_lists_only_running_processes(self):
        import os
        self.assertEqual(stages.alive_pids({os.getpid(), 2 ** 22 + 54321}), [os.getpid()])
        self.assertEqual(stages.alive_pids(set()), [])

    def test_no_pgrep_or_kill_in_the_harness_uses_the_generic_wrapper_name(self):
        """Another linuxdeploy-built AppImage shares `AppRun.wrapped`: matching it by name could end someone's application."""
        import re
        for path in (Path(stages.__file__), Path(stages.__file__).with_name("kwin.py")):
            for number, line in enumerate(path.read_text().splitlines(), 1):
                if line.lstrip().startswith("#"):
                    continue
                self.assertFalse(re.search(r"(pgrep|pkill|killall).*AppRun", line), f"{path.name}:{number}: {line.strip()}")


class Cli(unittest.TestCase):
    def test_plan_orders_and_rejects_unknown_stages(self):
        self.assertEqual(acceptance.plan(["all"]), acceptance.ORDER)
        self.assertEqual(acceptance.plan(["identity", "probe"]), ["probe", "identity"])
        with self.assertRaises(SystemExit):
            acceptance.plan(["nonsense"])

    def test_dry_run_runs_nothing_and_changes_nothing(self):
        with tempfile.TemporaryDirectory() as d:
            code = acceptance.main(["--dry-run", "--output", str(Path(d) / "out"), "all"])
            self.assertEqual(code, 0)
            self.assertFalse((Path(d) / "out").exists())

    def test_session_stages_are_skipped_without_permission(self):
        class Args:
            allow_session_changes = False
        ctx = stages.Ctx(REPO, REPO, Path("/x"), Path("/tmp"), None, Args())
        for fn in (stages.stage_appearance, stages.stage_identity):
            st = fn(ctx)
            self.assertEqual([c.status for c in st.checks], [Status.SKIPPED])
            self.assertIn("--allow-session-changes", st.checks[0].detail)


if __name__ == "__main__":
    unittest.main()
