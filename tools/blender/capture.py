#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Capture the approved runtime moment and a fixed-time replay sequence.

Temporarily instruments the existing gate, restoring its source in finally.
Run with .envrc sourced and a real Qt renderer (xcb/Wayland or cocoa/Metal).
No user cache is used. --baseline reads the scene at main, not scratch images.
"""

import argparse
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def replace_once(text, old, new):
    if text.count(old) != 1:
        raise ValueError(f"capture hook moved: {old[:80]}")
    return text.replace(old, new)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scheme", choices=("light", "dark"), default="light")
    parser.add_argument("--baseline", action="store_true")
    parser.add_argument("--baseline-ref", default="main")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    main_path = ROOT / "qml/RowPlay/Main.qml"
    scene_path = ROOT / "qml/RowPlay/Replay/ReplayScene.qml"
    original_main, original_scene = main_path.read_text(), scene_path.read_text()
    text = replace_once(original_main,
        '            case 61: Replay.loadGhost(-1); break  // dismiss ghost',
        '            case 61: Replay.loadGhost(-1); Replay.seek(0.50091); '
        'root.grabSettledScene("style-row-middrive"); root.gateStep = 299; break')
    text = replace_once(text,
        '            case 55: Replay.loadWorkout(1003); break     // skierg demo workout',
        '            case 55: root.gateStep = 58; break // row-only visual comparison')
    cases = []
    step = 300
    for tier, name in enumerate(("low", "medium", "high", "ultra")):
        cases.append(f'case {step}: Replay.setQualityIndex({tier}); Replay.seek(0.50091); '
                     f'root.grabSettledScene("style-{name}"); break')
        step += 1
    for frame in range(13):
        cases.append(f'case {step}: Replay.setQualityIndex(1); '
                     f'Replay.seek(0.50091 + {frame * 0.25} / Replay.durationSeconds); '
                     f'root.grabSettledScene("motion-{frame:03}"); break')
        step += 1
    cases.append(f'case {step}: Qt.quit(); break')
    text = replace_once(text, "            switch (root.gateStep) {",
                        "            switch (root.gateStep) {\n" + "\n".join(cases))
    anchor = '            result.saveToFile(Settings.screenshotDir + "/" + name + ".ppm")'
    text = replace_once(text, anchor, anchor + '\n            console.log("ART_FRAME " + JSON.stringify({name: name, frame: Array.prototype.slice.call(Replay.poseFrame), grip: Replay.gripPoses, tier: Replay.effectiveQuality}))')
    try:
        main_path.write_text(text)
        if args.baseline:
            scene_path.write_bytes(subprocess.check_output(
                ["git", "show", f"{args.baseline_ref}:qml/RowPlay/Replay/ReplayScene.qml"], cwd=ROOT))
        subprocess.run(["cargo", "build", "-p", "rowplay-app"], cwd=ROOT, check=True)
        env = dict(os.environ, ROWPLAY_SMOKE_GATE="1", ROWPLAY_SYNC_MOCK="1",
                   ROWPLAY_GATE_PROFILE="full", ROWPLAY_FORCE_COLOR_SCHEME=args.scheme,
                   ROWPLAY_SMOKE_SCREENSHOT_DIR=str(output), ROWPLAY_DATA_DIR=str(output / "data"),
                   QT_MESSAGE_PATTERN="[%{time process}] %{message}")
        with (output / "gate.log").open("w") as log:
            subprocess.run([str(ROOT / "target/debug/rowplay-app")], cwd=ROOT, env=env,
                           stdout=log, stderr=subprocess.STDOUT, check=True, timeout=900)
        log = (output / "gate.log").read_text()
        if any(error in log for error in ("screenshot FAILED", "Error:", "failed to load component", "is not a type")):
            raise RuntimeError("capture failed; inspect gate.log")
        if not (output / "motion-012.png").is_file():
            raise RuntimeError("incomplete replay sequence")
    finally:
        main_path.write_text(original_main)
        scene_path.write_text(original_scene)


if __name__ == "__main__":
    main()
