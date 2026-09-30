#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Fedora KDE Plasma native acceptance for rowplay-qt.

Runs the checks that make the Plasma integration (ADR 0018) repeatable: the host and Qt probe, the
repository's own native gates, generic Linux, the AppImage packages,
the exact AppImage on Plasma Wayland, and appearance changes with verified restoration. Evidence goes to
one self-contained directory: manifest.json (the source of truth), summary.md, host.json, commands.log.

Modes (stages), any number of them:
  probe    host record, the bundled Qt's platform theme, palette, accent, contrast
  guards   clean trees, #143's containment, no KDE dependency, nothing else touched
  services URL opening, file chooser, notifications, tray, menu, MPRIS, secret store: from the source
  native   debug quick, release quick and full hardware-GL gates (branch), full gate (the acceptance baseline)
  generic  Xvfb + Fusion with every desktop variable removed; X11 identity properties
  package  both AppImages in ubuntu:24.04, inventory, comparison, KDE-bundle scan
  identity the exact AppImage through a desktop entry: KWin identity, AT-SPI focus walk, grouping (--allow-session-changes)
  appearance accent, dark, 150 % font, restore and verify (--allow-session-changes)
  checks   harness tests, fmt, clippy, cargo test, cargo test --workspace, git diff --check
  all      everything above, in dependency order

