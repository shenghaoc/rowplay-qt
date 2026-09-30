# SPDX-License-Identifier: GPL-3.0-or-later
"""Keeping the screen awake for a run, and telling whether it locked anyway.

Nobody touches the machine while the gates run, so Plasma's screen locker engages after its idle
timeout (five minutes by default), and a locked session stops sending frame callbacks to every window:
the walk's holds run out their bound and the run says nothing about the app (the same effect AGENTS.md
records for a locked Mac). The sanctioned, unprivileged answer is what a video player does:
`org.freedesktop.ScreenSaver.Inhibit`, held on a D-Bus connection for the run and released after it.
Whether it worked is *measured*, not assumed: `locked_since` reads the user journal for the screen
locker's greeter starting after the run began.
"""

from __future__ import annotations

from .shell import host_env

DEST = ("org.freedesktop.ScreenSaver", "/ScreenSaver", "org.freedesktop.ScreenSaver")


class IdleInhibitor:
    """Holds an idle/lock inhibition for the block. `cookie` is None if it could not be taken."""

    def __init__(self, reason="native gate runs need the screen awake"):
        self.reason = reason
        self.cookie = None
        self.note = ""
        self._bus = None

    def _call(self, method, args=None, reply=None):
        from gi.repository import Gio, GLib
        return self._bus.call_sync(*DEST[:3], method, args, GLib.VariantType(reply) if reply else None,
                                   Gio.DBusCallFlags.NONE, 5000, None)

    def __enter__(self):
        try:
            from gi.repository import Gio, GLib
            self._bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            self.cookie = self._call("Inhibit", GLib.Variant("(ss)", ("rowplay-kde-acceptance", self.reason)), "(u)").unpack()[0]
        except Exception as exc:  # no gi, no session bus, no ScreenSaver service: say so, do not pretend
            self.cookie, self.note = None, f"{type(exc).__name__}: {exc}"
        return self

    @property
    def active(self):
        return self.cookie is not None

    def __exit__(self, *exc):
        if self.cookie is not None:
            try:
                from gi.repository import GLib
                self._call("UnInhibit", GLib.Variant("(u)", (self.cookie,)), "()")
            except Exception:
                pass
            self.cookie = None
        return False


def locked_since(runner, since_epoch):
    """The screen locker's journal lines since since_epoch (seconds): any means the screen locked during the run."""
    import time
    since = time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(since_epoch))
    out = runner.run(["journalctl", "--user", "--since", since, "--no-pager", "-o", "short-iso", "_COMM=kscreenlocker_g"],
                     env=host_env(), tag="lock-check").out
    return [ln for ln in out.splitlines() if "kscreenlocker" in ln and not ln.startswith("--")]
