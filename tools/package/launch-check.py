#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Launch a packaged rowplay-qt binary from a clean environment; require a clean exit.

The release bundles carry none of the debug-only gate hooks, so this is the
one check every package script runs on its output: start the *deployed*
binary with `ROWPLAY_EXIT_AFTER_FRAMES` (the shell renders that many frames
and quits — by then every Qt framework, platform plugin and QML module the
bundle must carry has been loaded), from an environment stripped of every
loader and Qt variable, and fail on a non-zero exit, a timeout, or a loader /
QML failure signature on stderr.

It is deliberately blunt about the environment: `PATH` is cut to the system
directories, and `DYLD_*`, `LD_LIBRARY_PATH`, `QT_*`, `QML*` and `QSG_*` are
dropped, so a bundle that only works because the build machine's Qt install
is reachable fails here. (A macOS binary whose `LC_RPATH` still points into
the Qt install is caught separately by `macos.sh`'s otool scan — dyld does not
consult the environment for that.)

Usage:
    launch-check.py BINARY [--platform NAME] [--frames N] [--timeout SECONDS]
                    [--env KEY=VALUE ...] [--cwd DIR]

`--env` passes through the few variables a platform needs to draw at all
(`DISPLAY` under Xvfb, `QSG_RHI_BACKEND` / `LIBGL_ALWAYS_SOFTWARE` for Mesa,
`APPIMAGE_EXTRACT_AND_RUN` for AppImages on FUSE-less runners).
"""

import argparse
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# Any of these on stderr means the bundle is incomplete or the shell failed
# to load, whatever the exit status says.
FAILURE_SIGNATURES = (
    "Library not loaded",             # macOS dyld: a framework is missing
    "error while loading shared libraries",  # glibc ld.so
    "could not be located in the dynamic link library",  # Windows loader
    "is not installed",               # QML: module "X" is not installed
    "is not a type",                  # QML: a module loaded without its plugin
    "QQmlApplicationEngine failed to load component",
    "Could not find the Qt platform plugin",
    "This application failed to start",
    "ReferenceError",
    "TypeError",
)

# Kept from the caller's environment: enough to find the home directory and a
# window system, nothing that could point the loader at a Qt install.
KEEP = (
    "HOME", "USER", "LOGNAME", "LANG", "LC_ALL", "TMPDIR", "TEMP", "TMP",
    # Windows: the loader needs SystemRoot; the CRT reads these.
    "SystemRoot", "SystemDrive", "USERPROFILE", "APPDATA", "LOCALAPPDATA",
    "ProgramData", "COMSPEC", "PATHEXT", "windir",
    # Linux window system and session bus (keyring's Secret Service).
    "DISPLAY", "XAUTHORITY", "WAYLAND_DISPLAY", "XDG_RUNTIME_DIR",
    "XDG_SESSION_TYPE", "DBUS_SESSION_BUS_ADDRESS",
)


def system_path() -> str:
    if os.name == "nt":
        root = os.environ.get("SystemRoot", r"C:\Windows")
        return os.pathsep.join([os.path.join(root, "System32"), root])
    return "/usr/bin:/bin:/usr/sbin:/sbin"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("binary", type=Path)
    parser.add_argument("--platform", help="QT_QPA_PLATFORM to request (default: the platform's own)")
    parser.add_argument("--frames", type=int, default=30)
    # A healthy bundle exits in a few seconds even under software GL; a QML load
    # failure never exits at all (docs/qt-bridges-notes.md #16), so the
    # timeout is the assertion that catches it — keep it short.
    parser.add_argument("--timeout", type=float, default=60.0)
    parser.add_argument("--env", action="append", default=[], metavar="KEY=VALUE")
    parser.add_argument("--cwd", type=Path)
    args = parser.parse_args()

    if not args.binary.exists():
        print(f"launch-check: {args.binary} does not exist", file=sys.stderr)
        return 2

    env = {key: os.environ[key] for key in KEEP if key in os.environ}
    env["PATH"] = system_path()
    env["ROWPLAY_EXIT_AFTER_FRAMES"] = str(args.frames)
    # Qt on Windows routes qDebug/qWarning to OutputDebugString when stderr
    # is a pipe; the signature scan needs them on stderr everywhere.
    env["QT_FORCE_STDERR_LOGGING"] = "1"
    if args.platform:
        env["QT_QPA_PLATFORM"] = args.platform
    for item in args.env:
        key, sep, value = item.partition("=")
        if not sep:
            parser.error(f"--env expects KEY=VALUE, got {item!r}")
        env[key] = value

    with tempfile.TemporaryDirectory(prefix="rowplay-launch-") as temp:
        # Hermetic: the probe must never touch a real logbook cache.
        env["ROWPLAY_DATA_DIR"] = temp
        started = time.monotonic()
        try:
            completed = subprocess.run(
                [str(args.binary.resolve())],
                cwd=str(args.cwd) if args.cwd else temp,
                env=env,
                capture_output=True,
                text=True,
                errors="replace",
                timeout=args.timeout,
            )
        except subprocess.TimeoutExpired as expired:
            stderr = (expired.stderr or b"")
            if isinstance(stderr, bytes):
                stderr = stderr.decode("utf-8", "replace")
            print(f"launch-check: FAIL — no exit after {args.timeout:.0f} s", file=sys.stderr)
            print(stderr[-4000:], file=sys.stderr)
            return 1
        elapsed = time.monotonic() - started

    output = completed.stdout + completed.stderr
    hits = [line for line in output.splitlines() if any(sig in line for sig in FAILURE_SIGNATURES)]
    if completed.returncode != 0 or hits:
        print(
            f"launch-check: FAIL — exit {completed.returncode} after {elapsed:.1f} s, "
            f"{len(hits)} failure signature(s)",
            file=sys.stderr,
        )
        for line in hits:
            print(f"  {line}", file=sys.stderr)
        print("--- stderr (tail) ---", file=sys.stderr)
        print(completed.stderr[-4000:], file=sys.stderr)
        return 1
    print(
        f"launch-check: ok — {args.binary.name} rendered {args.frames} frames and exited 0 "
        f"in {elapsed:.1f} s (platform {env.get('QT_QPA_PLATFORM', 'default')})"
    )
    # The stderr tail is worth having in a CI log even on success: a font
    # alias warning is noise, a "Quick 3D is not functional" line under an
    # offscreen platform is expected, anything else is a lead.
    tail = completed.stderr.strip().splitlines()[-6:]
    for line in tail:
        print(f"  | {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
