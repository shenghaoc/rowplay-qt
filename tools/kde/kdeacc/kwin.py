# SPDX-License-Identifier: GPL-3.0-or-later
"""Asking KWin which RowPlay windows exist, and closing them, through its scripting interface.

KWin runs a small script (loaded over D-Bus, unloaded straight after); the
script `console.info`s one marked line per window, which is read back from the
user journal. It is how the harness sees `desktopFileName`, the property KWin's
task manager groups windows by. Host tools only: run in the clean host environment.
"""

from __future__ import annotations

import json
import re
import tempfile
import time
import uuid
from pathlib import Path

from .shell import host_env

# What a RowPlay window calls itself: the desktop entry ID the app names at start-up (ADR 0018), and the
# executable names an unpackaged or older build reports in its place. A window is RowPlay's only when one of
# KWin's own identity properties equals one of these exactly. Never the caption, never a substring: a browser
# tab titled "...shenghaoc/rowplay-qt" (a person reviewing these very pull requests) is not RowPlay, and the
# first version of this finder took it for one, which blocked the identity stage and had the raiser activate
# the tab. Another application whose name merely contains "rowplay" is not RowPlay either.
ROWPLAY_IDENTITIES = ("io.github.shenghaoc.rowplay", "rowplay-qt", "rowplay-app")
IDENTITY_PROPERTIES = ("desktopFileName", "resourceClass", "resourceName")


def is_rowplay_window(window):
    """The rule the KWin scripts apply, for a parsed window dict (or any mapping with KWin's property names)."""
    return any(window.get(prop) in ROWPLAY_IDENTITIES for prop in IDENTITY_PROPERTIES)


# The same rule in KWin's JavaScript, defined once and put in front of every script that selects windows.
_IS_ROWPLAY_JS = r"""
const ROWPLAY_IDS = @IDS@;
function isRow(w) {
    return ROWPLAY_IDS.indexOf(w.desktopFileName) >= 0 || ROWPLAY_IDS.indexOf(w.resourceClass) >= 0 ||
           ROWPLAY_IDS.indexOf(w.resourceName) >= 0;
}
"""


def render(js, marker):
    """A KWin script ready to load: its marker and the identity list filled in."""
    return js.replace("@MARK@", marker).replace("@IDS@", json.dumps(list(ROWPLAY_IDENTITIES)))


LIST_JS = _IS_ROWPLAY_JS + r"""
for (const w of workspace.windowList()) if (isRow(w)) {
    console.info("@MARK@ caption=" + JSON.stringify(w.caption) + " resourceClass=" + JSON.stringify(w.resourceClass) +
        " resourceName=" + JSON.stringify(w.resourceName) + " desktopFileName=" + JSON.stringify(w.desktopFileName) +
        " pid=" + w.pid + " wayland=" + (!w.x11Client) + " width=" + Math.round(w.width) + " height=" + Math.round(w.height));
}
console.info("@MARK@ end");
"""

CLOSE_JS = _IS_ROWPLAY_JS + r"""
for (const w of workspace.windowList()) if (isRow(w)) w.closeWindow();
console.info("@MARK@ end");
"""

LINE_RE = re.compile(r'caption=(?P<caption>".*?") resourceClass=(?P<resourceClass>".*?") resourceName=(?P<resourceName>".*?") '
                     r'desktopFileName=(?P<desktopFileName>".*?") pid=(?P<pid>\d+) wayland=(?P<wayland>\w+) '
                     r'width=(?P<width>\d+) height=(?P<height>\d+)')


def parse_windows(text, marker):
    """[{caption, resourceClass, resourceName, desktopFileName, pid, wayland, width, height}] from journal text."""
    import json
    out = []
    for line in text.splitlines():
        if marker not in line:
            continue
        match = LINE_RE.search(line.split(marker, 1)[1])
        if match:
            d = match.groupdict()
            for key in ("caption", "resourceClass", "resourceName", "desktopFileName"):
                d[key] = json.loads(d[key])
            d["pid"], d["width"], d["height"] = int(d["pid"]), int(d["width"]), int(d["height"])
            d["wayland"] = d["wayland"] == "true"
            out.append(d)
    return out


def _qdbus(runner, *args, tag="kwin"):
    return runner.run(["qdbus-qt6", *args], env=host_env(), tag=tag, timeout=20)


