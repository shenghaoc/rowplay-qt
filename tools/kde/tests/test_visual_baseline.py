# SPDX-License-Identifier: GPL-3.0-or-later
"""The visual rules apply only to captures shown to be taken at the baseline they were derived against."""
import io
import json
import signal
import sys
import tempfile
import unittest
from contextlib import contextmanager, redirect_stdout
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.dont_write_bytecode = True

import acceptance  # noqa: E402
import derive_rules  # noqa: E402
from kdeacc import baseline, provenance, session, shell, stages  # noqa: E402
from kdeacc.results import Status  # noqa: E402
from tests.test_baseline_env import Repo  # noqa: E402
from tests.test_visual import H, W, canvas, paint  # noqa: E402

REPO = HERE.parents[2]
SHA = baseline.BASELINE_SHA
OTHER = "1" * 40


@contextmanager
def restore_signal_handlers():
    """Confine the CLI entry point's signal changes to this in-process call, even when it raises."""
    saved = {sig: signal.getsignal(sig) for sig in session.INTERRUPT_SIGNALS}
    try:
        yield
    finally:
        for sig, handler in saved.items():
            signal.signal(sig, handler)


def rules_file(directory, sha=SHA):
    data = {"schema": 1,
            "noise_profiles": {"native-hardware": {"max_delta": 1, "max_pixels_2d": 1000, "max_pixels_3d": 300, "calibration": "unchanged"}},
            "sets": {"baseline-vs-branch": {**({"baseline_sha": sha} if sha else {}), "captures": {
                "phase-row-catch": {"size": [W, H], "regions": [[48, 58, 122, 65]], "must_change": True}}}}}
    path = Path(directory) / "rules.json"
    path.write_text(json.dumps(data))
    return path


def ppm(path, buf):
    path.write_bytes(b"P6\n%d %d\n255\n" % (W, H) + bytes(buf))


