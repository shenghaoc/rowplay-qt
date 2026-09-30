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
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path

from . import baseline, host, qtprobe, services, session
from .results import Stage, Status
from .shell import Runner, bundled_qt_env, generic_env, host_env, scrubbed_vars

TOOLS = Path(__file__).resolve().parent.parent
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
    st.expect("the gate's chord parser reads every platform's shortcut strings (Linux, Windows, macOS glyphs)",
              bool(summary) and summary.group(1) == summary.group(2), summary.group(0) if summary else "",
              "the chord parser is wrong for: " + "; ".join(l.split("qml: ")[-1] for l in chords.text.splitlines() if "CHORD FAIL" in l) or "no summary line")
    limitation = info["contrast"] == 0
    st.passed("contrast preference", ("Qt reports NoPreference: a platform limitation, high contrast engages only when Qt reports it; "
                                      "nothing is inferred from the palette"
                                      if limitation else f"Qt reports {info['contrastName']}: RowPlay's high-contrast variant engages natively"),
              qt_contrast=info["contrastName"], limitation=limitation)
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
    if not all(check.ok for check in checks):
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

def alive_pids(pids):
    """The subset of pids that are running processes."""
    out = []
    for pid in sorted(pids):
        try:
            os.kill(pid, 0)
            out.append(pid)
        except ProcessLookupError:
            pass
        except PermissionError:  # not ours to signal, but it exists
            out.append(pid)
    return out


# Helpers this harness starts, recognised by what they *are*: the process's own command line begins with the
# interpreter or the tool. A pattern that merely occurs somewhere in a command line matches every shell, editor and
# `grep` that mentions atspi_walk or a probe directory, and fails a clean run. (POSIX ERE, as pgrep -f reads it.)
HELPER_PROCESS_PATTERN = (r"^([^ ]*/)?qml .*(kdeacc-probe|/kde/probe/)"
                          r"|^([^ ]*/)?python[0-9.]* ([^ ]*/)?atspi_walk\.py( |$)")


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
    by_name = ctx.runner.run(["pgrep", "-a", "-x", "rowplay-app|rowplay-qt"], env=host_env(), tag="leftovers").out
    by_name += ctx.runner.run(["pgrep", "-af", HELPER_PROCESS_PATTERN], env=host_env(), tag="leftovers").out
    mine = [ln for ln in by_name.splitlines() if "pgrep" not in ln and "acceptance.py" not in ln]
    mine += [f"{pid} (launched by this run)" for pid in alive_pids(ctx.launched_pids)]
    st.expect("no helper or RowPlay process remains", not mine, "none", "; ".join(mine[:5]))
    return st
