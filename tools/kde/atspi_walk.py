#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Drive the exact packaged AppImage through AT-SPI, the accessibility bus.

Wayland gives no unprivileged key synthesis (that needs the RemoteDesktop
portal's consent dialog, /dev/uinput or a daemon, none of which a gate may
require). AT-SPI is the sanctioned, permission-free surface: Qt exposes every
control with the name the app gave it, and the bridge turns on with the
environment variable QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1, no app change. Through
it this script moves *focus* and invokes *actions*, which is exactly what a Tab
does to a text field: focus leaves it and its editingFinished handler runs, the
path issue #143 aborted release builds on. It cannot send key presses; the key
chords are covered by the gate's QtTest events (tests/qml/GateKeys.qml).

Prints one JSON object: {"steps": [{"name", "ok", "detail"}], "error": null|str}.
usage: atspi_walk.py [--timeout SECONDS]
"""

import json
import re
import sys
import time

APP_NAMES = ("rowplay", "rowplay-qt", "rowplay-app")


def find_app(timeout):
    import pyatspi
    deadline = time.time() + timeout
    while time.time() < deadline:
        desktop = pyatspi.Registry.getDesktop(0)
        for i in range(desktop.childCount):
            app = desktop.getChildAtIndex(i)
            if app and app.name in APP_NAMES:
                return app
        time.sleep(0.5)
    return None


def walk(node, depth=0, maxd=12):
    yield node
    if depth < maxd:
        for i in range(node.childCount):
            child = node.getChildAtIndex(i)
            if child:
                yield from walk(child, depth + 1, maxd)


def find(app, role, name=None, index=0):
    matches = [n for n in walk(app) if n.getRoleName() == role and (name is None or n.name == name)]
    return matches[index] if len(matches) > index else None


def names(app, role):
    return [n.name for n in walk(app) if n.getRoleName() == role]


def count_label(app):
    """The '17 matching' label: the first label that starts with a number."""
    for node in walk(app):
        if node.getRoleName() == "label" and re.match(r"^\d+\s", node.name or ""):
            return node.name
    return None


def main():
    timeout = float(sys.argv[sys.argv.index("--timeout") + 1]) if "--timeout" in sys.argv else 40
    steps, error = [], None

    def step(name, ok, detail=""):
        steps.append({"name": name, "ok": bool(ok), "detail": str(detail)})

    try:
        app = find_app(timeout)
        step("the app appears on the accessibility bus", app is not None, app.name if app else "not found")
        if app is None:
            print(json.dumps({"steps": steps, "error": None}))
            return
        texts = [n for n in walk(app) if n.getRoleName() == "text" and n.getState().contains(__import__("pyatspi").STATE_EDITABLE)
                 and n.getState().contains(__import__("pyatspi").STATE_FOCUSABLE)]
        step("the search field and both date fields are exposed as editable text", len(texts) >= 3, f"{len(texts)} editable text fields")

        def focus(node):
            node.queryComponent().grabFocus()
            time.sleep(0.5)

        if len(texts) >= 3:
            search, date_from, date_to = texts[:3]
            before = count_label(app)
            focus(date_from)
            date_from.queryEditableText().setTextContents("2026-05-15")
            time.sleep(0.3)
            focus(date_to)   # focus leaves From: editingFinished -> Library.setDateRange -> model reset
            after = count_label(app)
            step("leaving From with a date resets the list without aborting", before and after and before != after, f"{before!r} -> {after!r}")
            focus(search)    # focus leaves To: editingFinished again
            date_from.queryEditableText().setTextContents("")
            focus(date_to)
            focus(search)
            restored = count_label(app)
            step("clearing the date and leaving the field restores the list", restored == before, f"{restored!r} (was {before!r})")

        replay = find(app, "button", "Replay")
        step("the workout's Replay button is exposed", replay is not None)
        if replay is not None:
            replay.queryAction().doAction(0)
            time.sleep(4)
            play = find(app, "button", "Play")
            step("the replay screen shows a Play button", play is not None, ",".join(sorted(set(names(app, "button")) & {"Play", "Pause"})))
            if play is not None:
                play.queryAction().doAction(0)
                time.sleep(1)
                step("Press Play turns it into Pause", find(app, "button", "Pause") is not None)
                pause = find(app, "button", "Pause")
                if pause is not None:
                    pause.queryAction().doAction(0)
                    time.sleep(1)
                    step("Press Pause turns it back into Play", find(app, "button", "Play") is not None)
            step("the app is still on the bus after every step", find_app(5) is not None)
    except Exception as exc:  # reported, never swallowed: the harness fails the check
        error = f"{type(exc).__name__}: {exc}"
    print(json.dumps({"steps": steps, "error": error}))


if __name__ == "__main__":
    main()
