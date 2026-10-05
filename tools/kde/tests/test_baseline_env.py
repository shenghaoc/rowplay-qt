# SPDX-License-Identifier: GPL-3.0-or-later
"""The acceptance baseline, the three environments, timeouts and the probe's icon-theme evidence."""
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.dont_write_bytecode = True

import acceptance  # noqa: E402
from kdeacc import baseline, host, qtprobe, shell, stages  # noqa: E402
from kdeacc.results import Status  # noqa: E402

REPO = Path(__file__).resolve().parents[3]
GIT_ENV = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@example.org", "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@example.org",
           "GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_SYSTEM": os.devnull}


class Repo:
    """A throw-away repository: baseline commit A, a feature commit B on top of it, and an unrelated root commit U."""

    def __init__(self, testcase):
        self.tmp = tempfile.TemporaryDirectory()
        testcase.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.main = self.root / "repo"
        self.main.mkdir()
        self.git("init", "-q", "-b", "trunk")
        self.a = self.commit("a.txt", "baseline\n", "A")
        self.git("checkout", "-q", "-b", "feature")
        self.b = self.commit("b.txt", "feature\n", "B")
        self.git("checkout", "-q", "--orphan", "elsewhere")
        self.git("rm", "-rfq", ".")
        self.u = self.commit("u.txt", "unrelated\n", "U")
        self.git("checkout", "-q", "feature")

    def git(self, *args, cwd=None):
        done = subprocess.run(["git", *args], cwd=cwd or self.main, capture_output=True, text=True, env={**os.environ, **GIT_ENV})
        assert done.returncode == 0, (args, done.stderr)
        return done.stdout.strip()

    def commit(self, name, text, message):
        (self.main / name).write_text(text)
        self.git("add", name)
        self.git("commit", "-q", "-m", message)
        return self.git("rev-parse", "HEAD")

    def worktree(self, name, sha, branch=None):
        path = self.root / name
        self.git("worktree", "add", "-q", *(["-b", branch] if branch else ["--detach"]), str(path), sha)
        return path


class BaselineContract(unittest.TestCase):
    def setUp(self):
        self.repo = Repo(self)
        self.runner = shell.Runner()

    def check(self, subject, tree, expected=None):
        return baseline.check_baseline(self.runner, subject, tree, expected or self.repo.a)

    def failed(self, checks):
        return [c.name for c in checks if not c.ok]

    def test_the_configured_baseline_is_the_post_144_commit_not_a_branch(self):
        self.assertEqual(baseline.BASELINE_SHA, "046c2e30062f7ca38c725307ecc73c6ea62777d2")
        self.assertNotIn("main", baseline.BASELINE_SHA)

    def test_a_checkout_of_the_baseline_commit_passes_whatever_its_branch_is_called(self):
        tree = self.repo.worktree("base-branch", self.repo.a, branch="totally-not-main")
        self.assertEqual(self.failed(self.check(self.repo.main, tree)), [])

    def test_a_detached_checkout_at_the_baseline_passes(self):
        tree = self.repo.worktree("base-detached", self.repo.a)
        self.assertEqual(self.failed(self.check(self.repo.main, tree)), [])

    def test_the_tree_under_test_as_its_own_baseline_fails(self):
        # the same directory and commit, whichever commit is expected
        for expected in (self.repo.a, self.repo.b):
            failed = self.failed(self.check(self.repo.main, self.repo.main, expected))
            self.assertTrue(any("not its own baseline" in name for name in failed), (expected, failed))

    def test_the_same_commit_in_another_directory_is_still_its_own_baseline(self):
        twin = self.repo.worktree("twin", self.repo.b)
        failed = self.failed(self.check(self.repo.main, twin, self.repo.b))
        self.assertEqual([n for n in failed if "own baseline" in n], ["the tree under test is not its own baseline"])

    def test_a_wrong_clean_commit_fails(self):
        for label, sha in (("later feature commit", self.repo.b), ("unrelated root commit", self.repo.u)):
            tree = self.repo.worktree("wrong-" + label.split()[0], sha)
            failed = self.failed(self.check(self.repo.main, tree))
            self.assertTrue(any("acceptance baseline" in n and "checkout is at" in n for n in failed), (label, failed))

    def test_a_tree_not_built_on_the_baseline_fails(self):
        base = self.repo.worktree("base", self.repo.a)
        stray = self.repo.worktree("stray", self.repo.u)
        failed = self.failed(self.check(stray, base))
        self.assertEqual(len(failed), 1)
        self.assertIn("descends from the baseline", failed[0])

    def test_a_baseline_is_found_by_commit_not_by_branch(self):
        tree = self.repo.worktree("found", self.repo.a, branch="whatever")
        self.assertEqual(baseline.find_baseline_tree(self.runner, self.repo.main, self.repo.a), tree)
        self.assertIsNone(baseline.find_baseline_tree(self.runner, self.repo.main, self.repo.u))
        # the subject itself is never its own baseline
        self.assertIsNone(baseline.find_baseline_tree(self.runner, self.repo.main, self.repo.b))

    def stage(self, tree, expected=None):
        ctx = stages.Ctx(self.repo.main, tree, Path("/x"), Path(tempfile.mkdtemp()), self.runner, None, baseline_sha=expected or self.repo.a)
        self.addCleanup(lambda: __import__("shutil").rmtree(ctx.out, ignore_errors=True))
        return stages.stage_guards(ctx)

    def test_guards_stop_at_a_wrong_baseline_instead_of_reporting_a_comparison_with_itself(self):
        st = self.stage(self.repo.main)
        names = [c.name for c in st.checks]
        self.assertEqual(st.status, Status.FAIL)
        self.assertIn("the baseline comparisons were not run", names)
        self.assertFalse(any("containment" in n or "lockfile" in n for n in names), names)

    def test_guards_without_a_baseline_say_what_is_needed(self):
        st = self.stage(None)
        self.assertEqual([c.status for c in st.checks], [Status.FAIL])
        self.assertIn(self.repo.a, st.checks[0].detail)


