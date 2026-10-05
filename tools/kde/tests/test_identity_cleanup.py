# SPDX-License-Identifier: GPL-3.0-or-later
"""Owned-PID shutdown, including a TERM-resistant process, without a Plasma session."""

import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.dont_write_bytecode = True

from kdeacc import shell, stages  # noqa: E402
from kdeacc.results import Status  # noqa: E402

REPO = Path(__file__).resolve().parents[3]


class Recorder(shell.Runner):
    def __init__(self):
        super().__init__()
        self.calls = []

    def run(self, argv, cwd=None, env=None, timeout=None, tag="", stdin=None):
        self.calls.append([str(arg) for arg in argv])
        out = '{"steps": [], "error": null}' if tag == "atspi" else ""
        return shell.Done(argv, 0, out, "", 0.0)


class OwnedProcessCleanup(unittest.TestCase):
    def process(self, resist_term=False):
        code = ("import signal, time; "
                + ("signal.signal(signal.SIGTERM, signal.SIG_IGN); " if resist_term else "")
                + "print('ready', flush=True); time.sleep(60)")
        process = subprocess.Popen([sys.executable, "-c", code], stdout=subprocess.PIPE, text=True)

        def cleanup():
            if process.poll() is None:
                process.kill()
            process.wait(timeout=5)
            process.stdout.close()

        self.addCleanup(cleanup)
        self.assertEqual(process.stdout.readline().strip(), "ready")
        return process

    def test_cleanup_waits_for_asynchronous_kill_delivery(self):
        ctx = SimpleNamespace(launched_pids={12345})
        clock = {"now": 0.0, "killed": None}

        def alive(pids):
            return sorted(pids) if clock["killed"] is None or clock["now"] < clock["killed"] + 0.15 else []

        def kill(pid, signum):
            self.assertIn(pid, ctx.launched_pids)
            if signum == signal.SIGKILL:
                clock["killed"] = clock["now"]

        with mock.patch.object(stages, "alive_pids", side_effect=alive), \
                mock.patch.object(stages.os, "kill", side_effect=kill), \
                mock.patch.object(stages.time, "monotonic", side_effect=lambda: clock["now"]), \
                mock.patch.object(stages.time, "sleep", side_effect=lambda delay: clock.update(now=clock["now"] + delay)):
            ended = stages.end_owned_processes(ctx, grace=0.3)
        self.assertEqual(ended, [12345])
        self.assertIsNotNone(clock["killed"])
        self.assertGreaterEqual(clock["now"], clock["killed"] + 0.15)
        self.assertEqual(alive(ctx.launched_pids), [])

    def test_cleanup_escalates_and_waits_only_for_owned_pids(self):
        resistant = self.process(resist_term=True)
        cooperative = self.process()
        unrelated = self.process(resist_term=True)
        ctx = SimpleNamespace(launched_pids={resistant.pid, cooperative.pid})
        with mock.patch.object(stages.os, "kill", wraps=os.kill) as kill:
            ended = stages.end_owned_processes(ctx, grace=0.05)
        self.assertEqual(ended, sorted(ctx.launched_pids))
        self.assertEqual(stages.alive_pids(ctx.launched_pids), [])
        self.assertEqual(resistant.wait(timeout=5), -signal.SIGKILL)
        self.assertEqual(cooperative.wait(timeout=5), -signal.SIGTERM)
        self.assertIsNone(unrelated.poll())
        signalled = [(call.args[0], call.args[1]) for call in kill.call_args_list if call.args[1]]
        self.assertEqual({pid for pid, _ in signalled}, ctx.launched_pids)
        self.assertIn((resistant.pid, signal.SIGKILL), signalled)
        self.assertNotIn((cooperative.pid, signal.SIGKILL), signalled)

    def test_exited_child_is_not_a_running_leftover_before_reaping(self):
        process = self.process()
        process.kill()
        os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOWAIT)
        self.assertEqual(stages.alive_pids({process.pid}), [])
        self.assertEqual(stages.end_owned_processes(SimpleNamespace(launched_pids={process.pid}), grace=0.05), [])
        self.assertEqual(process.wait(timeout=5), -signal.SIGKILL)

    def test_identity_uses_owned_cleanup_before_the_final_leftovers_check(self):
        resistant = self.process(resist_term=True)
        cooperative = self.process()
        unrelated = self.process(resist_term=True)
        with tempfile.TemporaryDirectory() as temp:
            image = Path(temp) / "rowplay.AppImage"
            image.write_text("")
            runner = Recorder()
            ctx = stages.Ctx(REPO, None, Path("/unused-qt"), Path(temp), runner,
                             SimpleNamespace(allow_session_changes=True))
            ctx.appimages["branch"] = image
            app_id = ctx.app_id()

            def window(pid):
                return {"pid": pid, "desktopFileName": app_id, "resourceClass": "rowplay-qt",
                        "resourceName": "rowplay-qt", "wayland": True}

            entry = mock.MagicMock()
            entry.__enter__.return_value = entry
            entry.restored.return_value = True
            cleanup = stages.end_owned_processes
            sleep = time.sleep
            survey = {"synthesizer_tools_installed": [], "uinput_writable_without_privilege": False}
            with mock.patch.object(stages, "DesktopEntry", return_value=entry), \
                    mock.patch.object(stages, "key_injection_survey", return_value=survey), \
                    mock.patch.object(stages.kwin, "rowplay_windows", return_value=[]), \
                    mock.patch.object(stages.kwin, "wait_for_windows", side_effect=[
                        [window(resistant.pid)], [window(resistant.pid), window(cooperative.pid)], []]), \
                    mock.patch.object(stages.kwin, "close_rowplay_windows"), \
                    mock.patch.object(stages, "pinned", return_value=False), \
                    mock.patch.object(stages.time, "sleep", side_effect=lambda seconds: sleep(min(seconds, 0.01))), \
                    mock.patch.object(stages, "end_owned_processes", side_effect=lambda context: cleanup(context, grace=0.05)) as end:
                result = stages.stage_identity(ctx)
                end.assert_called_once_with(ctx)
                leftovers = stages.stage_leftovers(ctx)

            self.assertEqual(ctx.launched_pids, {resistant.pid, cooperative.pid})
            self.assertEqual(stages.alive_pids(ctx.launched_pids), [])
            self.assertEqual(resistant.wait(timeout=5), -signal.SIGKILL)
            self.assertEqual(cooperative.wait(timeout=5), -signal.SIGTERM)
            self.assertIsNone(unrelated.poll(), "an unrelated AppImage-like process must remain running")
            shutdown = next(check for check in result.checks if check.name.startswith("no RowPlay process this run launched"))
            self.assertEqual(shutdown.status, Status.FAIL, "requiring forced cleanup is still a shutdown failure")
            self.assertIn("still running: []", shutdown.detail)
            self.assertEqual(leftovers.status, Status.PASS)
            self.assertFalse(any(command[0] in ("kill", "pkill", "killall") for command in runner.calls))


if __name__ == "__main__":
    unittest.main()