def run_script(runner, js, wait=1.5, tag="kwin-script"):
    """Load, run and unload a KWin script; returns (marker, journal text since it ran)."""
    marker = "ROWPLAY-KWIN-" + uuid.uuid4().hex[:10]
    name = "kdeacc-" + marker
    with tempfile.NamedTemporaryFile("w", suffix=".js", delete=False) as handle:
        handle.write(render(js, marker))
        path = handle.name
    since = time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(time.time() - 1))
    try:
        loaded = _qdbus(runner, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.loadScript", path, name, tag=tag)
        script_id = loaded.out.strip()
        if not loaded.ok or not script_id.isdigit():
            return marker, ""
        _qdbus(runner, "org.kde.KWin", f"/Scripting/Script{script_id}", "org.kde.kwin.Script.run", tag=tag)
        deadline = time.time() + wait + 5
        text = ""
        while time.time() < deadline:
            time.sleep(0.4)
            text = runner.run(["journalctl", "--user", "--since", since, "--no-pager", "-o", "cat"], env=host_env(), tag=tag).out
            if f"{marker} end" in text:
                break
        return marker, text
    finally:
        _qdbus(runner, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.unloadScript", name, tag=tag)
        Path(path).unlink(missing_ok=True)


def rowplay_windows(runner):
    """The RowPlay windows KWin has now, or None if KWin could not be asked."""
    marker, text = run_script(runner, LIST_JS, tag="kwin-list")
    if f"{marker} end" not in text:
        return None
    return [w for w in parse_windows(text, marker) if is_rowplay_window(w)]  # the script already filtered; the rule is checked twice


def close_rowplay_windows(runner):
    run_script(runner, CLOSE_JS, tag="kwin-close")


def wait_for_windows(runner, count, timeout=45):
    """Poll until exactly `count` RowPlay windows exist; returns the last list seen."""
    deadline = time.time() + timeout
    last = None
    while time.time() < deadline:
        last = rowplay_windows(runner)
        if last is not None and len(last) == count:
            return last
        time.sleep(1.0)
    return last


# A gate window that KWin maps behind another window (focus-stealing prevention) gets no frame
# callbacks and no keyboard focus: every hold in the walk then runs out its bound and every key
# check fails, whatever the app does. While a native stage runs, this script raises each RowPlay
# window as it appears, so the walk does not depend on what the desktop's user happens to be doing.
RAISE_JS = _IS_ROWPLAY_JS + r"""
function raise(w) { if (isRow(w) && workspace.activeWindow !== w) { w.minimized = false; workspace.activeWindow = w; } }
workspace.windowAdded.connect(function (w) {
    raise(w);
    w.desktopFileNameChanged.connect(function () { raise(w); });
});
for (const w of workspace.windowList()) raise(w);
console.info("@MARK@ end");
"""


def load_persistent(runner, js, tag="kwin-persist"):
    """Load and run a KWin script and leave it loaded; returns its name for unload(). None if it did not load."""
    marker = "ROWPLAY-KWIN-" + uuid.uuid4().hex[:10]
    name = "kdeacc-" + marker
    with tempfile.NamedTemporaryFile("w", suffix=".js", delete=False) as handle:
        handle.write(render(js, marker))
        path = handle.name
    loaded = _qdbus(runner, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.loadScript", path, name, tag=tag)
    script_id = loaded.out.strip()
    if not loaded.ok or not script_id.isdigit():
        Path(path).unlink(missing_ok=True)
        return None
    _qdbus(runner, "org.kde.KWin", f"/Scripting/Script{script_id}", "org.kde.kwin.Script.run", tag=tag)
    Path(path).unlink(missing_ok=True)
    return name


def unload(runner, name, tag="kwin-persist"):
    if name:
        _qdbus(runner, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.unloadScript", name, tag=tag)


class Raiser:
    """Keeps RowPlay windows frontmost for a block; always unloads. `active` says whether it loaded."""

    def __init__(self, runner):
        self.runner, self.name = runner, None

    def __enter__(self):
        self.name = load_persistent(self.runner, RAISE_JS, tag="kwin-raiser")
        return self

    @property
    def active(self):
        return self.name is not None

    def __exit__(self, *exc):
        unload(self.runner, self.name, tag="kwin-raiser")
        self.name = None
        return False
