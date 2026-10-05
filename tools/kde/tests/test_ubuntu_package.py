# SPDX-License-Identifier: GPL-3.0-or-later
"""The configured Qt reaches both package builds, including paths with spaces."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.dont_write_bytecode = True

import acceptance  # noqa: E402
from kdeacc import package, shell, stages  # noqa: E402
from kdeacc.results import Status  # noqa: E402

TOOLS = Path(__file__).resolve().parents[1]
APP_ID = "io.github.shenghaoc.rowplay"


def executable(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(0o755)


class UbuntuPackageWrapper(unittest.TestCase):
    """Replay the container shell with fake podman, apt, Rust and packaging tools."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="kde package ")
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.home = self.root / "home user"
        self.src = self.root / "source tree"
        self.target = self.root / "container target"
        self.log = self.root / "package log.txt"
        self.bin = self.root / "fake bin"
        self.capture = self.root / "podman.json"
        self.env_capture = self.root / "package-env.json"
        self.qmake_calls = self.root / "qmake-calls.txt"
        self.env = {**os.environ, "HOME": str(self.home), "PATH": f"{self.bin}:/usr/bin:/bin",
                    "FAKE_PODMAN_CAPTURE": str(self.capture), "FAKE_PACKAGE_ENV": str(self.env_capture),
                    "FAKE_QMAKE_CALLS": str(self.qmake_calls), "FAKE_GITDIR": str(self.root / "common git"),
                    "DISPLAY": ":bad", "WAYLAND_DISPLAY": "bad-wayland"}
        fake_git = '#!/bin/sh\nif [ "$1" = -C ]; then printf "%s\\n" "$FAKE_GITDIR"; fi\n'
        executable(self.bin / "git", fake_git)
        executable(self.home / ".cargo/bin/git", fake_git)
        executable(self.home / ".cargo/bin/rustc", '#!/bin/sh\necho "rustc fixture"\n')
        executable(self.bin / "apt-get", '#!/bin/sh\nexit 0\n')
        executable(self.bin / "podman", '''#!/usr/bin/python3
import json, os, subprocess, sys
from pathlib import Path
args = sys.argv[1:]
Path(os.environ["FAKE_PODMAN_CAPTURE"]).write_text(json.dumps(args))
env = dict(os.environ)
mounts = []
for i, value in enumerate(args[:-1]):
    if value == "-e":
        key, value = args[i + 1].split("=", 1)
        env[key] = value
    elif value == "-v":
        mounts.append(args[i + 1])
src = next(value[:-5] for value in mounts if value.endswith(":/src"))
command = args[args.index("bash") + 2]
sys.exit(subprocess.run(["/bin/bash", "-c", command], cwd=src, env=env).returncode)
''')
        executable(self.src / "tools/package/linux.sh", '''#!/usr/bin/python3
import json, os
from pathlib import Path
keys = ("HOME", "PATH", "QMAKE", "QTDIR", "LD_LIBRARY_PATH", "DISPLAY", "WAYLAND_DISPLAY")
Path(os.environ["FAKE_PACKAGE_ENV"]).write_text(json.dumps({key: os.environ.get(key) for key in keys}))
print("launch-check: ok - fixture")
''')

    def invoke(self, qt=None):
        args = ["bash", str(TOOLS / "ubuntu-package.sh"), str(self.src), str(self.target), str(self.log)]
        if qt is not None:
            args.append(str(qt))
        return subprocess.run(args, env=self.env, capture_output=True, text=True, timeout=10)

    def qt(self, relative):
        qt = self.root / relative
        executable(qt / "bin/qmake", '#!/bin/sh\nprintf "%s\\n" "$0" >> "$FAKE_QMAKE_CALLS"\necho 6.fixture\n')
        (qt / "lib").mkdir()
        return qt

    def assert_configured_qt(self, qt):
        done = self.invoke(qt)
        self.assertEqual(done.returncode, 0, done.stderr + (self.log.read_text() if self.log.exists() else ""))
        args = json.loads(self.capture.read_text())
        mounts = [args[i + 1] for i, arg in enumerate(args[:-1]) if arg == "-v"]
        self.assertIn(f"{qt}:{qt}:ro", mounts)
        self.assertNotIn(f"{self.home}/Qt:{self.home}/Qt:ro", mounts)
        self.assertIn(f"{self.src}:/src", mounts)
        self.assertIn(f"{self.target}:/src/target", mounts)
        self.assertIn(f"QTDIR={qt}", args)
        self.assertIn(f"HOME={self.home}", args)
        env = json.loads(self.env_capture.read_text())
        self.assertEqual(env["QMAKE"], str(qt / "bin/qmake"))
        self.assertEqual(env["QTDIR"], str(qt))
        self.assertEqual(env["LD_LIBRARY_PATH"], str(qt / "lib"))
        self.assertEqual(env["PATH"], f"{self.home}/.cargo/bin:{qt}/bin:/usr/bin:/bin")
        self.assertEqual(env["HOME"], str(self.home))
        self.assertIsNone(env["DISPLAY"])
        self.assertIsNone(env["WAYLAND_DISPLAY"])
        self.assertEqual(self.qmake_calls.read_text().splitlines()[-1], str(qt / "bin/qmake"))
        self.assertIn("launch-check: ok - fixture", self.log.read_text())

    def test_default_and_external_qt_mounts_and_environment_survive_spaces(self):
        for qt in (self.qt("home user/Qt/6.11.2/gcc_64"), self.qt("external Qt/6.custom/gcc_64")):
            with self.subTest(qt=qt):
                self.assert_configured_qt(qt)

    def test_qt_path_shell_metacharacters_are_literal_data(self):
        self.assert_configured_qt(self.qt("external Qt 'quoted' $HOME $(false)/gcc_64"))

    def test_missing_qt_argument_fails_before_starting_a_container(self):
        done = self.invoke()
        self.assertNotEqual(done.returncode, 0)
        self.assertFalse(self.capture.exists())