Nothing changes the desktop without --allow-session-changes. --dry-run prints the plan and runs nothing.
Exit status is nonzero when any check FAILED. See tools/kde/README.md.
"""

import argparse
import os
import signal
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

from kdeacc import baseline, session, stages  # noqa: E402
from kdeacc.results import Run, Stage, Status, now_iso, render_summary, write_manifest  # noqa: E402
from kdeacc.shell import Runner, scrubbed_vars  # noqa: E402

ORDER = ["probe", "guards", "services", "native", "generic", "package", "identity", "appearance", "checks"]
FUNCS = {"probe": stages.stage_probe, "guards": stages.stage_guards, "services": stages.stage_services, "native": stages.stage_native,
         "generic": stages.stage_generic, "package": stages.stage_package,
         "identity": stages.stage_identity, "appearance": stages.stage_appearance, "checks": stages.stage_repo_checks}
NEEDS_SESSION = {"identity", "appearance"}
DEPENDS = {"identity": ["package"]}


def parse(argv):
    p = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0], epilog="See the module docstring (--help of the file) and tools/kde/README.md.",
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("stages", nargs="*", default=["all"], help="stages to run: " + " ".join(ORDER) + " all (default: all)")
    root = Path(__file__).resolve().parents[2]
    p.add_argument("--repo", type=Path, default=root, help="the worktree under test (default: this checkout)")
    p.add_argument("--baseline-tree", "--main-tree", dest="baseline_tree", type=Path,
                   help="a checkout of the acceptance baseline commit (see --baseline-sha; a detached worktree is fine, its branch name "
                        "does not matter; default: a worktree already at that commit). --main-tree is the old name of this option.")
    p.add_argument("--baseline-sha", default=baseline.BASELINE_SHA,
                   help=f"the commit the integration is compared with (default {baseline.BASELINE_SHA}: {baseline.BASELINE_NOTE}). "
                        "Not `main`, which moves once the stack merges.")
    p.add_argument("--qt-dir", type=Path, default=Path.home() / "Qt/6.11.2/gcc_64", help="the repository's Qt (README: aqt linux_gcc_64)")
    p.add_argument("--output", type=Path, help="evidence directory (default: <repo>/artifacts/kde/acceptance-<timestamp>)")
    p.add_argument("--dry-run", action="store_true", help="print the plan and the commands' shape; run nothing and change nothing")
    p.add_argument("--allow-session-changes", action="store_true", help="permit stages that change the live Plasma session (identity, appearance)")
    p.add_argument("--appimage", type=Path, help="use this branch AppImage for `identity` instead of building one")
    p.add_argument("--reuse-container-target", action="store_true", help="package: keep the container build's target directory (faster; not a cold build)")
    return p.parse_args(argv)


def plan(names):
    wanted = ORDER if "all" in names else [n for n in ORDER if n in names]
    unknown = [n for n in names if n != "all" and n not in ORDER]
    if unknown:
        raise SystemExit(f"unknown stage(s): {', '.join(unknown)}; choose from {' '.join(ORDER)} all")
    return wanted


NEEDS_BASELINE = {"guards", "native", "generic", "package"}   # the stages that run or compare the acceptance baseline checkout


def main(argv=None):
    args = parse(argv if argv is not None else sys.argv[1:])
    names = plan(args.stages)
    repo = args.repo.resolve()
    stamp = time.strftime("%Y%m%d-%H%M%S")
    out = (args.output or repo / "artifacts" / "kde" / f"acceptance-{stamp}").resolve()
    needs_baseline = any(n in NEEDS_BASELINE for n in names)
    print(f"plan: {' '.join(names)}\nrepo: {repo}\nevidence: {out}")
    session_stages = [n for n in names if n in NEEDS_SESSION]
    if args.dry_run:
        print("dry run: nothing is run or changed.")
        # Nothing is resolved or validated here: a plan must be printable from any checkout.
        print("baseline: " + (f"would be required by {', '.join(n for n in names if n in NEEDS_BASELINE)}: "
                              f"{baseline.describe_requirement(args.baseline_sha)}" if needs_baseline else "not required by these stages"))
        for n in names:
            note = " (would change the live session; needs --allow-session-changes)" if n in NEEDS_SESSION else ""
            print(f"  - {n}{note}")
        return 0
    baseline_tree = None
    if needs_baseline:
        baseline_tree = args.baseline_tree or baseline.find_baseline_tree(Runner(), repo, args.baseline_sha)
        if baseline_tree is None:
            raise SystemExit(f"no acceptance baseline checkout: {baseline.describe_requirement(args.baseline_sha)}")
    out.mkdir(parents=True, exist_ok=True)
    for sub in ("native", "generic", "package", "appearance", "identity"):
        (out / sub).mkdir(exist_ok=True)
    runner = Runner(out / "commands.log")
    ctx = stages.Ctx(repo, Path(baseline_tree).resolve() if baseline_tree else None, args.qt_dir, out, runner, args,
                     baseline_sha=args.baseline_sha)
    run = Run(started=now_iso(), argv=sys.argv)
    run.context = {"harness": "tools/kde/acceptance.py", "scrubbed_desktop_variables_in_generic_runs": ", ".join(scrubbed_vars()),
                   "session_changes_allowed": args.allow_session_changes}

    def finish():
        run.finished = now_iso()
        run.context.update({f"{k}_sha": v["sha"] for k, v in ctx.shas.items()})
        run.context.update({f"{k}_dirty": v["dirty"] for k, v in ctx.shas.items()})
        run.context.update({"branch": ctx.shas.get("branch", {}).get("branch"), "kde_tree": str(repo), "baseline_tree": str(ctx.baseline_tree) if ctx.baseline_tree else None,
                            "baseline_sha_expected": ctx.baseline_sha})
        write_manifest(run, out / "manifest.json")
        (out / "summary.md").write_text(render_summary(run))

    def signals_ignored():
        for sig in session.INTERRUPT_SIGNALS:
            signal.signal(sig, signal.SIG_IGN)

    for sig in session.INTERRUPT_SIGNALS:   # SIGINT, SIGTERM and SIGHUP (a closed terminal): all end the run the same way
        signal.signal(sig, session.raise_interrupted)
    ctx.run_started = time.time()
    inhibitor = session.IdleInhibitor().__enter__()
    ctx.inhibitor = inhibitor
    try:
        # guards first: it records the SHAs every other stage's context wants
        order = ["guards"] + [n for n in names if n != "guards"] if "guards" in names or "all" in args.stages else names
        for name in order:
            if name in DEPENDS and not all(d in names for d in DEPENDS[name]):
                need = [d for d in DEPENDS[name] if d not in names]
                if not (name == "identity" and args.appimage):
                    print(f"note: {name} needs {' '.join(need)}; pass those stages or the matching --appimage option")
            print(f"== {name}", flush=True)
            started = time.time()
            try:
                stage = FUNCS[name](ctx)
            except KeyboardInterrupt:
                stage = Stage(name, error="interrupted")
                run.stages.append(stage)
                stage.seconds = time.time() - started
                raise
            except Exception as exc:
                import traceback
                stage = Stage(name, error=f"{type(exc).__name__}: {exc}\n{traceback.format_exc()[-1200:]}")
            stage.seconds = time.time() - started
            run.stages.append(stage)
            print(f"   {stage.status.value} ({len(stage.checks)} checks, {stage.seconds:.0f} s)", flush=True)
            for c in stage.checks:
                if c.status == Status.FAIL:
                    print(f"   FAIL {c.name}: {c.detail}")
            write_manifest(run, out / "manifest.json")
            if ctx.interrupted is not None:
                # A stage caught the interruption to finish its own evidence (the appearance stage's restoration).
                # That is now recorded: stop here, exactly as if the signal had arrived between stages.
                raise ctx.interrupted
    except KeyboardInterrupt as exc:
        run.interrupted = str(exc)
        run.interrupt_signal = getattr(exc, "signum", 0)
        print(f"interrupted: {exc}: no further stage runs", file=sys.stderr)
        # No stage runs now; the desktop was restored by its own transaction. End what this run launched.
        signals_ignored()
        ended = stages.end_owned_processes(ctx)
        if ended:
            print(f"ended the processes this run launched: {ended}", file=sys.stderr)
    signals_ignored()   # the last check and the manifest are not interruptible: a second signal must not lose them
    try:
        left = stages.stage_leftovers(ctx)   # after an interruption too: nothing this run started may survive it
    except Exception as exc:  # recorded like any stage's error: never a crash that skips the manifest
        left = Stage("leftovers", error=f"{type(exc).__name__}: {exc}")
    run.stages.append(left)
    inhibitor.__exit__(None, None, None)
    finish()
    print(f"\n{'INTERRUPTED' if run.interrupted else 'FAILED' if run.failed else 'PASSED'}: {out}/summary.md")
    return run.exit_code()


if __name__ == "__main__":
    sys.exit(main())
