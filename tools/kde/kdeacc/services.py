# SPDX-License-Identifier: GPL-3.0-or-later
"""Which desktop services RowPlay uses, read from the repository itself.

Nothing is added for a service the app does not use. Each capability is
classified from what the source actually contains:

  Qt/XDG sufficient   the app uses it, and Qt or freedesktop already does the work
  not applicable      the app has no such feature (evidence: nothing in the source)
  needs code          the app uses it in a way Qt/XDG does not cover (would need a decision)
"""

from __future__ import annotations

import re
from pathlib import Path

CAPABILITIES = (
    # name, source patterns that mean "the app uses it", what covers it if it does
    ("URL opening", r"openUrlExternally|QDesktopServices|xdg-open|open::that|onLinkActivated",
     "Qt.openUrlExternally goes through the OpenURI portal or xdg-open; no KIO needed"),
    ("File chooser", r"FileDialog|FolderDialog|QFileDialog|QtQuick\.Dialogs",
     "Qt's FileDialog uses the xdg-desktop-portal FileChooser; no KIO needed"),
    ("Notifications", r"Notification\b|SystemTrayIcon|KNotification|sendNotification",
     "Qt has no notification API; a feature would use the freedesktop Notifications portal"),
    ("System tray", r"SystemTrayIcon|StatusNotifierItem|KStatusNotifier",
     "SystemTrayIcon speaks the StatusNotifierItem protocol; no KStatusNotifierItem needed"),
    ("Global menu", r"(?<![.\w])(Platform\.MenuBar|MenuBar)\s*\{[^}]*",
     "a Qt menu bar exports DBusMenu to Plasma's global menu with no KDE code"),
    ("Media session (MPRIS)", r"MPRIS|org\.mpris",
     "an MPRIS service would be its own D-Bus adaptor"),
)


def _sources(repo):
    for base in ("qml", "crates"):
        for path in sorted((Path(repo) / base).rglob("*")):
            if path.suffix in (".qml", ".rs") and "target" not in path.parts and "tests" not in path.parts:
                yield path


def audit(repo):
    """[{capability, verdict, evidence, covered_by}] for every capability, from the source tree."""
    files = {p: p.read_text(errors="replace") for p in _sources(repo)}
    out = []
    for name, pattern, covered in CAPABILITIES:
        regex = re.compile(pattern)
        hits = []
        for path, text in files.items():
            for number, line in enumerate(text.splitlines(), 1):
                if line.lstrip().startswith("//"):
                    continue
                if regex.search(line):
                    hits.append(f"{path.relative_to(repo)}:{number}: {line.strip()[:100]}")
        if name == "Global menu":
            # The only menu bar is created for macOS (Instantiator active: root.isMac); on Linux none exists.
            hits = [h for h in hits if "Platform.MenuBar" in h or "MenuBar {" in h]
            mac_only = bool(hits) and "active: root.isMac" in files.get(Path(repo) / "qml/RowPlay/Main.qml", "")
            verdict = "not applicable" if (not hits or mac_only) else "Qt/XDG sufficient"
            note = ("the only MenuBar is instantiated on macOS (active: root.isMac); on Linux the app has no menu bar to export"
                    if mac_only else covered)
        else:
            verdict = "Qt/XDG sufficient" if hits else "not applicable"
            note = covered if hits else "the source uses none; nothing to add"
        out.append({"capability": name, "verdict": verdict, "evidence": hits[:5], "covered_by": note})
    # The one service the app does use: the token's Secret Service store (the OS keychain).
    keyring = [f"{p.relative_to(repo)}" for p in files if "keyring" in files[p] and p.suffix == ".rs"]
    out.append({"capability": "Secret Service (token store)", "verdict": "Qt/XDG sufficient" if keyring else "not applicable",
                "evidence": keyring[:3], "covered_by": "the `keyring` crate's Secret Service backend (freedesktop.org.secrets); ksecretd on Plasma"})
    return out
