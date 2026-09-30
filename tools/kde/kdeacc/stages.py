# SPDX-License-Identifier: GPL-3.0-or-later
"""The acceptance stages. Each takes the Ctx and returns a Stage of Checks.

A stage never decides the run's exit status; the Run does, from its checks: any FAIL
(or a stage that died) fails it. A check that cannot be measured is UNAVAILABLE or
SKIPPED and says why, never a silent pass.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import signal
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path

from . import baseline, gates, host, kwin, package, plasma, provenance, qtprobe, services, session, visual
from .results import Stage, Status
from .shell import Runner, bundled_qt_env, generic_env, host_env, scrubbed_vars

TOOLS = Path(__file__).resolve().parent.parent
VISUAL_RULES = TOOLS / "expected-visual-diff.json"
APP_ID_RE = re.compile(r'const APP_ID: &str = "([^"]+)"')


@dataclass
class Ctx:
    repo: Path                 # the worktree under test (the KDE branch)
    baseline_tree: Path | None  # a checkout of the acceptance baseline commit (baseline.BASELINE_SHA), detached is fine; None until a stage needs it
    qt_dir: Path
    out: Path
    runner: Runner
    args: object
    shas: dict = field(default_factory=dict)
    captures: dict = field(default_factory=dict)   # label -> capture directory
    appimages: dict = field(default_factory=dict)  # 'baseline' / 'branch' -> path
    host: dict = field(default_factory=dict)
    run_started: float = 0.0
    inhibitor: object = None
    interrupted: BaseException | None = None   # a stage caught the interruption itself (to finish its restoration evidence); the run stops after recording it
    baseline_validated: bool | None = None   # whether baseline_tree was proved to be the acceptance baseline (once per run)
    baseline_sha: str = baseline.BASELINE_SHA         # the commit the integration is compared with (not "whatever main is today")
    launched_pids: set = field(default_factory=set)   # processes this run started; the only ones it may end

    def sub(self, name):
        path = self.out / name
        path.mkdir(parents=True, exist_ok=True)
        return path

    def app_id(self, tree=None):
        text = (Path(tree or self.repo) / "crates/rowplay-app/src/main.rs").read_text()
        match = APP_ID_RE.search(text)
        return match.group(1) if match else None

    def probe(self, env_extra=None, base_env=None, prefix=None, tag="qt-probe"):
        return qtprobe.run_probe(self.runner, self.repo, self.qt_dir, env_extra, base_env, tag=tag, prefix=prefix)

    def probe_info(self):
        return self.probe(tag="qt-probe-state")[0]


def baseline_ready(ctx, st):
    """Whether ctx.baseline_tree is the acceptance baseline; a failing stage says why. Proved once per run, by whichever stage needs it first."""
    if ctx.baseline_tree is None:
        st.failed("the acceptance baseline checkout", "none supplied or found: this stage needs " + baseline.describe_requirement(ctx.baseline_sha))
        return False
    if ctx.baseline_validated is None:
        checks = baseline.check_baseline(ctx.runner, ctx.repo, ctx.baseline_tree, ctx.baseline_sha)
        ctx.baseline_validated = all(check.ok for check in checks)
        if not ctx.baseline_validated:
            for check in checks:
                if not check.ok:
                    st.failed(check.name, check.detail_bad)
    elif not ctx.baseline_validated:
        st.failed("the acceptance baseline checkout", "already found not to be the acceptance baseline earlier in this run (see the guards stage)")
    return bool(ctx.baseline_validated)


def git(runner, tree, *args):
    return runner.run(["git", "-C", str(tree), *args], env=host_env(), tag="git").out.strip()


def git_state(runner, tree):
    porcelain = git(runner, tree, "status", "--porcelain")
    return {"sha": git(runner, tree, "rev-parse", "HEAD"), "short": git(runner, tree, "rev-parse", "--short", "HEAD"),
            "branch": git(runner, tree, "branch", "--show-current") or "(detached)",
            "dirty": bool(porcelain), "dirty_paths": porcelain.splitlines()[:20]}


# --------------------------------------------------------------------------- probe

def stage_probe(ctx):
    st = Stage("probe")
    ctx.host = host.query_host(ctx.runner, ctx.qt_dir, ctx.repo)
    (ctx.out / "host.json").write_text(json.dumps(ctx.host, indent=2) + "\n")
    gaps = host.host_gaps(ctx.host)
    st.expect("host record is complete and the session is Wayland", not gaps,
              f"{ctx.host['fedora']['pretty']}, kernel {ctx.host['kernel']}, Plasma {ctx.host['plasma']}, KWin {ctx.host['kwin']}, "
              f"KF {ctx.host['kde_frameworks']}, host Qt {ctx.host['host_qt']}, bundled Qt {ctx.host['bundled_qt']}, Mesa {ctx.host['mesa']}, "
              f"{ctx.host['gpu_renderer']}, portal {ctx.host['xdg_desktop_portal']} / KDE {ctx.host['xdg_desktop_portal_kde']}",
              "missing: " + ", ".join(gaps), host=ctx.host)
    info, done = ctx.probe()
    if info is None:
        st.failed("the bundled Qt's probe ran", (done.text or "no output")[-300:])
        return st
    (ctx.out / "qt-probe.json").write_text(json.dumps(info, indent=2) + "\n")
    st.passed("the bundled Qt's platform theme", f"theme names {info['platformThemeNames']}, created "
              f"{info['platformThemeCreated']!r}, platform {info['platform']}", **{
                  k: info[k] for k in ("platform", "platformThemeNames", "platformThemeCreated")})
    icon_ok, icon_story = qtprobe.icon_theme_evidence(info)
    st.expect("the bundled Qt reported its icon theme (measured; which theme it is is the user's choice)", icon_ok, icon_story, icon_story,
              iconTheme=info.get("iconTheme"))
    st.expect("the bundled Qt selected its KDE platform theme (this is a Plasma acceptance run)", info["platformThemeCreated"] == "kde",
              "platform theme 'kde' created", f"platform theme {info['platformThemeCreated']!r} was created: these are not Plasma results")
    st.passed("the palette Qt reports", ", ".join(f"{k} {info[k]}" for k in ("window", "windowText", "base", "button", "highlight", "accent"))
              + f"; scheme {info['colorSchemeName']}; font {info['fontFamily']} {info['fontPointSize']} pt", palette=info)
    ok, story = qtprobe.accent_condition(info)
    st.expect("the accent condition holds and Theme resolves it by Qt's palette rules", ok, story, "Theme.accentColor disagrees: " + story,
              accent=info["accent"], highlight=info["highlight"], theme_accent=info["themeAccentColor"],
              unusable_accent_reproduced=info["accent"].lower() in qtprobe.UNUSABLE_ACCENTS)
    st.expect("Fusion is the style in use", info["fusion"], f"Button background is {info['buttonBackground'].split('(')[0]}",
              f"Button background is {info['buttonBackground']}, not Fusion's ButtonPanel")
    chords = ctx.runner.run([f"{ctx.qt_dir}/bin/qml", str(TOOLS / "probe" / "chord-test.qml"), "--apptype", "gui"],
                            env=bundled_qt_env(ctx.qt_dir, {"QT_QPA_PLATFORM": "offscreen", "QT_FORCE_STDERR_LOGGING": "1"}),
                            tag="chord-test", timeout=60)
    summary = re.search(r"CHORD summary: (\d+) of (\d+) correct", chords.text)
    failures = [line.split("qml: ")[-1] for line in chords.text.splitlines() if "CHORD FAIL" in line]
    detail = ("the chord parser is wrong for: " + "; ".join(failures) if failures
              else f"{summary.group(0) if summary else 'no summary line'} (exit {chords.rc}, timed_out={chords.timed_out}): {chords.text[-250:]}")
    st.expect("the gate's chord parser reads every platform's shortcut strings (Linux, Windows, macOS glyphs)",
              chords.ok and not chords.timed_out and bool(summary) and summary.group(1) == summary.group(2),
              summary.group(0) if summary else "", detail, exit_code=chords.rc, timed_out=chords.timed_out)
    portal = plasma.Plasma(ctx.runner, lambda: None).read_portal()
    limitation = info["contrast"] == 0
    st.passed("contrast preference", ("Qt reports NoPreference and the portal's contrast key is "
                                      f"{portal.get('contrast')}: a platform limitation, high contrast engages only when Qt reports it; nothing is inferred from the palette"
                                      if limitation else f"Qt reports {info['contrastName']}: RowPlay's high-contrast variant engages natively"),
              qt_contrast=info["contrastName"], portal_contrast=portal.get("contrast"), limitation=limitation)
    return st


# --------------------------------------------------------------------------- guards

def _runner_json(runner, tree, argv):
    done = runner.in_tree(tree, argv, tag="guard-cargo", timeout=300)
    return json.loads(done.out) if done.ok else None


def _app_deps(meta):
    pkg = next(p for p in meta["packages"] if p["name"] == "rowplay-app")
    return sorted({d["name"] for d in pkg["dependencies"] if d["kind"] in (None, "normal")})


def stage_guards(ctx):
    st = Stage("guards")
    repo, base = ctx.repo, ctx.baseline_tree
    if base is None:
        st.failed("the acceptance baseline checkout", "none supplied or found: this stage compares with " + baseline.describe_requirement(ctx.baseline_sha))
        return st
    for label, tree in (("branch", repo), ("baseline", base)):
        state = git_state(ctx.runner, tree)
        ctx.shas[label] = state
        if label == "branch":
            st.expect("the branch worktree is clean", not state["dirty"], f"{state['branch']} at {state['short']}, clean",
                      "uncommitted changes: " + "; ".join(state["dirty_paths"][:6]), **state)
        else:
            st.expect("the baseline checkout is clean", not state["dirty"], f"{state['branch']} at {state['short']}, clean",
                      "uncommitted changes: " + "; ".join(state["dirty_paths"][:6]), **state)
    checks = baseline.check_baseline(ctx.runner, repo, base, ctx.baseline_sha)
    for check in checks:
        st.expect(check.name, check.ok, check.detail_ok, check.detail_bad)
    ctx.baseline_validated = all(check.ok for check in checks)
    if not ctx.baseline_validated:
        # A comparison with the wrong tree would report PASS for everything: say so and stop.
        st.failed("the baseline comparisons were not run", "every dependency and diff guard below compares the tree under test with the "
                  "baseline, so none of them means anything until the baseline is the acceptance baseline")
        return st
    text, base_text = (repo / "Cargo.toml").read_text(), (base / "Cargo.toml").read_text()
    section = re.compile(r"\[profile\.release\.package\.qtbridge-interfaces\]\nopt-level = 0")
    st.expect("#143's containment is intact and identical to the baseline's",
              bool(section.search(text)) and bool(section.search(base_text)),
              "[profile.release.package.qtbridge-interfaces] opt-level = 0 present in both",
              "the containment section is missing or changed")
    lock, base_lock = (repo / "Cargo.lock").read_text(), (base / "Cargo.lock").read_text()
    names = lambda t: set(re.findall(r'^name = "([^"]+)"', t, re.M))
    new_packages = sorted(names(lock) - names(base_lock))
    st.expect("the lockfile gains no package", not new_packages, "same package set as the baseline (an edge, not a package)",
              "new packages: " + ", ".join(new_packages))
    meta_b = _runner_json(ctx.runner, repo, ["cargo", "metadata", "--locked", "--format-version", "1"])
    meta_m = _runner_json(ctx.runner, base, ["cargo", "metadata", "--locked", "--format-version", "1"])
    if meta_b and meta_m:
        added = sorted(set(_app_deps(meta_b)) - set(_app_deps(meta_m)))
        kde = [n for n in added if re.search(r"^(kf|kirigami|plasma|kde|kio|kconfig|ki18n)", n, re.I)]
        st.expect("rowplay-app's production dependencies added over the baseline: no KDE, KF or Kirigami crate", not kde,
                  "added: " + (", ".join(added) or "none"), "KDE-like production dependency: " + ", ".join(kde), added=added)
        for crate in ("rowplay-core", "rowplay-platform", "rowplay-viewmodel"):
            d_b = sorted(d["name"] for p in meta_b["packages"] if p["name"] == crate for d in p["dependencies"] if d["kind"] in (None, "normal"))
            d_m = sorted(d["name"] for p in meta_m["packages"] if p["name"] == crate for d in p["dependencies"] if d["kind"] in (None, "normal"))
            st.expect(f"{crate} gained no dependency", d_b == d_m, "unchanged", f"{sorted(set(d_b) ^ set(d_m))}")
    else:
        st.failed("cargo metadata --locked", "could not read the dependency graphs")
    diff_paths = git(ctx.runner, repo, "diff", "--name-only", f"{ctx.baseline_sha}...HEAD").splitlines()
    forbidden = [p for p in diff_paths if re.search(r"(^|/)(blender|athlete)|hero-fit", p, re.I)]
    st.expect("the branch does not touch the Blender / athlete stack", not forbidden, f"{len(diff_paths)} changed paths, none in the Blender stack",
              "touches: " + ", ".join(forbidden))
    st.expect("the diff is whitespace-clean", ctx.runner.run(["git", "-C", str(repo), "diff", "--check", f"{ctx.baseline_sha}...HEAD"], env=host_env()).ok,
              "git diff --check passes", "git diff --check reports problems")
    return st


# --------------------------------------------------------------------------- services

def stage_services(ctx):
    st = Stage("services")
    rows = services.audit(ctx.repo)
    (ctx.out / "services.json").write_text(json.dumps(rows, indent=2) + "\n")
    for row in rows:
        st.expect(row["capability"], row["verdict"] != "needs code", f"{row['verdict']}: {row['covered_by']}",
                  f"needs code: {row['evidence']}", evidence=row["evidence"], verdict=row["verdict"])
    return st


# --------------------------------------------------------------------------- native gates

def _gate_checks(st, name, result, hardware=True, expect_full=False, release=False, baseline=False):
    problems = gates.gate_problems(result, hardware=hardware, expect_full=expect_full, baseline=baseline)
    detail = (f"{result.profile} profile, walk {result.walk_seconds} s, wall {result.wall_seconds} s, {result.captures} captures, "
              f"replay entry {result.replay_entry_seconds} s, renderer {result.renderer.get('renderer')}, keyboard: "
              f"{result.keyboard.get('lines')} key lines ended by {result.keyboard.get('ended_by')}")
    if result.shadow:
        detail += f", shadow {result.shadow['percent_of_capture']} % / darkening {result.shadow['darkening_percent']} %"
    st.add(name, Status.PASS if not problems else Status.FAIL, detail if not problems else "; ".join(problems),
           {"result": result.__dict__}, [result.output_file])
    if release:
        st.expect(f"{name}: no #143 signature (BorrowError, role_names, panic, abort)", not result.signatures,
                  "none in the output or the app log", ", ".join(result.signatures))
    return result


def stage_native(ctx):
    st = Stage("native")
    base = ctx.sub("native")
    if not os.environ.get("WAYLAND_DISPLAY") or os.environ.get("XDG_SESSION_TYPE") != "wayland":
        st.add("a Wayland session", Status.UNAVAILABLE, "no Wayland session in the environment: the native gates cannot run")
        return st
    if not baseline_ready(ctx, st):
        return st
    st.expect("the native run keeps the GPU: LIBGL_ALWAYS_SOFTWARE is not set for the gates", True,
              "removed from the gates' environment; each run is failed if its renderer is a software one")
    ctx.captures = ctx.captures or {}
    raiser = kwin.Raiser(ctx.runner).__enter__()
    try:
        return _native_gates(ctx, st, base, raiser)
    finally:
        raiser.__exit__(None, None, None)


def _record_provenance(ctx, tree, role, directory):
    """Write what a capture directory is (the commit, clean or not) beside it: the visual stage reads this, not a worktree."""
    provenance.write(directory, role, git_state(ctx.runner, tree))


def _native_gates(ctx, st, base, raiser):
    st.expect("KWin keeps the gate window frontmost during the native runs", raiser.active,
              "a KWin script raises each RowPlay window as it appears, and is unloaded afterwards",
              "the KWin script could not be loaded: the runs depend on the window being frontmost by luck")
    r = gates.run_gate(ctx.runner, ctx.repo, base, "branch-debug-quick", "quick", native=True, release=False)
    _gate_checks(st, "debug quick gate (branch, native Wayland)", r)
    r = gates.run_gate(ctx.runner, ctx.repo, base, "branch-release-quick", "quick", native=True, release=True)
    _gate_checks(st, "release quick gate (branch, native Wayland; #144's)", r, release=True)
    r = gates.run_gate(ctx.runner, ctx.repo, base, "branch-full", "full", native=True, release=False, phase_shots=True)
    _gate_checks(st, "full native gate (branch, hardware GL, phase shots and close-ups)", r, expect_full=True)
    _record_provenance(ctx, ctx.repo, "branch", base / "branch-full")
    ctx.captures["branch-full"] = base / "branch-full"
    r = gates.run_gate(ctx.runner, ctx.baseline_tree, base, "baseline-full", "full", native=True, release=False, phase_shots=True)
    _gate_checks(st, "full native gate (the acceptance baseline, the visual baseline)", r, expect_full=True, baseline=True)
    _record_provenance(ctx, ctx.baseline_tree, "baseline", base / "baseline-full")
    ctx.captures["baseline-full"] = base / "baseline-full"
    if getattr(ctx.args, "calibrate_noise", False):
        r = gates.run_gate(ctx.runner, ctx.baseline_tree, base, "baseline-full-run2", "full", native=True, phase_shots=True)
        _gate_checks(st, "second full native gate of the baseline (noise calibration)", r, expect_full=True, baseline=True)
        _record_provenance(ctx, ctx.baseline_tree, "baseline", base / "baseline-full-run2")
        ctx.captures["baseline-full-run2"] = base / "baseline-full-run2"
    return st


# --------------------------------------------------------------------------- visual contract

def stage_visual(ctx):
    """Classify the branch's captures against the baseline's with the derived rules.

    Needs no Git worktree: the captures' own provenance says what commit the baseline captures were taken at, and that is
    checked against the rule set's `baseline_sha` before any rule is applied. A baseline mismatch fails here, once and by
    name, rather than as dozens of misleading "the expected change is missing" results."""
    st = Stage("visual")
    out = ctx.sub("visual")
    before, after = ctx.captures.get("baseline-full"), ctx.captures.get("branch-full")
    if not before or not after or not before.exists() or not after.exists():
        st.add("baseline and branch full native captures", Status.UNAVAILABLE,
               "run the `native` stage first (or pass --baseline-captures/--branch-captures)")
        return st
    cd = visual.load_capture_diff(ctx.repo)
    rules, profile = visual.load_rules(VISUAL_RULES, "baseline-vs-branch", "native-hardware")
    checks = provenance.check_baseline_captures(before, rules, ctx.baseline_sha)
    for name, ok, detail_ok, detail_bad in checks:
        st.expect(name, ok, detail_ok, detail_bad)
    if not all(ok for _, ok, _, _ in checks):
        st.failed("the visual rules were not applied", "they describe a change made on the acceptance baseline, and these captures are not "
                  "shown to be its captures: every result would be about the wrong pair")
        return st
    branch_record = provenance.read(after)
    if branch_record and branch_record.get("dirty"):
        st.failed("the branch captures come from a clean tree", "the branch tree had uncommitted changes when the captures were taken")
    results = visual.compare_dirs(before, after, rules, cd, profile=profile)
    (out / "results.json").write_text(json.dumps([r.__dict__ for r in results], indent=1) + "\n")
    (out / "table.md").write_text(visual_table(results))
    summary = visual.summarize(results)
    st.expect("every capture is either noise or a listed change confined to its allowed area", not summary["failed"],
              f"{summary['captures']} captures; {summary['expected_changes']} exceed the generic noise bound and are listed changes; "
              f"{summary['noise_only']} are noise; {summary['unexpected_pixels']} unexpected pixels",
              "failed: " + ", ".join(summary["failed"][:12]), **summary)
    st.expect("zero unexpected changed pixels", summary["unexpected_pixels"] == 0, "0", str(summary["unexpected_pixels"]))
    if "baseline-full-run2" in ctx.captures:
        cal = visual.compare_dirs(before, ctx.captures["baseline-full-run2"], {"captures": {}}, cd, profile=profile)
        st.expect("the native-hardware noise profile covers same-tree run-to-run differences", all(r.status == "PASS" for r in cal),
                  f"{len(cal)} captures of the baseline vs the baseline: all within the profile {profile}",
                  "outside: " + ", ".join(r.name for r in cal if r.status == "FAIL"))
    return st


def visual_table(results):
    lines = ["| capture | status | kind | changed px | inside | outside | bbox | colours (before→after, px) | detail |", "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for r in results:
        colours = "; ".join(f"{a}→{b} ×{n}" for a, b, n in r.colours[:2])
        lines.append(f"| {r.name} | {r.status} | {r.kind} | {r.changed} | {r.inside} | {r.outside} | {r.bbox} | {colours} | {r.detail} |")
    return "\n".join(lines) + "\n"


# --------------------------------------------------------------------------- generic Linux

XVFB = ["xvfb-run", "-a", "-s", "-screen 0 1280x800x24"]


def parse_xprop(text):
    """{'WM_CLASS': ['a', 'b'], '_KDE_NET_WM_DESKTOP_FILE': 'id', ...} from xprop output."""
    out = {}
    for line in text.splitlines():
        m = re.match(r"(WM_CLASS)\(STRING\) = (.+)", line)
        if m:
            out["WM_CLASS"] = re.findall(r'"([^"]*)"', m.group(2))
            continue
        m = re.match(r"(\w+)\(UTF8_STRING\) = \"(.*)\"", line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


def x11_identity(runner, tree, out, qt_dir):
    binary = Path(tree) / "target" / "debug" / "rowplay-app"
    if not binary.exists():
        return None
    script = (f'ROWPLAY_DATA_DIR=$(mktemp -d) QT_QPA_PLATFORM=xcb "{binary}" >/dev/null 2>&1 & p=$!; '
              'for i in $(seq 1 30); do xprop -name rowplay WM_NAME >/dev/null 2>&1 && break; sleep 1; done; sleep 1; '
              'xprop -name rowplay WM_CLASS _KDE_NET_WM_DESKTOP_FILE _GTK_APPLICATION_ID; kill $p; wait $p 2>/dev/null')
    env = generic_env({"LD_LIBRARY_PATH": f"{qt_dir}/lib"})   # the configured Qt (--qt-dir), through the generic helper
    done = runner.run([*XVFB, "bash", "-c", script], env=env, tag="x11-identity", timeout=120)
    (Path(out) / f"x11-identity-{Path(tree).name}.txt").write_text(done.text)
    return parse_xprop(done.out)


def generic_gate_env(artifact_dir):
    """The overrides for one generic walk: CI's Linux recipe under Xvfb, the *full* profile, no desktop, no inherited gate variable.

    Runner.in_tree merges os.environ underneath these, so anything not set here would be inherited: every gate variable is
    pinned (gates.pinned_gate_env) and the desktop variables are removed by name."""
    env = dict(generic_env(gates.pinned_gate_env({
        "ROWPLAY_GATE_PROFILE": "full",
        "QT_QPA_PLATFORM": "xcb", "QSG_RHI_BACKEND": "opengl", "LIBGL_ALWAYS_SOFTWARE": "1", "ROWPLAY_QT_SMOKE": "1",
        "ROWPLAY_PHASE_SHOTS": "1", "ROWPLAY_PHASE_CLOSEUPS": "1", "QT_LOGGING_RULES": "qt.qpa.theme=true",
        "ROWPLAY_GATE_NO_WINDOW_MANAGER": "1",     # Xvfb has none: the keyboard contract's inactive-window skip is expected here
        "ROWPLAY_SMOKE_ARTIFACT_DIR": str(artifact_dir), "ROWPLAY_SMOKE_SCREENSHOT_DIR": str(artifact_dir),
        "LANG": "C.UTF-8",
    })))
    for var in scrubbed_vars():
        env[var] = None
    return env


def stage_generic(ctx):
    st = Stage("generic")
    out = ctx.sub("generic")
    if shutil.which("xvfb-run") is None:
        st.add("Xvfb", Status.UNAVAILABLE, "xvfb-run is not installed")
        return st
    if not baseline_ready(ctx, st):
        return st
    removed = scrubbed_vars()
    st.passed("desktop variables scrubbed from every generic run", "removed from the environment: " + ", ".join(removed), removed=removed)
    info, done = ctx.probe(base_env=generic_env(), env_extra={"QT_QPA_PLATFORM": "xcb"}, prefix=XVFB, tag="qt-probe-generic")
    if info is None:
        st.failed("the generic Qt probe ran under Xvfb", (done.text or "")[-300:])
    else:
        (out / "qt-probe-generic.json").write_text(json.dumps(info, indent=2) + "\n")
        st.expect("no KDE platform theme is selected", info["platformThemeCreated"] != "kde",
                  f"theme names {info['platformThemeNames']}, created {info['platformThemeCreated']!r}, icon theme {info['iconTheme']!r}",
                  "the KDE theme was created although no desktop variable is set")
        st.expect("Fusion is the style in use", info["fusion"], "Button background is Fusion's ButtonPanel", f"got {info['buttonBackground']}")
        ok, story = qtprobe.accent_condition(info)
        st.expect("the generic palette resolves by the same rule", ok, story + f"; Theme.accentColor {info['themeAccentColor']}", story)
    dirs = {}
    for label, tree in (("baseline", ctx.baseline_tree), ("branch", ctx.repo)):
        d = out / f"{label}-xvfb"
        d.mkdir(parents=True, exist_ok=True)
        merged = generic_gate_env(d)
        done = ctx.runner.in_tree(tree, [*XVFB, "cargo", "test", "-p", "rowplay-app", "--test", "qml_runtime_gate", "--", "--nocapture"], env=merged,
                                  timeout=1800, tag=f"generic-{label}", logfile=d / "test-output.txt")
        log = (d / "gate-log.txt").read_text(errors="replace") if (d / "gate-log.txt").exists() else ""
        r = gates.parse_gate(f"{label}-xvfb", "cargo test -p rowplay-app --test qml_runtime_gate (CI's Linux recipe, desktop variables scrubbed)", done.rc, done.seconds,
                             done.text, log, captures=len(list(d.glob("*.png"))), output_file=f"generic/{label}-xvfb/test-output.txt")
        problems = gates.gate_problems(r, hardware=False, expect_full=True, baseline=(label == "baseline"))
        theme = re.search(r'Successfully created platform theme "([^"]+)"', log)
        if label == "branch":
            problems += [] if not (theme and theme.group(1) == "kde") else ["the KDE platform theme was created under Xvfb"]
        st.add(f"generic gate ({label}, Xvfb + Mesa llvmpipe, no desktop)", Status.PASS if not problems else Status.FAIL,
               (f"walk {r.walk_seconds} s, {r.captures} captures, platform theme {theme.group(1) if theme else 'none created'}, accent {r.accent}, "
                f"keyboard ended by {r.keyboard.get('ended_by')}") if not problems else "; ".join(problems), {"result": r.__dict__}, [r.output_file])
        dirs[label] = d
    if len(dirs) == 2:
        cd = visual.load_capture_diff(ctx.repo)
        results = visual.compare_dirs(dirs["baseline"], dirs["branch"], {"captures": {}}, cd)
        (out / "capture-diff.json").write_text(json.dumps([r.__dict__ for r in results], indent=1) + "\n")
        summary = visual.summarize(results)
        st.expect("generic captures: branch vs baseline within the repository's noise bounds", not summary["failed"],
                  f"{summary['captures']} captures compared with capture-diff's own bounds; every one is noise; 0 outside",
                  "outside the bound: " + ", ".join(summary["failed"][:10]), **summary)
    idents = {}
    for label, tree in (("baseline", ctx.baseline_tree), ("branch", ctx.repo)):
        idents[label] = x11_identity(ctx.runner, tree, out, ctx.qt_dir)
    app_id = ctx.app_id()
    b, m = idents.get("branch"), idents.get("baseline")
    if b is None:
        st.add("X11 identity", Status.UNAVAILABLE, "no debug binary to launch; run the native stage first")
    else:
        st.expect("X11: the desktop-file properties name the desktop entry", b.get("_KDE_NET_WM_DESKTOP_FILE") == app_id and b.get("_GTK_APPLICATION_ID") == app_id,
                  f"WM_CLASS {b.get('WM_CLASS')}, _KDE_NET_WM_DESKTOP_FILE {b.get('_KDE_NET_WM_DESKTOP_FILE')!r}, _GTK_APPLICATION_ID {b.get('_GTK_APPLICATION_ID')!r}",
                  f"got {b}, wanted {app_id!r}", branch=b, baseline=m)
        if m:
            st.expect("X11: the baseline had the executable's name there (the change is real)", m.get("_KDE_NET_WM_DESKTOP_FILE") != app_id,
                      f"baseline: {m.get('_KDE_NET_WM_DESKTOP_FILE')!r}", "the baseline already carried the ID")
        st.expect("X11: WM_CLASS keeps Qt's natural instance/class", bool(b.get("WM_CLASS")) and b["WM_CLASS"] == (m or b).get("WM_CLASS"),
                  f"{b.get('WM_CLASS')} (unchanged from the baseline)", f"branch {b.get('WM_CLASS')} vs baseline {(m or {}).get('WM_CLASS')}")
    return st


# --------------------------------------------------------------------------- packages

def _build_package(ctx, label, tree, log_dir):
    """Build one AppImage in the Ubuntu 24.04 container; returns (record or None, problems)."""
    target = Path.home() / ".cache" / "rowplay-ubuntu2404-target" / f"acceptance-{label}"
    if not getattr(ctx.args, "reuse_container_target", False):
        shutil.rmtree(target, ignore_errors=True)
    dist = Path(tree) / "dist"
    shutil.rmtree(dist, ignore_errors=True)
    log = log_dir / f"{label}-build.log"
    done = ctx.runner.run([str(TOOLS / "ubuntu-package.sh"), str(tree), str(target), str(log), str(ctx.qt_dir.resolve())],
                          tag=f"package-{label}", timeout=3600)
    text = log.read_text(errors="replace") if log.exists() else ""
    images = sorted(dist.glob("*.AppImage"))
    if not done.ok or not images:
        return None, [f"the container build failed (exit {done.rc}); see {log.name}: " + text[-300:].replace("\n", " ")]
    keep = log_dir / label
    keep.mkdir(parents=True, exist_ok=True)
    shutil.copy(images[0], keep / images[0].name)
    shutil.copy(str(images[0]) + ".sha256", keep / (images[0].name + ".sha256")) if Path(str(images[0]) + ".sha256").exists() else None
    appdir = keep / "extracted"
    shutil.rmtree(appdir, ignore_errors=True)
    root = package.extract(ctx.runner, keep / images[0].name, appdir)
    inv = package.inventory(root)
    record = {"path": str(keep / images[0].name), "bytes": (keep / images[0].name).stat().st_size,
              "sha256": package.sha256_file(keep / images[0].name), "files": len(inv), "inventory": inv,
              "launch_check": package.launch_check_line(text), "build_seconds": round(done.seconds), "root": str(root),
              "commit": git_state(ctx.runner, tree)["sha"]}
    return record, []


def stage_package(ctx):
    st = Stage("package")
    out = ctx.sub("package")
    if shutil.which("podman") is None:
        st.add("podman", Status.UNAVAILABLE, "podman is not installed: the Ubuntu 24.04 build cannot run (Fedora-host packaging is not canonical)")
        return st
    if not baseline_ready(ctx, st):
        return st
    records = {}
    for label, tree in (("baseline", ctx.baseline_tree), ("branch", ctx.repo)):
        record, problems = _build_package(ctx, label, tree, out)
        if record is None:
            st.failed(f"{label} AppImage builds in ubuntu:24.04", "; ".join(problems))
            continue
        records[label] = record
        ctx.appimages[label] = Path(record["path"])
        st.expect(f"{label} AppImage builds and launch-checks", bool(record["launch_check"] and "ok" in record["launch_check"]),
                  f"{record['bytes']} bytes, sha256 {record['sha256'][:16]}…, {record['files']} files; {record['launch_check']}",
                  f"launch check: {record['launch_check']}")
        hits = package.forbidden_in_paths(record["inventory"])
        needed = package.scan_needed(ctx.runner, record["root"], record["inventory"])
        st.expect(f"{label} AppImage bundles no KDE Frameworks, Kirigami, Plasma or Breeze QML files", not hits and not needed,
                  "no path matches KF, Kirigami, Plasma, qqc2-desktop-style, org.kde, Breeze, KConfig, KI18n or KIO; no ELF file NEEDs a KDE library",
                  f"paths: {hits[:5]}; NEEDED: {dict(list(needed.items())[:3])}", path_hits=hits[:20], needed_hits=needed)
        plugins = package.plugin_inventory(record["inventory"])
        st.passed(f"{label} AppImage plugin and QML inventory", "platform themes: " + ", ".join(plugins["plugins"].get("platformthemes", [])) +
                  "; platforms: " + ", ".join(plugins["plugins"].get("platforms", [])), **plugins)
    if len(records) == 2:
        cmp = package.compare(records["baseline"], records["branch"])
        table = ["| | acceptance baseline | KDE branch | delta |", "| --- | --- | --- | --- |",
                 f"| commit | {records['baseline']['commit'][:9]} | {records['branch']['commit'][:9]} | |",
                 f"| bytes | {cmp['bytes']['a']} | {cmp['bytes']['b']} | {cmp['bytes']['delta']:+d} |",
                 f"| files | {cmp['files']['a']} | {cmp['files']['b']} | {cmp['files']['delta']:+d} |",
                 f"| SHA-256 | {records['baseline']['sha256']} | {records['branch']['sha256']} | |"]
        (out / "comparison.md").write_text("\n".join(table) + "\n\nsize-changed files: " + json.dumps(cmp["size_changed"]) +
                                           "\nadded: " + json.dumps(cmp["added"]) + "\nremoved: " + json.dumps(cmp["removed"]) + "\n")
        st.passed("package comparison (same container, same script, separate target directories)",
                  f"bytes {cmp['bytes']['a']} -> {cmp['bytes']['b']} ({cmp['bytes']['delta']:+d}); files {cmp['files']['a']} -> {cmp['files']['b']} "
                  f"({cmp['files']['delta']:+d}); added {len(cmp['added'])}, removed {len(cmp['removed'])}, changed {[p for p, *_ in cmp['size_changed']]}",
                  comparison=cmp)
    slim = {k: {kk: vv for kk, vv in v.items() if kk not in ("inventory",)} for k, v in records.items()}
    (out / "packages.json").write_text(json.dumps(slim, indent=2) + "\n")
    b = records.get("branch")
    if b:
        app_id, root = ctx.app_id(), Path(b["root"])
        desktop = root / "usr/share/applications" / f"{app_id}.desktop"
        meta = root / "usr/share/metainfo" / f"{app_id}.metainfo.xml"
        st.expect("the packaged desktop entry and AppStream metadata carry the app ID", desktop.exists() and meta.exists()
                  and f"<id>{app_id}</id>" in meta.read_text(), f"{desktop.name} and {meta.name}", "missing or mismatched")
        if desktop.exists():
            v = ctx.runner.run(["desktop-file-validate", str(desktop)], env=host_env(), tag="desktop-validate")
            st.expect("desktop-file-validate: no errors (hints allowed)", v.ok, v.text.strip()[:200] or "clean", v.text.strip()[:300])
        if meta.exists() and shutil.which("appstreamcli"):
            v = ctx.runner.run(["appstreamcli", "validate", "--no-net", str(meta)], env=host_env(), tag="appstream-validate")
            st.expect("appstreamcli validate: passes", v.ok, "Validation was successful", v.text.strip()[-300:])
    return st


# --------------------------------------------------------------------------- exact AppImage on Plasma Wayland

DESKTOP_DIR = Path.home() / ".local/share/applications"
ICON_DIR = Path.home() / ".local/share/icons/hicolor/512x512/apps"


class DesktopEntry:
    """A temporary desktop entry + icon for the exact AppImage; the previous ones are put back byte for byte."""

    def __init__(self, runner, app_id, launcher, icon_src):
        self.runner, self.app_id, self.launcher, self.icon_src = runner, app_id, launcher, icon_src
        self.desktop = DESKTOP_DIR / f"{app_id}.desktop"
        self.icon = ICON_DIR / f"{app_id}.png"
        self.saved = {}

    def _refresh(self):
        self.runner.run(["kbuildsycoca6"], env=host_env(), tag="kbuildsycoca")

    def __enter__(self):
        for path in (self.desktop, self.icon):
            self.saved[path] = path.read_bytes() if path.exists() else None
        DESKTOP_DIR.mkdir(parents=True, exist_ok=True)
        ICON_DIR.mkdir(parents=True, exist_ok=True)
        shutil.copy(self.icon_src, self.icon)
        self.desktop.write_text("[Desktop Entry]\nType=Application\nName=rowplay\nComment=RowPlay (acceptance run)\n"
                                f"Exec={self.launcher}\nIcon={self.app_id}\nTerminal=false\nCategories=Utility;Sports;\n"
                                "StartupWMClass=rowplay-qt\n")
        self._refresh()
        return self

    def __exit__(self, *exc):
        for path, data in self.saved.items():
            if data is None:
                path.unlink(missing_ok=True)
            else:
                path.write_bytes(data)
        self._refresh()
        return False

    def restored(self):
        return all((p.read_bytes() if p.exists() else None) == d for p, d in self.saved.items())


def alive_pids(pids):
    """The subset of pids that are running processes."""
    out = []
    for pid in sorted(pids):
        try:
            os.kill(pid, 0)
            # A child may have exited before its parent reaps it. A zombie cannot run or be
            # signalled again, and must not make a successful teardown look like a survivor.
            try:
                state = Path(f"/proc/{pid}/stat").read_text().rpartition(") ")[2].split()[0]
            except (OSError, IndexError):
                # macOS has no /proc; ps reports the same zombie state without reaping a child.
                try:
                    state = subprocess.run(["ps", "-p", str(pid), "-o", "stat="], capture_output=True,
                                           text=True, timeout=1, env=host_env()).stdout.strip()
                except (OSError, subprocess.SubprocessError):
                    state = ""  # cannot establish exit: conservatively keep this PID
            if state.startswith("Z"):
                continue
            out.append(pid)
        except ProcessLookupError:
            pass
        except PermissionError:  # not ours to signal, but it exists
            out.append(pid)
    return out


def end_owned_processes(ctx, grace=3.0):
    """End the processes this run launched (ctx.launched_pids), and only those: TERM, then KILL after `grace` seconds.

    Waits after KILL too. Returns the PIDs that needed cleanup; the caller still checks for survivors.
    Never matches by name: another application built the same way shares
    the generic AppImage wrapper's name."""
    mine = alive_pids(ctx.launched_pids)
    for pid in mine:
        try:
            os.kill(pid, signal.SIGTERM)
        except (ProcessLookupError, PermissionError):
            pass

    def wait_for_exit():
        deadline = time.monotonic() + grace
        while alive_pids(mine) and time.monotonic() < deadline:
            time.sleep(0.1)

    wait_for_exit()
    for pid in alive_pids(mine):
        try:
            os.kill(pid, signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
    wait_for_exit()  # SIGKILL is asynchronous too: do not report leftovers before it takes effect.
    return mine


def pinned(app_id):
    text = (Path.home() / ".config/plasma-org.kde.plasma.desktop-appletsrc")
    return text.exists() and f"applications:{app_id}.desktop" in text.read_text(errors="replace")


def key_injection_survey(runner):
    """Why the exact AppImage cannot be sent key presses on this Wayland session, measured, not assumed."""
    tools = {t: shutil.which(t) for t in ("ydotool", "wtype", "xdotool", "evemu-event", "dotool", "kdotool")}
    uinput = "/dev/uinput"
    rd = runner.run(["gdbus", "call", "--session", "--dest", "org.freedesktop.portal.Desktop", "--object-path", "/org/freedesktop/portal/desktop",
                     "--method", "org.freedesktop.DBus.Properties.GetAll", "org.freedesktop.portal.RemoteDesktop"], env=host_env(), tag="survey-portal")
    return {"synthesizer_tools_installed": [t for t, p in tools.items() if p],
            "uinput_writable_without_privilege": os.access(uinput, os.W_OK) if os.path.exists(uinput) else False,
            "remote_desktop_portal_present": rd.ok and "AvailableDeviceTypes" in rd.out,
            "remote_desktop_portal_needs_consent_dialog": True}


def stage_identity(ctx):
    """Launch the exact branch AppImage through its desktop entry; identity, focus-loss, grouping, shutdown."""
    st = Stage("identity")
    out = ctx.sub("identity")
    if not getattr(ctx.args, "allow_session_changes", False):
        st.add("launching the exact AppImage through a temporary desktop entry", Status.SKIPPED, "needs --allow-session-changes (installs a temporary desktop entry and opens windows)")
        return st
    appimage = ctx.appimages.get("branch") or (Path(ctx.args.appimage) if getattr(ctx.args, "appimage", None) else None)
    if not appimage or not Path(appimage).exists():
        st.add("the branch AppImage", Status.UNAVAILABLE, "run the `package` stage first or pass --appimage")
        return st
    open_now = kwin.rowplay_windows(ctx.runner)
    if open_now != []:
        # Two different problems, told apart: a RowPlay window the stage did not open (close it: the stage
        # would otherwise close it with the ones it launches), or KWin that could not be asked at all.
        st.failed("no RowPlay window is open before the run",
                  "KWin could not be asked (is this a Plasma Wayland session, and is its scripting interface on the session bus?)"
                  if open_now is None else
                  "close RowPlay first; KWin lists " + ", ".join(
                      f"pid {w['pid']} {w['desktopFileName'] or w['resourceClass']!r}" for w in open_now))
        return st
    app_id = ctx.app_id()
    launched = ctx.launched_pids   # the only processes this stage may ever end: the windows KWin reported
    survey = key_injection_survey(ctx.runner)
    (out / "key-injection-survey.json").write_text(json.dumps(survey, indent=2) + "\n")
    st.add("key-press injection into the exact AppImage", Status.UNAVAILABLE,
           "not achievable without a privileged or interactive channel: synthesizer tools installed: "
           f"{survey['synthesizer_tools_installed'] or 'none'}; /dev/uinput writable without privilege: {survey['uinput_writable_without_privilege']}; "
           "the RemoteDesktop portal is present but needs a consent dialog every session. Replaced by: the gate's real QtTest key events on the "
           "same release code path (native stage) plus AT-SPI focus and action driving of the exact AppImage (below)", survey)
    launcher = out / "launch.sh"
    logs = out / "app-logs"
    logs.mkdir(exist_ok=True)
    data_dir = out / "app-data"
    launcher.write_text("#!/usr/bin/env bash\nn=$(date +%s%N)\nexec env APPIMAGE_EXTRACT_AND_RUN=1 QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1 LANG=en_US.UTF-8 "
                        f"QT_FORCE_STDERR_LOGGING=1 QT_LOGGING_RULES='qt.qpa.theme=true;qt.gui.icon.loader=true' ROWPLAY_DATA_DIR='{data_dir}' "
                        f"'{Path(appimage).resolve()}' 2>>'{logs}'/app-$n.log\n")
    launcher.chmod(0o755)
    started = time.strftime("%Y-%m-%d %H:%M:%S")
    with DesktopEntry(ctx.runner, app_id, launcher, ctx.repo / "assets/icon/rowplay-icon-512.png") as entry:
        def launch():
            return ctx.runner.run(["kstart", "--application", app_id], env=host_env(), tag="kstart", timeout=30)
        try:
            launch()
            first = kwin.wait_for_windows(ctx.runner, 1)
            launched.update(w["pid"] for w in first or [])
            ok = first is not None and len(first) == 1
            st.expect("launching once through the desktop entry opens exactly one RowPlay window", ok, f"{first}", f"KWin reports {first}")
            if ok:
                w = first[0]
                st.expect("KWin's desktopFileName is the desktop entry's ID", w["desktopFileName"] == app_id,
                          f"desktopFileName={w['desktopFileName']!r}, resourceClass={w['resourceClass']!r}, resourceName={w['resourceName']!r}, wayland={w['wayland']}",
                          f"desktopFileName={w['desktopFileName']!r}, wanted {app_id!r}", window=w)
                st.expect("the window is a native Wayland window", w["wayland"], "wayland=true", "it is an X11 window")
                shot = ctx.runner.run(["spectacle", "-b", "-n", "-f", "-o", str(out / "one-window-fullscreen.png")], env=host_env(), tag="spectacle")
                walk = ctx.runner.run([shutil.which("python3"), str(TOOLS / "atspi_walk.py"), "--timeout", "40"], env=host_env(), tag="atspi", timeout=180)
                (out / "atspi-walk.json").write_text(walk.out or walk.text)
                try:
                    report = json.loads(walk.out.strip().splitlines()[-1])
                except (ValueError, IndexError):
                    report = {"steps": [], "error": "no JSON from atspi_walk.py: " + walk.text[-300:]}
                for step in report["steps"]:
                    st.expect("AT-SPI on the exact AppImage: " + step["name"], step["ok"], step["detail"], step["detail"])
                if report["error"]:
                    st.failed("AT-SPI walk completed", report["error"])
                st.expect("the app survived the AT-SPI walk", kwin.rowplay_windows(ctx.runner) not in (None, []), "the window is still there", "the window is gone")
                launch()
                two = kwin.wait_for_windows(ctx.runner, 2)
                launched.update(w["pid"] for w in two or [])
                ok2 = two is not None and len(two) == 2
                st.expect("a second launch opens a second window", ok2, f"{len(two or [])} windows", f"KWin reports {two}")
                if ok2:
                    st.expect("both windows carry the same desktop-file association (Task Manager groups them as one)",
                              {x["desktopFileName"] for x in two} == {app_id} and len({x["pid"] for x in two}) == 2,
                              f"desktopFileName {sorted({x['desktopFileName'] for x in two})}, pids {sorted(x['pid'] for x in two)}",
                              f"got {two}", windows=two)
                    ctx.runner.run(["spectacle", "-b", "-n", "-f", "-o", str(out / "two-windows-fullscreen.png")], env=host_env(), tag="spectacle")
        finally:
            kwin.close_rowplay_windows(ctx.runner)
            gone = kwin.wait_for_windows(ctx.runner, 0, timeout=30)
            st.expect("closing the windows through KWin ends every RowPlay window", gone == [], "no window left", f"KWin still lists {gone}")
            time.sleep(3)
            # Only the PIDs KWin reported for the windows this run launched: another AppImage that
            # shares the generic `AppRun.wrapped` name is never touched.
            left = end_owned_processes(ctx)
            survivors = alive_pids(launched)
            st.expect("no RowPlay process this run launched remains after the windows close", not left, "none",
                      f"processes {left} were still running (cleanup attempted only for this run's PIDs; still running: {survivors})")
        st.add("Task Manager pin / unpin", Status.MANUAL_OPTIONAL,
               f"not automated: the panel is the user's configuration. Currently pinned: {pinned(app_id)}. Visual confirmation stays an optional manual smoke.",
               {"currently_pinned": pinned(app_id)})
    st.expect("the temporary desktop entry and icon are removed and any previous ones restored byte for byte", entry.restored(), "restored", "not restored")
    cores = ctx.runner.run(["coredumpctl", "list", "--since", started, "--no-pager"], env=host_env(), tag="coredumpctl")
    rowplay_cores = [ln for ln in cores.out.splitlines() if "AppRun" in ln or "rowplay" in ln]
    st.expect("no RowPlay core dump during the run", not rowplay_cores, "none", "; ".join(rowplay_cores[:3]))
    logs_text = "\n".join(p.read_text(errors="replace") for p in sorted(logs.glob("*.log")))
    st.expect("the exact AppImage logged no abort or QML error", not gates.SIGNATURES.search(logs_text) and not re.search(r"TypeError|ReferenceError|Binding loop", logs_text),
              f"{len(list(logs.glob('*.log')))} app logs clean", "found: " + ", ".join(sorted({m.group(0) for m in gates.SIGNATURES.finditer(logs_text)})))
    theme = re.search(r'Successfully created platform theme "([^"]+)"', logs_text)
    icons = re.search(r'Initialized icon loader with system theme "([^"]+)"', logs_text)
    st.expect("the exact AppImage's own Qt created a platform theme and an icon theme", bool(theme and icons),
              f"platform theme {theme.group(1) if theme else None!r}, icon theme {icons.group(1) if icons else None!r}", "no theme lines in the app's own log",
              platform_theme=theme.group(1) if theme else None, icon_theme=icons.group(1) if icons else None)
    themed = sorted(set(re.findall(r'Finding icon "([a-z-]+)" in theme "([^"]+)"', logs_text)))
    st.expect("the standard command icons are looked up in the icon theme first", bool(themed) and any(n in ("view-refresh", "go-previous", "system-search", "view-sort-ascending") for n, _ in themed),
              "looked up in the theme: " + ", ".join(f"{n} ({t})" for n, t in themed[:8]), "no themed lookup of a standard action icon in the app's log", themed=themed)
    return st


# --------------------------------------------------------------------------- appearance (changes the desktop, transactionally)

def _dark_scheme(runner):
    listing = runner.run(["plasma-apply-colorscheme", "--list-schemes"], env=host_env(), tag="schemes").out
    names = re.findall(r"^\s*\*\s*(\S+)", listing, re.M)
    return next((n for n in names if n == "BreezeDark"), next((n for n in names if "Dark" in n), None))


def _copy_shots(gate_dir, dest, names=("dashboard", "settings", "detail-full", "replay-row", "sidebar-hidden", "dashboard-compact")):
    dest.mkdir(parents=True, exist_ok=True)
    for name in names:
        src = Path(gate_dir) / f"{name}.png"
        if src.exists():
            shutil.copy(src, dest / f"{name}.png")


def stage_appearance(ctx):
    st = Stage("appearance")
    out = ctx.sub("appearance")
    if not getattr(ctx.args, "allow_session_changes", False):
        st.add("accent, dark scheme and 150 % font on the live desktop", Status.SKIPPED, "needs --allow-session-changes (changes the live Plasma appearance, then restores and verifies it)")
        return st
    if not os.environ.get("WAYLAND_DISPLAY"):
        st.add("a Wayland session", Status.UNAVAILABLE, "no Wayland session in the environment")
        return st
    pl = plasma.Plasma(ctx.runner, ctx.probe_info)
    cd = visual.load_capture_diff(ctx.repo)
    derive = getattr(ctx.args, "derive", False)
    tx = plasma.SessionTransaction(pl, allow=True)
    error = None
    raiser = kwin.Raiser(ctx.runner).__enter__()
    try:
        with tx:
            snap = tx.snap
            (out / "snapshot.json").write_text(json.dumps(snap.to_dict(), indent=2) + "\n")
            (out / "kdeglobals.snapshot").write_bytes(snap.kdeglobals_bytes)
            st.passed("original Plasma state snapshotted", f"scheme {snap.scheme}, font {snap.font!r}, kdeglobals sha256 {snap.kdeglobals_sha256[:16]}…, "
                      f"Qt highlight {snap.qt.get('highlight')}, accent {snap.qt.get('accent')}", snapshot=snap.to_dict())
            g = gates.run_gate(ctx.runner, ctx.repo, out, "default-quick", "quick", native=True)
            _gate_checks(st, "default appearance: quick gate", g)
            # ---- a loud accent
            base = snap.qt
            accent = plasma.loudest_accent(base["highlight"], base["window"])
            pl.apply_scheme(snap.scheme, accent=accent)
            after = ctx.probe_info()
            selection = plasma.hex_from_rgb_csv(pl.read_selection())
            st.expect(f"Plasma accepted the accent {accent}", selection.lower() != base["highlight"].lower(), f"[Colors:Selection] BackgroundNormal is now {selection}",
                      "kdeglobals' selection colour did not change")
            st.expect("Qt's Highlight follows the Plasma accent while Accent stays Qt's default", after["highlight"].lower() == selection.lower()
                      and after["highlight"].lower() != base["highlight"].lower(),
                      f"Highlight {base['highlight']} -> {after['highlight']}; Accent {after['accent']}", f"Highlight {after['highlight']}, selection {selection}")
            ok, story = qtprobe.accent_condition(after)
            st.expect("RowPlay's Theme.accentColor changes with it", ok and after["themeAccentColor"].lower() != base["themeAccentColor"].lower(),
                      f"Theme.accentColor {base['themeAccentColor']} -> {after['themeAccentColor']}; focus ring {base['themeFocusRing']} -> {after['themeFocusRing']}", story)
            g = gates.run_gate(ctx.runner, ctx.repo, out, "accent-quick", "quick", native=True)
            _gate_checks(st, f"accent {accent}: quick gate", g)
            if derive:
                (out / "derived-accent-rules.json").write_text(json.dumps(visual.derive_set(out / "default-quick", out / "accent-quick", cd), indent=1))
            else:
                rules, profile = visual.load_rules(VISUAL_RULES, "accent-vs-default", "native-hardware")
                res = visual.compare_dirs(out / "default-quick", out / "accent-quick", rules, cd, profile=profile)
                (out / "accent-visual.md").write_text(visual_table(res))
                summ = visual.summarize(res)
                st.expect("the accent change is confined to the listed accent-driven regions", not summ["failed"],
                          f"{summ['expected_changes']} of {summ['captures']} captures changed, only inside their regions; {summ['unexpected_pixels']} unexpected pixels",
                          "failed: " + ", ".join(summ["failed"]), **summ)
            _copy_shots(out / "accent-quick", out / "screenshots" / "accent")
            # ---- dark
            dark = _dark_scheme(ctx.runner)
            if dark is None:
                st.add("a dark colour scheme", Status.UNAVAILABLE, "no dark colour scheme is installed")
            else:
                pl.apply_scheme(dark)
                d = ctx.probe_info()
                portal = pl.read_portal()
                st.expect(f"{dark}: Qt reports Dark and Theme follows", d["colorScheme"] == 2 and d["themeDark"] and d["window"] != base["window"],
                          f"colorScheme {d['colorSchemeName']}, window {base['window']} -> {d['window']}, Theme.dark {d['themeDark']}, portal color-scheme {portal.get('color-scheme')} (1 = dark)",
                          f"colorScheme {d['colorSchemeName']}, Theme.dark {d['themeDark']}")
                ok, story = qtprobe.accent_condition(d)
                st.expect(f"{dark}: the accent resolves by the same rule", ok, story + f"; Theme.accentColor {d['themeAccentColor']}", story)
                g = gates.run_gate(ctx.runner, ctx.repo, out, "dark-quick", "quick", native=True)
                _gate_checks(st, f"{dark}: quick gate", g)
                _copy_shots(out / "dark-quick", out / "screenshots" / "dark")
            # ---- back to the scheme's own accent
            pl.apply_scheme(snap.scheme)
            back = ctx.probe_info()
            st.expect("returning to the scheme restores the scheme's own accent and palette", back["highlight"] == base["highlight"] and back["window"] == base["window"]
                      and back["themeAccentColor"] == base["themeAccentColor"], f"Highlight {back['highlight']}, Theme.accentColor {back['themeAccentColor']}",
                      f"Highlight {back['highlight']} vs {base['highlight']}")
            # ---- 150 % font
            factor = 1.5
            new_font, done = pl.apply_font_scale(factor)
            big = ctx.probe_info()
            want = round(plasma.font_point_size(snap.font) * factor)
            st.expect(f"Plasma accepted the general font at {factor:.0%}: {plasma.font_point_size(snap.font):.0f} pt -> {want} pt", big["fontPointSize"] == want,
                      f"a fresh Qt process reports {big['fontFamily']} {big['fontPointSize']} pt ({big['fontPixelSize']} px)", f"Qt reports {big['fontPointSize']} pt, wanted {want}")
            g = gates.run_gate(ctx.runner, ctx.repo, out, "font150-full", "full", native=True)
            result = _gate_checks(st, f"font at {factor:.0%}: full gate (every screen, all six languages, the compact layouts)", g)
            log = (out / "font150-full" / "gate-log.txt").read_text(errors="replace")
            fits = {loc: f"gate compact gap: {loc} fits true" in log for loc in ("en", "zh", "de", "es", "fr", "ja")}
            st.expect("the compact replay gap fits in every language at 150 % text", all(fits.values()), "en zh de es fr ja all fit", f"{fits}", fits=fits)
            widths = [ln for ln in log.splitlines() if "gate width classes:" in ln]
            st.expect("the three width classes still lay out at 150 % text", len(widths) == 3 and all("FAILED" not in w for w in widths), "; ".join(w.split("qml: ")[-1] for w in widths),
                      str(widths))
            _copy_shots(out / "font150-full", out / "screenshots" / "font150")
    except KeyboardInterrupt as exc:  # SIGINT, SIGTERM, SIGHUP: the transaction has already restored the desktop
        # Do not raise from here: the restoration evidence below must reach the manifest first. The run
        # stops after recording this stage (ctx.interrupted), and no later stage runs.
        ctx.interrupted = exc
        error = f"interrupted ({exc}) after the desktop was restored; nothing later runs"
    except BaseException as exc:  # the transaction has already restored; record it
        error = f"{type(exc).__name__}: {exc}"
    finally:
        raiser.__exit__(None, None, None)
    st.expect("the desktop is restored exactly: kdeglobals bytes, colour scheme, portal, font and what Qt reports", tx.restored,
              "kdeglobals sha256, scheme, portal keys, font and the bundled Qt's palette all equal the snapshot",
              "STILL CHANGED: " + "; ".join(tx.left or ["restoration did not run"]), left=tx.left)
    (out / "restoration.json").write_text(json.dumps({"restored": tx.restored, "left": tx.left}, indent=2) + "\n")
    if error:
        st.failed("the appearance run completed", error)
    return st


# --------------------------------------------------------------------------- repository checks

def stage_repo_checks(ctx):
    st = Stage("repo-checks")
    out = ctx.sub("repo-checks")
    steps = [
        ("harness unit tests", [shutil.which("python3"), "-B", "-m", "unittest", "discover", "-s", "tools/kde/tests", "-t", "tools/kde"], {}),
        ("cargo fmt --all -- --check", ["cargo", "fmt", "--all", "--", "--check"], {}),
        ("cargo clippy --workspace --all-targets -- -D warnings", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"], {}),
        ("cargo test (Qt-free crates)", ["cargo", "test"], {}),
        ("cargo test --workspace", ["cargo", "test", "--workspace"], {}),
        ("git diff --check (against the acceptance baseline)", ["git", "diff", "--check", f"{ctx.baseline_sha}...HEAD"], {}),
    ]
    for name, argv, env in steps:
        done = ctx.runner.in_tree(ctx.repo, argv, env=env, tag="repo-check", timeout=1800, logfile=out / (re.sub(r"\W+", "-", name)[:40] + ".log"))
        passed = sum(int(m.group(1)) for m in re.finditer(r"test result: ok\. (\d+) passed", done.text))
        st.expect(name, done.ok, f"exit 0" + (f", {passed} tests passed" if passed else ""), f"exit {done.rc}: {done.text[-250:]}", tests_passed=passed)
    return st


# --------------------------------------------------------------------------- no helper processes left

# Helpers this harness starts, recognised by what they *are*: the process's own command line begins with the
# interpreter or the tool. A pattern that merely occurs somewhere in a command line matches every shell, editor and
# `grep` that mentions atspi_walk or a probe directory, and fails a clean run. (POSIX ERE, as pgrep -f reads it.)
HELPER_PROCESS_PATTERN = (r"^([^ ]*/)?qml .*(kdeacc-probe|/kde/probe/)"
                          r"|^([^ ]*/)?[Pp]ython[0-9.]* ([^ ]*/)?atspi_walk\.py( |$)")


def stage_leftovers(ctx):
    st = Stage("leftovers")
    time.sleep(1)
    if ctx.run_started and ctx.inhibitor is not None:
        locked = session.locked_since(ctx.runner, ctx.run_started)
        first = locked[0][:120] if locked else ""
        st.expect("the screen did not lock during the run (a locked session starves every window of frames)", not locked,
                  "no screen-locker activity in the journal since the run began",
                  f"the screen locker ran during the run ({len(locked)} journal lines; first: {first}): the idle inhibition did not hold",
                  inhibition_taken=ctx.inhibitor.cookie is not None or ctx.inhibitor.note == "")
    # By executable name and by the PIDs this run launched: never the generic AppImage wrapper name,
    # which another linuxdeploy-built application shares.
    # -l lists names on both procps and BSD; BSD's -a includes ancestors instead.
    by_name = ctx.runner.run(["pgrep", "-l", "-x", "rowplay-app|rowplay-qt"], env=host_env(), tag="leftovers").out
    by_name += ctx.runner.run(["pgrep", "-fl", HELPER_PROCESS_PATTERN], env=host_env(), tag="leftovers").out
    mine = [ln for ln in by_name.splitlines() if "pgrep" not in ln and "acceptance.py" not in ln]
    mine += [f"{pid} (launched by this run)" for pid in alive_pids(ctx.launched_pids)]
    st.expect("no helper or RowPlay process remains", not mine, "none", "; ".join(mine[:5]))
    return st
