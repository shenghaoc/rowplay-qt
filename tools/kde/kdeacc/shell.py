# SPDX-License-Identifier: GPL-3.0-or-later
"""Running commands: a clean host environment, the repository's Qt environment, and one log.

Two environments are never mixed (ADR 0018): host KDE tools run without any
of the bundled Qt's loader or plugin variables, and the app and its tests run
with the repository's `.envrc` (the aqt Qt 6.11). Every command is appended
to commands.log as one JSON line with its exit status and time.
"""

from __future__ import annotations

import json
import os
import shlex
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path

# Variables that point a process at a particular Qt or theme. Host tools must
# not see the bundled Qt's; the scrubbed generic runs must not see any desktop's.
QT_LOADER_VARS = (
    "LD_LIBRARY_PATH", "QML2_IMPORT_PATH", "QML_IMPORT_PATH", "QT_PLUGIN_PATH",
    "QT_QPA_PLATFORMTHEME", "QT_QPA_PLATFORM", "QT_STYLE_OVERRIDE", "QTDIR", "QMAKE",
    "QSG_RHI_BACKEND", "QT_QUICK_CONTROLS_STYLE", "QT_LOGGING_RULES",
    # Where Qt looks for platform plugins, styles and their configuration, and which backend it
    # draws with: any of these inherited from a developer shell that set up the bundled Qt
    # would make a host tool (plasmashell, kwin_wayland, the host's Qt) load the wrong plugins.
    "QT_QPA_PLATFORM_PLUGIN_PATH", "QT_QPA_GENERIC_PLUGINS", "QT_QUICK_CONTROLS_CONF",
    "QT_QUICK_CONTROLS_FALLBACK_STYLE", "QT_QUICK_BACKEND", "QT_FILE_SELECTORS", "QT_XCB_GL_INTEGRATION",
)
# Variables that make Qt pick a desktop's platform theme (qgenericunixtheme.cpp
# reads the desktop from these, XDG_CURRENT_DESKTOP first), or that describe a
# desktop session the generic path must not have.
DESKTOP_VARS = (
    "XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION", "KDE_FULL_SESSION",
    "KDE_SESSION_VERSION", "KDE_SESSION_UID", "KDE_APPLICATIONS_AS_SCOPE", "KDEDIRS", "KDEHOME",
    "GNOME_DESKTOP_SESSION_ID", "WAYLAND_DISPLAY", "XDG_SESSION_TYPE", "XDG_SESSION_CLASS",
    "QT_WAYLAND_RECONNECT",
)


def host_env(extra=None):
    """The environment for host tools: no Qt loader or plugin variables."""
    env = {k: v for k, v in os.environ.items() if k not in QT_LOADER_VARS}
    env.update(extra or {})
    return env


def bundled_qt_env(qt_dir, extra=None):
    """The environment for a process that runs the repository's bundled Qt.

    Starts from host_env (no inherited Qt variable at all) and adds only what that Qt needs: its
    own library directory. Callers add the platform, style and logging variables they mean."""
    env = host_env({"LD_LIBRARY_PATH": f"{qt_dir}/lib"})
    env.update(extra or {})
    return env


def generic_env(extra=None):
    """The environment for the generic Linux run: host_env without any desktop."""
    env = {k: v for k, v in host_env().items() if k not in DESKTOP_VARS}
    env.update(extra or {})
    return env


def scrubbed_vars(environ=None):
    """Which desktop variables a scrubbed environment had removed (for the record)."""
    environ = os.environ if environ is None else environ
    return sorted(k for k in DESKTOP_VARS if k in environ)


def text_of(value):
    """Captured output as text: subprocess reports it as bytes (or None) when a command times out."""
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value


@dataclass
class Done:
    argv: list
    rc: int
    out: str
    err: str
    seconds: float
    timed_out: bool = False

    @property
    def ok(self):
        return self.rc == 0

    @property
    def text(self):
        return self.out + self.err


class Runner:
    """Runs commands and logs them. dry_run records what would run and runs nothing."""

    def __init__(self, log_path=None, dry_run=False):
        self.log_path = Path(log_path) if log_path else None
        self.dry_run = dry_run

    def _log(self, entry):
        if self.log_path:
            with open(self.log_path, "a") as handle:
                handle.write(json.dumps(entry) + "\n")

    def run(self, argv, cwd=None, env=None, timeout=None, tag="", stdin=None):
        argv = [str(a) for a in argv]
        started = time.time()
        if self.dry_run:
            self._log({"tag": tag, "argv": argv, "cwd": str(cwd) if cwd else None, "dry_run": True})
            return Done(argv, 0, "", "", 0.0)
        try:
            proc = subprocess.run(
                argv, cwd=cwd, env=env if env is not None else host_env(), input=stdin,
                capture_output=True, text=True, timeout=timeout, errors="replace",
            )
            done = Done(argv, proc.returncode, proc.stdout, proc.stderr, time.time() - started)
        except subprocess.TimeoutExpired as exc:
            done = Done(argv, 124, text_of(exc.stdout), text_of(exc.stderr) + f"\ntimeout after {timeout}s", time.time() - started,
                        timed_out=True)
        except FileNotFoundError as exc:
            done = Done(argv, 127, "", str(exc), time.time() - started)
        self._log({"tag": tag, "argv": argv, "cwd": str(cwd) if cwd else None,
                   "rc": done.rc, "seconds": round(done.seconds, 2), **({"timed_out": True} if done.timed_out else {})})
        return done

    def in_tree(self, tree, argv, env=None, timeout=None, tag="", logfile=None):
        """Run argv in a worktree with its .envrc (the repository's Qt) sourced."""
        script = 'cd "$1" && shift && source .envrc && exec "$@"'
        merged = os.environ.copy()
        for key, value in (env or {}).items():
            if value is None:
                merged.pop(key, None)   # None removes a variable (LIBGL_ALWAYS_SOFTWARE for a native run)
            else:
                merged[key] = value
        done = self.run(["bash", "-c", script, "_", str(tree), *argv], env=merged, timeout=timeout, tag=tag)
        if logfile:
            Path(logfile).write_text(f"$ {shlex.join(map(str, argv))}\n(exit {done.rc}, {done.seconds:.1f}s)\n\n{done.out}\n{done.err}")
        return done
