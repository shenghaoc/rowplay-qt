# SPDX-License-Identifier: GPL-3.0-or-later
"""Running RowPlay's own runtime-error gate, and reading what it says.

The gate is `crates/rowplay-app/tests/qml_runtime_gate.rs`; this module does
not reimplement any of it. It runs the same `cargo test` commands AGENTS.md
gives, in a worktree with its own target directory, and parses the output.
A native run is Wayland on the real GPU: `LIBGL_ALWAYS_SOFTWARE` is removed
from the environment and the run fails if the renderer is a software one.
"""

from __future__ import annotations

import re
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

WALK_RE = re.compile(r"gate walk \((\w+) profile\): ([0-9.]+) s of app log")
ENTRY_RE = re.compile(r"replay entry, step 52 to first frame: ([0-9.]+) s")
SHADOW_RE = re.compile(r"shadow darkens ([0-9.]+) % of the capture \((\d+) px\) by ([0-9.]+) %")
RESULT_RE = re.compile(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed")
RENDERER_RE = re.compile(r"OpenGL VENDOR: (.+?) RENDERER: (.+?) VERSION: (.+)")
STARVED_RE = re.compile(r"holds that ran out their tick bound: (\d+) \(steps ([^)]*)\)")
ACCENT_RE = re.compile(r"accent: palette accent (#\w+), highlight (#\w+), resolved (#\w+)")
KEYS_RE = re.compile(r"keyboard contract: (\d+) lines, all ok, ended by (.+)")
KEYS_SKIPPED_RE = re.compile(r"keyboard contract: skipped, no active window under (\w+)")
SOFTWARE = ("llvmpipe", "softpipe", "swrast", "software rasterizer")
# What must never appear in a healthy run's output. #143's abort, in every spelling.
SIGNATURES = re.compile(
    r"BorrowError|Failed to borrow|role_names|panicked at|panic in ffi|panic in a destructor|"
    r"non-unwinding panic|aborting\.|SIGABRT|Segmentation fault|core dumped|abnormal")
# Log noise that is information, not a warning.
INFO_CATEGORIES = ("qt.rhi.general", "qt.scenegraph.general", "qt.qpa.theme", "qt.gui.icon.loader")


def normalise_warning(line):
    """A warning with timestamps, addresses and numbers removed, so repeats count together."""
    line = re.sub(r"^\s*[0-9.]+\s+", "", line)
    line = re.sub(r"0x[0-9a-fA-F]+", "0x…", line)
    return re.sub(r"\b\d+(\.\d+)?\b", "N", line).strip()  # standalone numbers only: not the 2 in "graphs2d"


def find_warnings(gate_log):
    counts = Counter()
    for line in gate_log.splitlines():
        text = re.sub(r"^\s*[0-9.]+\s+", "", line)
        if any(text.startswith(c) for c in INFO_CATEGORIES):
            continue
        if re.match(r"(qt\.[a-z0-9._]+|qml|QML|Qt Quick|file://).*", text) and re.search(
                r"warn|invalid|unable|cannot|failed|deprecated|error", text, re.I):
            counts[normalise_warning(line)] += 1
    return counts


@dataclass
class GateResult:
    name: str
    command: str
    exit_code: int
    wall_seconds: float
    profile: str = ""
    walk_seconds: float | None = None
    passed_tests: int | None = None
    failed_tests: int | None = None
    captures: int = 0
    replay_entry_seconds: float | None = None
    shadow: dict = field(default_factory=dict)
    renderer: dict = field(default_factory=dict)
    accent: dict = field(default_factory=dict)
    keyboard: dict = field(default_factory=dict)
    starved_holds: dict = field(default_factory=dict)
    signatures: list = field(default_factory=list)
    warnings: dict = field(default_factory=dict)
    output_file: str = ""


def parse_gate(name, command, exit_code, wall, output, gate_log, captures=0, output_file=""):
    result = GateResult(name, command, exit_code, round(wall, 1), captures=captures, output_file=output_file)
    if m := WALK_RE.search(output):
        result.profile, result.walk_seconds = m.group(1), float(m.group(2))
    passed = failed = None
    for m in RESULT_RE.finditer(output):   # the last `test result` is the integration test's
        passed, failed = int(m.group(2)), int(m.group(3))
    result.passed_tests, result.failed_tests = passed, failed
    if m := ENTRY_RE.search(output):
        result.replay_entry_seconds = float(m.group(1))
    if m := SHADOW_RE.search(output):
        result.shadow = {"percent_of_capture": float(m.group(1)), "pixels": int(m.group(2)), "darkening_percent": float(m.group(3))}
    if m := RENDERER_RE.search(gate_log):
        result.renderer = {"vendor": m.group(1), "renderer": m.group(2), "version": m.group(3)[:80]}
    if m := ACCENT_RE.search(output):
        result.accent = {"accent": m.group(1), "highlight": m.group(2), "resolved": m.group(3)}
    if m := STARVED_RE.search(output):
        result.starved_holds = {"count": int(m.group(1)), "steps": m.group(2)}
    if m := KEYS_RE.search(output):
        result.keyboard = {"lines": int(m.group(1)), "ended_by": m.group(2).strip()}
    elif m := KEYS_SKIPPED_RE.search(output):
        # A display with no window manager never activates a window, and shortcuts fire only in one.
        result.keyboard = {"lines": 0, "ended_by": f"skipped: no active window under {m.group(1)}", "skipped": True}
    result.signatures = sorted({m.group(0) for m in SIGNATURES.finditer(output + "\n" + gate_log)})
    result.warnings = dict(find_warnings(gate_log).most_common(10))
    return result


def gate_problems(result, hardware=False, expect_full=False, baseline=False):
    """[str] of everything that makes a gate result unacceptable; empty means it passed.

    baseline is for the comparison tree (post-#144 main), which by definition has neither the
    keyboard contract nor the accent rule this branch adds: those two summaries are not asked of it."""
    problems = []
    if result.exit_code != 0:
        problems.append(f"exit status {result.exit_code}")
    if result.failed_tests not in (0, None) or result.passed_tests in (0, None):
        problems.append(f"tests passed={result.passed_tests} failed={result.failed_tests}")
    if result.signatures:
        problems.append("abort/panic signature: " + ", ".join(result.signatures))
    if result.walk_seconds is None:
        problems.append("no `gate walk` timing line: the walk did not finish")
    if not baseline and not result.keyboard:
        problems.append("no keyboard-contract summary")
    if hardware and result.keyboard.get("skipped"):
        problems.append("the keyboard contract was skipped on a native run: the gate window was not active "
                        "(a compositor that did not raise it); a skip is accepted only on a display with no window manager")
    if not baseline and not result.accent:
        problems.append("no accent-rule summary")
    if hardware and result.starved_holds:
        problems.append(f"the gate window was starved of frames: {result.starved_holds['count']} holds ran out their bound "
                        f"(steps {result.starved_holds['steps']}); it was occluded or not the active window, so the run says nothing about the app")
    if hardware:
        name = (result.renderer.get("renderer") or "").lower()
        if not name:
            problems.append("no renderer line (QSG_INFO logging missing)")
        elif any(word in name for word in SOFTWARE):
            problems.append(f"renderer is a software rasteriser: {result.renderer['renderer']}")
    if expect_full:
        if result.captures < 70:
            problems.append(f"{result.captures} captures, expected the full walk's 70")
        if not result.shadow:
            problems.append("no shadow check line")
        if result.replay_entry_seconds is None:
            problems.append("no replay-entry timing")
    return problems


def cargo_command(release):
    argv = ["cargo", "test"]
    if release:
        argv += ["--release", "--config", "profile.release.debug-assertions=true"]
    return argv + ["-p", "rowplay-app", "--test", "qml_runtime_gate", "--", "--nocapture"]


def run_gate(runner, tree, out_dir, name, profile="quick", native=True, release=False, phase_shots=False, extra_env=None, timeout=1800):
    """Run the gate walk in `tree`; captures and the app log land in out_dir/name."""
    out = Path(out_dir) / name
    out.mkdir(parents=True, exist_ok=True)
    env = {
        "ROWPLAY_GATE_PROFILE": profile,
        "ROWPLAY_SMOKE_ARTIFACT_DIR": str(out),
        "ROWPLAY_SMOKE_SCREENSHOT_DIR": str(out),
        "QSG_INFO": "1",
        "LIBGL_ALWAYS_SOFTWARE": None,  # native means the GPU
    }
    if native:
        env.update({"QT_QPA_PLATFORM": "wayland", "QSG_RHI_BACKEND": "opengl"})
    if phase_shots:
        env.update({"ROWPLAY_PHASE_SHOTS": "1", "ROWPLAY_PHASE_CLOSEUPS": "1"})
    env.update(extra_env or {})
    argv = cargo_command(release)
    done = runner.in_tree(tree, argv, env=env, timeout=timeout, tag=f"gate-{name}", logfile=out / "test-output.txt")
    log_path = out / "gate-log.txt"
    gate_log = log_path.read_text(errors="replace") if log_path.exists() else ""
    return parse_gate(name, " ".join(argv), done.rc, done.seconds, done.text, gate_log,
                      captures=len(list(out.glob("*.png"))), output_file=f"{name}/test-output.txt")
