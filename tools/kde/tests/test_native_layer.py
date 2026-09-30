# SPDX-License-Identifier: GPL-3.0-or-later
"""The native layer's review findings: the configured Qt in the X11 probe, the generic walk's full profile,
and what an interruption does to the run."""
import inspect
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.dont_write_bytecode = True

import acceptance  # noqa: E402
from kdeacc import baseline, gates, plasma, session, shell, stages  # noqa: E402
from kdeacc.results import Run, Stage, Status, render_summary  # noqa: E402
from tests.test_plasma_host import FakePlasma  # noqa: E402

REPO = Path(__file__).resolve().parents[3]


class Recorder(shell.Runner):
    """A real Runner that records the environment each command would get, and runs nothing."""

    def __init__(self, stdout=""):
        super().__init__()
        self.calls, self.stdout = [], stdout

    def run(self, argv, cwd=None, env=None, timeout=None, tag="", stdin=None):
        self.calls.append({"argv": [str(a) for a in argv], "env": dict(env if env is not None else shell.host_env()), "tag": tag})
        return shell.Done([str(a) for a in argv], 0, self.stdout, "", 0.0)


class X11IdentityUsesTheConfiguredQt(unittest.TestCase):
    XPROP = ('WM_CLASS(STRING) = "rowplay-qt", "rowplay-qt"\n_KDE_NET_WM_DESKTOP_FILE(UTF8_STRING) = "io.github.shenghaoc.rowplay"\n'
             '_GTK_APPLICATION_ID(UTF8_STRING) = "io.github.shenghaoc.rowplay"\n')

    def tree(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        binary = Path(tmp.name) / "target" / "debug" / "rowplay-app"
        binary.parent.mkdir(parents=True)
        binary.write_text("")
        return Path(tmp.name)

    def test_a_non_default_qt_directory_reaches_the_command_environment(self):
        rec = Recorder(self.XPROP)
        tree = self.tree()
        with mock.patch.dict(os.environ, {"LD_LIBRARY_PATH": "/contaminated/bundled/lib", "XDG_CURRENT_DESKTOP": "KDE"}):
            found = stages.x11_identity(rec, tree, tree, "/opt/custom-qt-6.12")
        env = rec.calls[0]["env"]
        self.assertEqual(env["LD_LIBRARY_PATH"], "/opt/custom-qt-6.12/lib")
        self.assertNotIn("XDG_CURRENT_DESKTOP", env)     # still the generic environment
        self.assertEqual(found["_KDE_NET_WM_DESKTOP_FILE"], "io.github.shenghaoc.rowplay")

    def test_no_home_directory_qt_path_is_left_in_the_harness(self):
        source = Path(stages.__file__).read_text()
        self.assertNotIn("Qt/6.11.2", source)
        self.assertNotIn("Path.home()}/Qt", source)
        self.assertIn("x11_identity(ctx.runner, tree, out, ctx.qt_dir)", inspect.getsource(stages.stage_generic))


class GenericWalkIsFull(unittest.TestCase):
    QUICK = {"ROWPLAY_GATE_PROFILE": "quick", "ROWPLAY_EXIT_AFTER_FRAMES": "3", "ROWPLAY_SMOKE_GATE": "1", "ROWPLAY_SYNC_MOCK": "1",
             "ROWPLAY_FORCE_COLOR_SCHEME": "dark", "QT_SCALE_FACTOR": "2", "XDG_CURRENT_DESKTOP": "KDE", "QT_QPA_PLATFORM": "wayland"}

    def test_the_overrides_pin_full_and_remove_every_other_inherited_gate_variable(self):
        with mock.patch.dict(os.environ, self.QUICK):
            env = stages.generic_gate_env("/tmp/artifacts")
        self.assertEqual(env["ROWPLAY_GATE_PROFILE"], "full")
        self.assertEqual((env["QT_QPA_PLATFORM"], env["LIBGL_ALWAYS_SOFTWARE"], env["ROWPLAY_QT_SMOKE"]), ("xcb", "1", "1"))
        self.assertEqual(env["ROWPLAY_SMOKE_ARTIFACT_DIR"], "/tmp/artifacts")
        for name in ("ROWPLAY_EXIT_AFTER_FRAMES", "ROWPLAY_SMOKE_GATE", "ROWPLAY_SYNC_MOCK", "ROWPLAY_FORCE_COLOR_SCHEME", "QT_SCALE_FACTOR",
                     "XDG_CURRENT_DESKTOP"):
            self.assertIsNone(env[name], name)     # None: Runner.in_tree removes it from what it inherits

    def test_the_generic_stage_constructs_full_although_the_parent_says_quick(self):
        rec = Recorder()
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        ctx = stages.Ctx(REPO, Path(tmp.name), Path("/x"), Path(tmp.name), rec, None)
        ctx.baseline_validated = True
        ctx.probe = lambda *a, **k: (None, shell.Done([], 1, "", "", 0.0))
        with mock.patch.dict(os.environ, self.QUICK), mock.patch.object(stages.shutil, "which", lambda name: "/usr/bin/" + name):
            stages.stage_generic(ctx)
        walks = [c for c in rec.calls if c["tag"].startswith("generic-")]
        self.assertEqual([c["tag"] for c in walks], ["generic-baseline", "generic-branch"])
        for call in walks:
            self.assertEqual(call["env"]["ROWPLAY_GATE_PROFILE"], "full", call["tag"])
            for name in ("ROWPLAY_EXIT_AFTER_FRAMES", "ROWPLAY_SMOKE_GATE", "XDG_CURRENT_DESKTOP", "QT_SCALE_FACTOR"):
                self.assertNotIn(name, call["env"], (call["tag"], name))

    def test_a_native_gate_pins_its_own_profile_and_ignores_the_callers_variables(self):
        rec = Recorder()
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        with mock.patch.dict(os.environ, {"ROWPLAY_GATE_PROFILE": "full", "ROWPLAY_EXIT_AFTER_FRAMES": "5", "ROWPLAY_QT_SMOKE": "1",
                                          "LIBGL_ALWAYS_SOFTWARE": "1", "QT_FONT_DPI": "200"}):
            gates.run_gate(rec, tmp.name, tmp.name, "n", profile="quick", native=True)
        env = rec.calls[0]["env"]
        self.assertEqual((env["ROWPLAY_GATE_PROFILE"], env["QT_QPA_PLATFORM"]), ("quick", "wayland"))
        for name in ("ROWPLAY_EXIT_AFTER_FRAMES", "ROWPLAY_QT_SMOKE", "LIBGL_ALWAYS_SOFTWARE", "QT_FONT_DPI"):
            self.assertNotIn(name, env, name)


class Interruption(unittest.TestCase):
    def setUp(self):
        self.saved = {sig: signal.getsignal(sig) for sig in session.INTERRUPT_SIGNALS}
        for sig, handler in self.saved.items():
            self.addCleanup(signal.signal, sig, handler)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.kdeglobals = Path(self.tmp.name) / "kdeglobals"
        self.kdeglobals.write_text("[General]\nColorSchemeHash=abc\n")
        self.original = self.kdeglobals.read_bytes()
        self.later = []

        class Inhibitor:
            cookie, note = 1, ""
            def __enter__(self): return self
            def __exit__(self, *exc): return False

        def later_stage(ctx):
            self.later.append("checks")
            return Stage("repo-checks")

        patches = [mock.patch.object(session, "IdleInhibitor", Inhibitor), mock.patch.object(session, "locked_since", lambda r, s: []),
                   mock.patch.dict(acceptance.FUNCS, {"checks": later_stage}),
                   mock.patch.object(stages.time, "sleep", lambda s: None),
                   mock.patch.dict(os.environ, {"WAYLAND_DISPLAY": "wayland-0"})]
        for patch in patches:
            patch.start()
            self.addCleanup(patch.stop)
        self.raiser = mock.patch.object(stages.kwin, "Raiser", lambda runner: mock.MagicMock())
        self.raiser.start()
        self.addCleanup(self.raiser.stop)

    def main(self, *stage_names):
        out = Path(self.tmp.name) / "evidence"
        code = acceptance.main([*stage_names, "--allow-session-changes", "--repo", str(REPO), "--output", str(out)])
        return code, json.loads((out / "manifest.json").read_text()), (out / "summary.md").read_text()

    def interrupt_in_the_appearance_stage(self, signum):
        """The real appearance stage on a fake desktop; a real signal arrives during its first gate."""
        fake = FakePlasma(self.kdeglobals)

        def gate(runner, tree, out_dir, name, *a, **k):
            os.kill(os.getpid(), signum)
            time.sleep(0.5)      # the handler raises before this returns
            raise AssertionError("the signal did not interrupt the gate")

        with mock.patch.object(plasma, "Plasma", lambda runner, probe_fn: fake), mock.patch.object(gates, "run_gate", gate):
            return self.main("appearance", "checks"), fake

    def test_each_signal_ends_the_run_after_the_desktop_is_restored_and_recorded(self):
        for signum, code in ((signal.SIGINT, 130), (signal.SIGTERM, 143), (signal.SIGHUP, 129)):
            self.later.clear()
            (exit_code, manifest, summary), fake = self.interrupt_in_the_appearance_stage(signum)
            name = signal.Signals(signum).name
            self.assertEqual(exit_code, code, name)
            self.assertEqual(self.later, [], f"{name}: checks must not run after an interruption")
            self.assertEqual(manifest["overall"], "INTERRUPTED")
            self.assertIn(name, manifest["interrupted"])
            self.assertEqual(self.kdeglobals.read_bytes(), self.original, f"{name}: the desktop file was not put back")
            by_name = {s["name"]: s for s in manifest["stages"]}
            self.assertEqual([s["name"] for s in manifest["stages"]], ["appearance", "leftovers"], name)
            restored = next(c for c in by_name["appearance"]["checks"] if c["name"].startswith("the desktop is restored exactly"))
            self.assertEqual(restored["status"], "PASS", f"{name}: restoration evidence must stay in the manifest")
            failed = next(c for c in by_name["appearance"]["checks"] if c["name"] == "the appearance run completed")
            self.assertIn("interrupted", failed["detail"])
            self.assertIn("interrupted", summary)

    def test_a_stage_that_raises_the_interruption_itself_also_stops_the_run(self):
        def dying(ctx):
            raise session.Interrupted(signal.SIGTERM)
        with mock.patch.dict(acceptance.FUNCS, {"probe": dying}):
            code, manifest, _ = self.main("probe", "checks")
        self.assertEqual((code, self.later, manifest["overall"]), (143, [], "INTERRUPTED"))
        self.assertEqual([s["name"] for s in manifest["stages"]], ["probe", "leftovers"])

    def test_the_processes_this_run_launched_are_ended_and_nothing_else_is(self):
        mine = subprocess.Popen(["sleep", "60"])
        stranger = subprocess.Popen(["sleep", "60"])
        self.addCleanup(lambda: (stranger.kill(), stranger.wait()))
        self.addCleanup(lambda: (mine.kill(), mine.wait()))

        def launching(ctx):
            ctx.launched_pids.add(mine.pid)
            raise session.Interrupted(signal.SIGINT)

        with mock.patch.dict(acceptance.FUNCS, {"probe": launching}):
            code, manifest, _ = self.main("probe")
        self.assertEqual(code, 130)
        self.assertIsNotNone(mine.wait(timeout=10), "the launched process must be ended")
        self.assertIsNone(stranger.poll(), "an unrelated process must never be touched")

    def test_an_uninterrupted_run_is_unchanged(self):
        code, manifest, _ = self.main("checks")
        self.assertEqual((code, self.later, manifest["overall"]), (0, ["checks"], "PASS"))
        self.assertEqual(manifest["interrupted"], "")

    def test_the_result_model(self):
        run = Run()
        self.assertEqual(run.exit_code(), 0)
        run.interrupted, run.interrupt_signal = "signal 15 (SIGTERM)", 15
        self.assertEqual((run.failed, run.exit_code(), run.to_dict()["overall"]), (True, 143, "INTERRUPTED"))
        self.assertIn("interrupted: signal 15", render_summary(run))

    def test_the_session_transaction_restores_on_sighup_too(self):
        fake = FakePlasma(self.kdeglobals)
        tx = plasma.SessionTransaction(fake, allow=True)
        with self.assertRaises(session.Interrupted) as raised:
            with tx:
                fake.apply_scheme("BreezeDark")
                os.kill(os.getpid(), signal.SIGHUP)
                time.sleep(0.5)
        self.assertEqual(raised.exception.signum, signal.SIGHUP)
        self.assertTrue(tx.restored)
        self.assertEqual(self.kdeglobals.read_bytes(), self.original)


if __name__ == "__main__":
    unittest.main()