class DryRunNeedsNoBaseline(unittest.TestCase):
    def run_cli(self, *argv):
        buf = io.StringIO()
        with tempfile.TemporaryDirectory() as d, redirect_stdout(buf):
            code = acceptance.main([*argv, "--output", str(Path(d) / "out")])
            self.assertFalse((Path(d) / "out").exists(), "a dry run must not write")
        return code, buf.getvalue()

    def test_dry_run_from_a_checkout_with_no_baseline_worktree_prints_the_plan(self):
        lone = Repo(self)   # a repository with no worktree at the baseline commit, and no `main`
        code, out = self.run_cli("--dry-run", "--repo", str(lone.main), "all")
        self.assertEqual(code, 0)
        self.assertIn("  - guards", out)
        self.assertIn(f"baseline: would be required by guards: {baseline.BASELINE_SHA}", out)

    def test_dry_run_validates_nothing_even_when_a_bad_baseline_is_named(self):
        code, out = self.run_cli("--dry-run", "--baseline-tree", "/does/not/exist", "all")
        self.assertEqual(code, 0)

    def test_stages_that_compare_nothing_need_no_baseline(self):
        code, out = self.run_cli("--dry-run", "probe", "services")
        self.assertEqual(code, 0)
        self.assertIn("baseline: not required by these stages", out)

    def test_the_old_option_name_is_an_alias(self):
        args = acceptance.parse(["--main-tree", "/somewhere", "probe"])
        self.assertEqual(args.baseline_tree, Path("/somewhere"))
        self.assertEqual(acceptance.parse([]).baseline_sha, baseline.BASELINE_SHA)

    def test_a_real_run_without_a_baseline_stops_with_the_requirement(self):
        lone = Repo(self)
        with tempfile.TemporaryDirectory() as d, self.assertRaises(SystemExit) as raised:
            acceptance.main(["guards", "--repo", str(lone.main), "--output", str(Path(d) / "o")])
        self.assertIn(baseline.BASELINE_SHA, str(raised.exception))