class Fixture(unittest.TestCase):
    """Baseline and branch capture directories, and a rules file patched in; no test of its own."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)
        self.before, self.after = self.dir / "baseline-full", self.dir / "branch-full"
        for d in (self.before, self.after):
            d.mkdir()
        self.branch_provenance()
        changed = canvas()
        paint(changed, (50, 60, 120, 63), (61, 174, 233))
        ppm(self.before / "phase-row-catch.ppm", canvas())
        ppm(self.after / "phase-row-catch.ppm", changed)
        self.rules = rules_file(self.dir)
        patch = mock.patch.object(stages, "VISUAL_RULES", self.rules)
        patch.start()
        self.addCleanup(patch.stop)

    def provenance(self, commit=SHA, dirty=False, role="baseline"):
        (self.before / provenance.PROVENANCE_FILE).write_text(json.dumps({"role": role, "commit": commit, "dirty": dirty}))

    def branch_provenance(self, commit=OTHER, dirty=False, role="branch"):
        (self.after / provenance.PROVENANCE_FILE).write_text(json.dumps({"role": role, "commit": commit, "dirty": dirty}))

    def stage(self, baseline_sha=SHA):
        ctx = stages.Ctx(REPO, None, Path("/x"), self.dir / "out", shell.Runner(), None, baseline_sha=baseline_sha)
        ctx.captures = {"baseline-full": self.before, "branch-full": self.after}
        return stages.stage_visual(ctx)

    def names(self, st, status=None):
        return [c.name for c in st.checks if status is None or c.status == status]


class VisualBaseline(Fixture):
    def test_captures_taken_at_the_rules_baseline_are_classified(self):
        self.provenance()
        st = self.stage()
        self.assertEqual(st.status, Status.PASS, [(c.name, c.detail) for c in st.checks if c.status == Status.FAIL])
        self.assertIn("zero unexpected changed pixels", self.names(st, Status.PASS))

    def test_branch_provenance_is_required_readable_clean_and_exactly_the_branch_role(self):
        self.provenance()
        path = self.after / provenance.PROVENANCE_FILE
        invalid = ((None, "the branch captures carry branch provenance", "carry no provenance.json"),
                   ("{not json", "the branch captures carry readable provenance", "not valid JSON"),
                   (json.dumps({"role": "branch", "commit": "abc", "dirty": False}),
                    "the branch captures carry readable provenance", "40-hex"),
                   (json.dumps({"role": "baseline", "commit": OTHER, "dirty": False}),
                    "the branch captures carry branch provenance", "'baseline'"),
                   (json.dumps({"commit": OTHER, "dirty": False}),
                    "the branch captures carry branch provenance", "None"),
                   (json.dumps({"role": "branch", "commit": OTHER, "dirty": True}),
                    "the branch captures come from a clean tree", "uncommitted changes"),
                   (json.dumps({"role": "branch", "commit": OTHER}),
                    "the branch captures come from a clean tree", "dirty"),
                   (json.dumps({"role": "branch", "commit": OTHER, "dirty": 0}),
                    "the branch captures come from a clean tree", "dirty"))
        for contents, name, detail in invalid:
            with self.subTest(contents=contents):
                if contents is None:
                    path.unlink()
                else:
                    path.write_text(contents)
                with mock.patch.object(stages.visual, "compare_dirs") as compare:
                    st = self.stage()
                self.assertEqual(st.status, Status.FAIL)
                failed = {c.name: c.detail for c in st.checks if c.status == Status.FAIL}
                self.assertIn(detail, failed[name])
                compare.assert_not_called()

    def test_unreadable_branch_provenance_is_a_named_failed_check(self):
        self.provenance()
        path = self.after / provenance.PROVENANCE_FILE
        path.unlink()
        path.mkdir()   # a directory cannot be read as the provenance file
        st = self.stage()
        self.assertIn("the branch captures carry readable provenance", self.names(st, Status.FAIL))

    def test_non_text_branch_provenance_is_a_named_failed_check(self):
        self.provenance()
        (self.after / provenance.PROVENANCE_FILE).write_bytes(b"\xff")
        st = self.stage()
        self.assertIn("the branch captures carry readable provenance", self.names(st, Status.FAIL))

    def test_captures_from_another_commit_fail_once_by_name_and_no_rule_is_applied(self):
        self.provenance(commit=OTHER)
        st = self.stage()
        failed = self.names(st, Status.FAIL)
        self.assertTrue(any("baseline mismatch" in c.detail for c in st.checks if c.status == Status.FAIL))
        self.assertIn("the baseline captures were taken at the rules' baseline commit", failed)
        self.assertIn("the visual rules were not applied", failed)
        self.assertFalse(any("every capture is either noise" in n or "zero unexpected" in n for n in self.names(st)), "no classification may have run")
        self.assertFalse(any("expected change is missing" in c.detail for c in st.checks))

    def test_a_moved_main_is_a_wrong_baseline_not_forty_missing_changes(self):
        # what a checkout of a later main would look like: the change already present on both sides
        self.provenance(commit="a" * 40)
        st = self.stage()
        self.assertEqual(st.status, Status.FAIL)
        self.assertEqual(len(self.names(st, Status.FAIL)), 2)      # the mismatch, and "not applied"

    def test_missing_provenance_is_a_precise_evidence_error(self):
        st = self.stage()
        (detail,) = [c.detail for c in st.checks if c.name == "the baseline captures carry provenance"]
        self.assertIn("carry no provenance.json", detail)
        self.assertIn(SHA[:7], detail)
        self.assertEqual(st.status, Status.FAIL)

    def test_malformed_provenance_is_an_evidence_error(self):
        (self.before / provenance.PROVENANCE_FILE).write_text("{not json")
        st = self.stage()
        self.assertIn("not valid JSON", [c.detail for c in st.checks if c.status == Status.FAIL][0])

    def test_a_dirty_baseline_tree_fails(self):
        self.provenance(dirty=True)
        self.assertIn("the baseline captures come from a clean tree", self.names(self.stage(), Status.FAIL))

    def test_the_captures_must_be_the_baselines(self):
        self.provenance(role="branch")
        self.assertIn("the captures are the baseline's", self.names(self.stage(), Status.FAIL))

    def test_the_rules_baseline_must_be_the_runs_baseline(self):
        self.provenance()
        st = self.stage(baseline_sha=OTHER)
        self.assertIn("the rules' baseline is this run's acceptance baseline", self.names(st, Status.FAIL))
        self.assertIn("the visual rules were not applied", self.names(st, Status.FAIL))

    def test_rules_that_name_no_baseline_cannot_be_applied(self):
        self.rules = rules_file(self.dir, sha=None)
        with mock.patch.object(stages, "VISUAL_RULES", self.rules):
            self.provenance()
            st = self.stage()
        self.assertIn("the rule set names the baseline it was derived against", self.names(st, Status.FAIL))

    def test_no_capture_directory_is_unavailable_not_a_pass(self):
        ctx = stages.Ctx(REPO, None, Path("/x"), self.dir / "out", shell.Runner(), None)
        st = stages.stage_visual(ctx)
        self.assertEqual([c.status for c in st.checks], [Status.UNAVAILABLE])


class CaptureOnlyVisualRun(Fixture):
    """`acceptance.py visual --baseline-captures A --branch-captures B` on a machine with a single checkout."""

    def run_cli(self, *extra):
        out = self.dir / "cli-out"

        def no_worktree(*a, **k):
            raise AssertionError("a capture-only visual run must not look for a baseline worktree")

        with restore_signal_handlers(), mock.patch.object(baseline, "find_baseline_tree", no_worktree), mock.patch.object(stages.session, "locked_since", lambda r, s: []), \
                redirect_stdout(io.StringIO()):
            code = acceptance.main(["visual", "--repo", str(REPO), "--baseline-captures", str(self.before), "--branch-captures", str(self.after),
                                    "--output", str(out), *extra])
        return code, json.loads((out / "manifest.json").read_text())

    def test_it_needs_no_baseline_worktree_when_the_captures_carry_their_provenance(self):
        self.provenance()
        code, manifest = self.run_cli()
        by_name = {s["name"]: s for s in manifest["stages"]}
        self.assertEqual(by_name["visual"]["status"], "PASS", by_name["visual"]["checks"])
        self.assertNotIn("guards", by_name)
        self.assertEqual(manifest["context"]["baseline_tree"], None)
        record = next(c for c in by_name["visual"]["checks"] if c["name"] == "the branch captures carry branch provenance")
        self.assertEqual(record["data"]["commit"], OTHER)
        self.assertIn(OTHER, record["detail"])
        self.assertIsNone(manifest["context"]["branch"])   # the evidence commit is not this checkout's branch

    def test_an_explicit_baseline_revision_is_checked_without_resolving_a_worktree(self):
        self.provenance()
        for revision, expected in ((SHA, "PASS"), (OTHER, "FAIL"), (SHA[:7], "FAIL"), ("main", "FAIL")):
            with self.subTest(revision=revision):
                code, manifest = self.run_cli("--baseline-sha", revision)
                visual_stage = next(s for s in manifest["stages"] if s["name"] == "visual")
                self.assertEqual(visual_stage["status"], expected)
                self.assertEqual(manifest["context"]["baseline_sha_expected"], revision)

    def test_signal_handlers_are_restored_after_success_and_a_failed_run(self):
        with restore_signal_handlers():
            def caller_handler(signum, frame):
                pass

            for sig in session.INTERRUPT_SIGNALS:
                signal.signal(sig, caller_handler)
            for valid in (True, False):
                with self.subTest(valid=valid):
                    if valid:
                        self.provenance()
                    else:
                        (self.before / provenance.PROVENANCE_FILE).unlink()
                    code, manifest = self.run_cli()
                    self.assertEqual(manifest["overall"], "PASS" if valid else "FAIL")
                    self.assertEqual(code, 0 if valid else 1)
                    for sig in session.INTERRUPT_SIGNALS:
                        self.assertIs(signal.getsignal(sig), caller_handler)

    def test_signal_handlers_are_restored_when_the_cli_raises(self):
        saved = {sig: signal.getsignal(sig) for sig in session.INTERRUPT_SIGNALS}
        for sig, handler in saved.items():
            self.addCleanup(signal.signal, sig, handler)

        def raises(*args):
            for sig in session.INTERRUPT_SIGNALS:
                signal.signal(sig, signal.SIG_IGN)
            raise RuntimeError("CLI failed outside its stage handler")

        with mock.patch.object(acceptance, "main", side_effect=raises):
            with self.assertRaisesRegex(RuntimeError, "CLI failed outside"):
                self.run_cli()
        for sig, handler in saved.items():
            self.assertEqual(signal.getsignal(sig), handler)

    def test_without_provenance_it_fails_with_the_evidence_error_not_a_missing_checkout(self):
        code, manifest = self.run_cli()
        self.assertEqual(code, 1)
        details = [c["detail"] for s in manifest["stages"] for c in s["checks"] if c["status"] == "FAIL"]
        self.assertTrue(any("carry no provenance.json" in d for d in details), details)

    def test_the_old_option_name_still_parses(self):
        self.assertEqual(acceptance.parse(["--main-captures", "/x", "visual"]).baseline_captures, Path("/x"))


class NativeWritesProvenance(unittest.TestCase):
    def test_the_commit_and_cleanliness_are_recorded_beside_the_captures(self):
        repo = Repo(self)
        out = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: __import__("shutil").rmtree(out, ignore_errors=True))
        ctx = stages.Ctx(repo.main, repo.main, Path("/x"), out, shell.Runner(), None)
        stages._record_provenance(ctx, repo.main, "baseline", out / "baseline-full")
        record = provenance.read(out / "baseline-full")
        self.assertEqual((record["commit"], record["dirty"], record["role"]), (repo.b, False, "baseline"))
        (repo.main / "b.txt").write_text("changed\n")
        stages._record_provenance(ctx, repo.main, "branch", out / "branch-full")
        self.assertTrue(provenance.read(out / "branch-full")["dirty"])


class DerivingRecordsTheBaseline(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)
        self.json = self.dir / "rules.json"
        self.json.write_text(json.dumps({"schema": 1, "noise_profiles": {}, "sets": {"baseline-vs-branch": {"captures": {}}}}))
        for name in ("a", "b"):
            (self.dir / name).mkdir()
        changed = canvas()
        paint(changed, (50, 60, 120, 63), (61, 174, 233))
        ppm(self.dir / "a" / "phase-row-catch.ppm", canvas())
        ppm(self.dir / "b" / "phase-row-catch.ppm", changed)

    def derive(self, *extra, name="baseline-vs-branch"):
        with redirect_stdout(io.StringIO()):
            derive_rules.main(["--before", str(self.dir / "a"), "--after", str(self.dir / "b"), "--set", name, "--note", "test",
                               "--json", str(self.json), *extra])
        return json.loads(self.json.read_text())["sets"][name]

    def test_the_baseline_comes_from_the_before_captures_own_provenance(self):
        provenance.write(self.dir / "a", "baseline", {"sha": SHA, "dirty": False, "branch": "(detached)"})
        self.assertEqual(self.derive()["baseline_sha"], SHA)
        self.assertEqual(self.derive("--baseline-sha", SHA)["baseline_sha"], SHA)

    def test_a_disagreeing_or_dirty_source_is_refused(self):
        provenance.write(self.dir / "a", "baseline", {"sha": SHA, "dirty": False})
        with self.assertRaises(SystemExit) as raised:
            self.derive("--baseline-sha", OTHER)
        self.assertIn("disagrees", str(raised.exception))
        provenance.write(self.dir / "a", "baseline", {"sha": SHA, "dirty": True})
        with self.assertRaises(SystemExit) as raised:
            self.derive()
        self.assertIn("dirty tree", str(raised.exception))

    def test_without_provenance_the_sha_is_given_explicitly_and_a_same_tree_set_names_none(self):
        self.assertEqual(self.derive("--baseline-sha", SHA)["baseline_sha"], SHA)
        self.assertNotIn("baseline_sha", self.derive(name="accent-vs-default"))
        with self.assertRaises(SystemExit):
            self.derive("--baseline-sha", "abc")

    def test_stamping_touches_only_the_metadata(self):
        before = self.derive(name="baseline-vs-branch")
        with redirect_stdout(io.StringIO()):
            derive_rules.main(["--set", "baseline-vs-branch", "--stamp-baseline", SHA, "--json", str(self.json)])
        after = json.loads(self.json.read_text())["sets"]["baseline-vs-branch"]
        self.assertEqual(after.pop("baseline_sha"), SHA)
        self.assertEqual(after, before)
        for bad in (("nope", SHA), ("baseline-vs-branch", "xyz")):
            with self.assertRaises(SystemExit):
                derive_rules.main(["--set", bad[0], "--stamp-baseline", bad[1], "--json", str(self.json)])


if __name__ == "__main__":
    unittest.main()
