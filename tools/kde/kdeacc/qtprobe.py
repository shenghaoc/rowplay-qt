# SPDX-License-Identifier: GPL-3.0-or-later
"""The Qt-facing probe: what the bundled Qt reports on this desktop, and what Theme.qml resolves.

The probe runs the repository's real `qml/RowPlay/Theme.qml` (copied, with a
stand-in `Settings` singleton) in the bundled Qt's own `qml` tool, in a window
that is never shown, and reads one JSON line. It also turns on Qt's theme and
icon-loader logging, which says which platform theme was created and which
icon theme Qt initialised, the facts ADR 0018 rests on.
"""

from __future__ import annotations

import json
import re
import shutil
import tempfile
from pathlib import Path

from .shell import bundled_qt_env, host_env

UNUSABLE_ACCENTS = ("#000000", "#308cc6")  # Qt's palette defaults: Theme.resolveAccent's list
THEME_RE = re.compile(r'Successfully created platform theme "([^"]+)"')
THEME_NAMES_RE = re.compile(r"theme names to list of theme names: QList\(([^)]*)\)")
ICON_RE = re.compile(r'Initialized icon loader with system theme "([^"]+)"')
CONTRAST = {0: "NoPreference", 1: "HighContrast"}
SCHEME = {0: "Unknown", 1: "Light", 2: "Dark"}


def parse_probe_output(text):
    """The probe's dict from its stderr/stdout, with the theme facts from Qt's own logging."""
    line = next((ln for ln in text.splitlines() if "PROBE {" in ln), None)
    if line is None:
        raise ValueError("the probe printed no PROBE line:\n" + text[-800:])
    info = json.loads(line.split("PROBE ", 1)[1])
    created = THEME_RE.search(text)
    names = THEME_NAMES_RE.search(text)
    icon = ICON_RE.search(text)
    info["platformThemeCreated"] = created.group(1) if created else None
    info["platformThemeNames"] = [n.strip().strip('"') for n in names.group(1).split(",")] if names else []
    info["iconTheme"] = icon.group(1) if icon else None
    info["colorSchemeName"] = SCHEME.get(info.get("colorScheme"), "?")
    info["contrastName"] = CONTRAST.get(info.get("contrast"), "?")
    info["fusion"] = "ButtonPanel" in info.get("buttonBackground", "")
    return info


def icon_theme_evidence(info):
    """(ok, detail): whether Qt's own log gave a usable icon-theme name.

    The invariant is that the theme was measured, never which theme it is: a user may
    legitimately run any freedesktop icon theme."""
    theme = (info.get("iconTheme") or "").strip()
    if theme:
        return True, f"Qt initialised its icon loader with system theme {theme!r}"
    return False, ("no icon theme was measured: Qt's icon-loader diagnostic ('Initialized icon loader with system "
                   "theme ...') is missing from the probe's output, or its format changed")


def accent_condition(info):
    """The semantic condition the accent fix rests on, judged from a probe.

    Returns (ok, story). Platform-neutral: whether Qt's Accent is usable decides which
    branch of Theme.resolveAccent must have run, and nothing here names a desktop."""
    accent, highlight, resolved = info["accent"].lower(), info["highlight"].lower(), info["themeAccentColor"].lower()
    if accent not in UNUSABLE_ACCENTS:
        return resolved == accent, f"Qt reports a usable Accent {accent}; Theme.accentColor {resolved} must equal it"
    if highlight in UNUSABLE_ACCENTS:
        return not info["themeSystemAccentAvailable"], (
            f"Qt reports the default Accent {accent} and default Highlight {highlight}: no platform accent; "
            f"Theme falls back to its brand blue ({resolved})")
    return resolved == highlight, (
        f"Qt's Accent is its unset default {accent}, the Highlight {highlight} is usable; "
        f"Theme.accentColor {resolved} must equal the highlight")


def build_module(repo, scratch):
    """Copy Theme.qml and the Settings stub into scratch/RowPlay; return the import path."""
    module = Path(scratch) / "mod" / "RowPlay"
    module.mkdir(parents=True, exist_ok=True)
    shutil.copy(Path(repo) / "qml" / "RowPlay" / "Theme.qml", module / "Theme.qml")
    shutil.copy(Path(__file__).resolve().parent.parent / "probe" / "Settings.qml", module / "Settings.qml")
    (module / "qmldir").write_text(
        "module RowPlay\nsingleton Theme 1.0 Theme.qml\nsingleton Settings 1.0 Settings.qml\n")
    return Path(scratch) / "mod"


def run_probe(runner, repo, qt_dir, env_extra=None, base_env=None, tag="qt-probe", timeout=90, prefix=None):
    """Run the probe once; returns (info dict, Done). base_env defaults to the clean host environment."""
    scratch = tempfile.mkdtemp(prefix="kdeacc-probe-")
    try:
        mod = build_module(repo, scratch)
        qml = Path(__file__).resolve().parent.parent / "probe" / "probe.qml"
        env = dict(base_env if base_env is not None else host_env())
        env["LD_LIBRARY_PATH"] = f"{qt_dir}/lib"
        env["QT_FORCE_STDERR_LOGGING"] = "1"
        env["QT_LOGGING_RULES"] = "qt.qpa.theme=true;qt.gui.icon.loader=true"
        env.update(env_extra or {})
        done = runner.run([*(prefix or []), f"{qt_dir}/bin/qml", "-I", mod, qml, "--apptype", "gui"], env=env, tag=tag, timeout=timeout)
        return (parse_probe_output(done.text) if done.ok or "PROBE {" in done.text else None), done
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