class PackageStageQt(unittest.TestCase):
    """Exercise the real stage and build helper through a fake package script."""

    def run_stage(self, configured_qt):
        tmp = tempfile.TemporaryDirectory(prefix="kde stage ")
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        home, branch, baseline = (root / name for name in ("home user", "branch tree", "baseline tree"))
        out, tools, capture = root / "evidence dir", root / "tools dir", root / "builds.jsonl"
        for path in (home, branch, baseline):
            path.mkdir()
        executable(tools / "ubuntu-package.sh", f'''#!{sys.executable}
import json, sys
from pathlib import Path
src, target, log, qt = sys.argv[1:]
with Path({str(capture)!r}).open("a") as handle:
    handle.write(json.dumps(sys.argv[1:]) + "\\n")
dist = Path(src) / "dist"
dist.mkdir(parents=True)
(dist / "fixture.AppImage").write_bytes(b"package fixture")
Path(log).write_text("launch-check: ok - fixture\\n")
''')
        with mock.patch.object(Path, "home", return_value=home):
            args = acceptance.parse(["package"] + (["--qt-dir", str(configured_qt)] if configured_qt is not None else []))
        ctx = stages.Ctx(branch, baseline, args.qt_dir, out, shell.Runner(), args, baseline_validated=True)
        ctx.app_id = lambda: APP_ID

        def extract(runner, appimage, dest):
            appdir = Path(dest) / "squashfs-root"
            desktop = appdir / "usr/share/applications" / f"{APP_ID}.desktop"
            meta = appdir / "usr/share/metainfo" / f"{APP_ID}.metainfo.xml"
            desktop.parent.mkdir(parents=True)
            meta.parent.mkdir(parents=True)
            desktop.write_text("[Desktop Entry]\n")
            meta.write_text(f"<id>{APP_ID}</id>")
            return appdir

        real_run = ctx.runner.run

        def run(argv, **kw):
            if kw.get("tag") == "desktop-validate":
                return shell.Done(argv, 0, "", "", 0.0)
            return real_run(argv, **kw)

        with mock.patch.object(Path, "home", return_value=home), \
                mock.patch.object(stages, "TOOLS", tools), \
                mock.patch.object(stages.shutil, "which", side_effect=lambda name: "/fake/podman" if name == "podman" else None), \
                mock.patch.object(stages, "git_state", return_value={"sha": "a" * 40}), \
                mock.patch.object(package, "extract", side_effect=extract), \
                mock.patch.object(ctx.runner, "run", side_effect=run):
            result = stages.stage_package(ctx)
        self.assertEqual(result.status, Status.PASS, [(check.name, check.detail) for check in result.checks])
        builds = [json.loads(line) for line in capture.read_text().splitlines()]
        self.assertEqual(len(builds), 2)
        self.assertEqual([build[0] for build in builds], [str(baseline), str(branch)])
        self.assertEqual([build[3] for build in builds], [str(args.qt_dir.resolve())] * 2)
        self.assertNotEqual(builds[0][1], builds[1][1])
        self.assertEqual(set(ctx.appimages), {"baseline", "branch"})
        self.assertTrue((out / "package/comparison.md").exists())

    def test_cli_default_qt_is_identical_for_baseline_and_branch(self):
        self.run_stage(None)

    def test_relative_configured_qt_is_resolved_for_both_builds(self):
        self.run_stage(Path("relative Qt") / "sdk" / ".." / "gcc_64")


if __name__ == "__main__":
    unittest.main()