BUNDLED = ["QT_QPA_PLATFORM_PLUGIN_PATH", "QT_PLUGIN_PATH", "QML_IMPORT_PATH", "QML2_IMPORT_PATH", "QT_QPA_PLATFORMTHEME",
           "QT_STYLE_OVERRIDE", "QT_QUICK_CONTROLS_STYLE", "LD_LIBRARY_PATH", "QT_QUICK_CONTROLS_CONF", "QT_QPA_GENERIC_PLUGINS"]


class Environments(unittest.TestCase):
    def dirty(self):
        return mock.patch.dict(os.environ, {v: "/bundled/qt/" + v.lower() for v in BUNDLED} | {"XDG_CURRENT_DESKTOP": "KDE"})

    def test_host_env_carries_no_bundled_qt_variable(self):
        with self.dirty():
            env = shell.host_env()
        for name in BUNDLED:
            self.assertNotIn(name, env, name)
        self.assertIn("PATH", env)

    def test_the_bundled_qt_env_has_its_library_directory_and_nothing_else_of_qts(self):
        with self.dirty():
            env = shell.bundled_qt_env("/opt/Qt/6.11.2/gcc_64", {"QT_QPA_PLATFORM": "offscreen"})
        self.assertEqual(env["LD_LIBRARY_PATH"], "/opt/Qt/6.11.2/gcc_64/lib")
        self.assertEqual(env["QT_QPA_PLATFORM"], "offscreen")
        for name in BUNDLED:
            if name != "LD_LIBRARY_PATH":
                self.assertNotIn(name, env, name)

    def test_the_generic_env_has_neither_bundled_qt_nor_a_desktop(self):
        with self.dirty():
            env = shell.generic_env()
        for name in BUNDLED + ["XDG_CURRENT_DESKTOP"]:
            self.assertNotIn(name, env, name)

    def test_a_stage_may_still_set_a_variable_explicitly(self):
        with self.dirty():
            self.assertEqual(shell.host_env({"QT_QPA_PLATFORM": "xcb"})["QT_QPA_PLATFORM"], "xcb")

    def test_the_probe_runs_in_the_bundled_environment_not_the_callers(self):
        seen = {}

        class Fake:
            def run(self, argv, **kw):
                seen.update(kw["env"])
                return shell.Done(argv, 0, "qml: PROBE " + json.dumps({"accent": "#000000"}) + "\n", "", 0.0)

        with self.dirty():
            qtprobe.run_probe(Fake(), REPO, "/opt/qt")
        self.assertEqual(seen["LD_LIBRARY_PATH"], "/opt/qt/lib")
        for name in BUNDLED:
            if name != "LD_LIBRARY_PATH":
                self.assertNotIn(name, seen, name)


class Timeouts(unittest.TestCase):
    def raising(self, stdout, stderr):
        def run(*a, **k):
            raise subprocess.TimeoutExpired(cmd="slow", timeout=3, output=stdout, stderr=stderr)
        return mock.patch.object(shell.subprocess, "run", run)

    def test_every_shape_of_captured_output_becomes_a_named_timeout(self):
        for stdout, stderr, want_out, want_err in ((b"bytes out", b"bytes err", "bytes out", "bytes err"),
                                                   (None, None, "", ""),
                                                   ("text out", "text err", "text out", "text err"),
                                                   (b"only out", None, "only out", ""),
                                                   (None, b"only err", "", "only err")):
            with tempfile.TemporaryDirectory() as d, self.raising(stdout, stderr):
                runner = shell.Runner(Path(d) / "log")
                done = runner.run(["slow"], timeout=3, tag="t")
                entry = json.loads((Path(d) / "log").read_text())
            self.assertEqual((done.rc, done.timed_out, done.out), (124, True, want_out), (stdout, stderr))
            self.assertTrue(done.err.startswith(want_err) and done.err.endswith("timeout after 3s"), done.err)
            self.assertIn(want_out + want_err, done.text)
            self.assertTrue(entry["timed_out"] and entry["rc"] == 124)

    def test_a_real_timeout_keeps_what_the_command_printed(self):
        # sh, not a Python child: nothing that has to start up before it prints, so a loaded machine cannot beat the timeout
        runner = shell.Runner()
        done = runner.run(["sh", "-c", "printf partial; printf diag >&2; exec sleep 30"], timeout=2)
        self.assertEqual((done.rc, done.timed_out), (124, True))
        self.assertIn("partial", done.out)
        self.assertIn("diag", done.err)

    def test_a_stage_fails_normally_on_a_timeout_instead_of_raising(self):
        with self.raising(b"probe output", b"probe error"):
            done = shell.Runner().run(["qml"], timeout=3)
        self.assertFalse(done.ok)
        self.assertIn("timeout after 3s", done.text)

    def test_text_of(self):
        self.assertEqual([shell.text_of(v) for v in (None, b"a", "b", b"\xff")], ["", "a", "b", "�"])


class LeftoversMatchOnlyTheHarnessHelpers(unittest.TestCase):
    """The leftovers check once matched any command line that contained `atspi_walk`: a clean run failed whenever a shell,
    editor or grep on the machine merely mentioned it (found when the suite failed once, unexplained, while the invoking
    command line said `tools/kde/atspi_walk.py`)."""

    PATTERN = re.compile(stages.HELPER_PROCESS_PATTERN)
    HELPERS = ("/home/u/Qt/6.11.2/gcc_64/bin/qml -I /tmp/kdeacc-probe-x1/mod /r/tools/kde/probe/probe.qml --apptype gui",
               "qml /r/tools/kde/probe/chord-test.qml --apptype gui",
               "/usr/bin/python3 /r/tools/kde/atspi_walk.py --timeout 40",
               "python3.11 atspi_walk.py")
    BYSTANDERS = ("vim tools/kde/atspi_walk.py", "bash -c git diff --stat tools/kde/atspi_walk.py", "grep -rn atspi_walk .",
                  "less /tmp/kdeacc-probe-1/mod/Theme.qml", "python3 -m unittest discover -s tools/kde/tests -t tools/kde",
                  "/usr/bin/python3 -c print('atspi_walk.py')", "tail -f /tmp/atspi_walk.py.log", "qmllint tools/kde/probe/probe.qml")

    def test_the_pattern_reads_helpers_and_not_bystanders(self):
        for line in self.HELPERS:
            self.assertRegex(line, self.PATTERN, line)
        for line in self.BYSTANDERS:
            self.assertNotRegex(line, self.PATTERN, line)

    def leftovers(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        ctx = stages.Ctx(REPO, None, Path("/x"), Path(tmp.name), shell.Runner(), None)
        return stages.stage_leftovers(ctx), Path(tmp.name)

    def spawn(self, argv):
        proc = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.addCleanup(lambda: (proc.kill(), proc.wait()))
        time.sleep(0.3)
        return proc

    def test_a_process_that_only_mentions_the_helper_is_not_a_leftover(self):
        self.spawn(["sh", "-c", "sleep 30 # tools/kde/atspi_walk.py kdeacc-probe /kde/probe/"])
        st, _ = self.leftovers()
        self.assertEqual([c.status for c in st.checks], [Status.PASS], [(c.name, c.detail) for c in st.checks])

    def test_a_real_walk_process_is_a_leftover(self):
        script = Path(tempfile.mkdtemp()) / "atspi_walk.py"
        self.addCleanup(lambda: __import__("shutil").rmtree(script.parent, ignore_errors=True))
        script.write_text("import time\ntime.sleep(30)\n")
        self.spawn([sys.executable, str(script)])
        st, _ = self.leftovers()
        self.assertEqual(st.status, Status.FAIL)
        self.assertIn("atspi_walk.py", st.checks[-1].detail)


class IconThemeEvidence(unittest.TestCase):
    HOST = {"fedora": {"pretty": "Fedora", "version_id": "44"}, "session": {}, **{k: "x" for k in (
        "kernel", "plasma", "kwin", "kde_frameworks", "host_qt", "bundled_qt", "mesa", "gpu_renderer", "xdg_desktop_portal", "xdg_desktop_portal_kde")}}

    def info(self, icon):
        return {"platform": "wayland", "platformThemeNames": ["kde"], "platformThemeCreated": "kde", "iconTheme": icon,
                "window": "#eff0f1", "windowText": "#31363b", "base": "#fcfcfc", "button": "#eff0f1", "highlight": "#3daee9", "accent": "#000000",
                "colorSchemeName": "Light", "fontFamily": "Noto Sans", "fontPointSize": 10, "themeAccentColor": "#3daee9",
                "themeSystemAccentAvailable": True, "fusion": True, "buttonBackground": "ButtonPanel_QMLTYPE_4(0x1)", "contrast": 0,
                "contrastName": "NoPreference"}

    def run_stage(self, icon, chord=None):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)

        class Runner:
            def run(self, argv, **kw):
                return chord or shell.Done(argv, 0, "CHORD summary: 18 of 18 correct\n", "", 0.0)

        ctx = stages.Ctx(REPO, None, Path("/x"), Path(tmp.name), Runner(), None)
        ctx.probe = lambda *a, **k: (self.info(icon), shell.Done([], 0, "", "", 0.0))
        with mock.patch.object(host, "query_host", lambda *a, **k: self.HOST), mock.patch.object(host, "host_gaps", lambda h: []):
            return stages.stage_probe(ctx)

    def icon_check(self, st):
        return next(c for c in st.checks if "icon theme" in c.name)

    def test_missing_icon_theme_evidence_is_a_failure_not_a_pass(self):
        for missing in (None, "", "   "):
            st = self.run_stage(missing)
            self.assertEqual(self.icon_check(st).status, Status.FAIL, missing)
            self.assertEqual(st.status, Status.FAIL)
            self.assertIn("no icon theme was measured", self.icon_check(st).detail)

    def test_any_measured_icon_theme_counts_breeze_is_not_an_invariant(self):
        for theme in ("breeze", "Adwaita", "hicolor", "my-own-theme"):
            st = self.run_stage(theme)
            self.assertEqual(self.icon_check(st).status, Status.PASS, theme)
            self.assertIn(theme, self.icon_check(st).detail)

    def test_evidence_function(self):
        self.assertEqual(qtprobe.icon_theme_evidence({"iconTheme": "breeze"})[0], True)
        self.assertEqual(qtprobe.icon_theme_evidence({})[0], False)

    def chord_check(self, done):
        return next(check for check in self.run_stage("breeze", done).checks if "chord parser" in check.name)

    def test_chord_early_exit_preserves_exit_status_and_captured_output(self):
        check = self.chord_check(shell.Done([], 2, "", "qml: import failed before probe startup\n", 0.0))
        self.assertEqual(check.status, Status.FAIL)
        self.assertIn("no summary line", check.detail)
        self.assertIn("exit 2", check.detail)
        self.assertIn("import failed before probe startup", check.detail)
        self.assertEqual(check.data, {"exit_code": 2, "timed_out": False})

    def test_chord_parser_failures_keep_the_existing_diagnostic(self):
        check = self.chord_check(shell.Done([], 1, "qml: CHORD FAIL Meta+R\nCHORD summary: 17 of 18 correct\n", "", 0.0))
        self.assertEqual(check.status, Status.FAIL)
        self.assertEqual(check.detail, "the chord parser is wrong for: CHORD FAIL Meta+R")

    def test_chord_summary_cannot_hide_a_failed_or_timed_out_process(self):
        for rc, timed_out in ((2, False), (124, True), (0, True)):
            with self.subTest(rc=rc, timed_out=timed_out):
                check = self.chord_check(shell.Done([], rc, "CHORD summary: 18 of 18 correct\n", "shutdown error", 0.0, timed_out))
                self.assertEqual(check.status, Status.FAIL)
                self.assertIn(f"exit {rc}", check.detail)
                self.assertIn("shutdown error", check.detail)

    def test_chord_complete_success_remains_a_pass(self):
        check = self.chord_check(shell.Done([], 0, "CHORD summary: 18 of 18 correct\n", "", 0.0))
        self.assertEqual(check.status, Status.PASS)
        self.assertEqual(check.detail, "CHORD summary: 18 of 18 correct")


if __name__ == "__main__":
    unittest.main()
