#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Fedora KDE Plasma native acceptance for rowplay-qt.

Runs the checks that make the Plasma integration (ADR 0018) repeatable: the host and Qt probe, the
repository guards, the services audit and the repository's own checks. The native, package, identity and
appearance stages arrive in later layers of the stack. Evidence goes to
one self-contained directory: manifest.json (the source of truth), summary.md, host.json, commands.log.

Modes (stages), any number of them:
  probe    host record, the bundled Qt's platform theme, palette, accent, contrast
  guards   clean trees, #143's containment, no KDE dependency, nothing else touched
  services URL opening, file chooser, notifications, tray, menu, MPRIS, secret store: from the source
  checks   harness tests, fmt, clippy, cargo test, cargo test --workspace, git diff --check
  all      everything above, in dependency order

Nothing here changes the desktop. --dry-run prints the plan and runs nothing.
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

from kdeacc import session, stages  # noqa: E402
from kdeacc.results import Run, Stage, Status, now_iso, render_summary, write_manifest  # noqa: E402
from kdeacc.shell import Runner, scrubbed_vars  # noqa: E402

ORDER = ["probe", "guards", "services", "checks"]
FUNCS = {"probe": stages.stage_probe, "guards": stages.stage_guards, "services": stages.stage_services, "checks": stages.stage_repo_checks}



def parse(argv):
    p = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0], epilog="See the module docstring (--help of the file) and tools/kde/README.md.",
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("stages", nargs="*", default=["all"], help="stages to run: " + " ".join(ORDER) + " all (default: all)")
    root = Path(__file__).resolve().parents[2]
    p.add_argument("--repo", type=Path, default=root, help="the worktree under test (default: this checkout)")
    p.add_argument("--main-tree", type=Path, help="a checkout of post-#144 main, the baseline (default: the worktree on branch main)")
    p.add_argument("--qt-dir", type=Path, default=Path.home() / "Qt/6.11.2/gcc_64", help="the repository's Qt (README: aqt linux_gcc_64)")
    p.add_argument("--output", type=Path, help="evidence directory (default: <repo>/artifacts/kde/acceptance-<timestamp>)")
    p.add_argument("--dry-run", action="store_true", help="print the plan and the commands' shape; run nothing and change nothing")
    return p.parse_args(argv)


def plan(names):
    wanted = ORDER if "all" in names else [n for n in ORDER if n in names]
    unknown = [n for n in names if n != "all" and n not in ORDER]
    if unknown:
        raise SystemExit(f"unknown stage(s): {', '.join(unknown)}; choose from {' '.join(ORDER)} all")
    return wanted


def main_tree_of(repo):
    import subprocess
    out = subprocess.run(["git", "-C", str(repo), "worktree", "list", "--porcelain"], capture_output=True, text=True).stdout
    path = None
    for line in out.splitlines():
        if line.startswith("worktree "):
            path = line.split(" ", 1)[1]
        elif line == "branch refs/heads/main" and path:
            return Path(path)
    return None


def main(argv=None):
    args = parse(argv if argv is not None else sys.argv[1:])
    names = plan(args.stages)
    repo = args.repo.resolve()
    main_tree = (args.main_tree or main_tree_of(repo))
    if main_tree is None:
        raise SystemExit("no checkout of main found: pass --main-tree")
    stamp = time.strftime("%Y%m%d-%H%M%S")
    out = (args.output or repo / "artifacts" / "kde" / f"acceptance-{stamp}").resolve()
    print(f"plan: {' '.join(names)}\nrepo: {repo}\nmain: {main_tree}\nevidence: {out}")
    if args.dry_run:
        print("dry run: nothing is run or changed.")
        for n in names:
            print(f"  - {n}")
        return 0
    out.mkdir(parents=True, exist_ok=True)
    runner = Runner(out / "commands.log")
    ctx = stages.Ctx(repo, Path(main_tree).resolve(), args.qt_dir, out, runner, args)
    run = Run(started=now_iso(), argv=sys.argv)
    run.context = {"harness": "tools/kde/acceptance.py", "scrubbed_desktop_variables_in_generic_runs": ", ".join(scrubbed_vars())}

    def finish():
        run.finished = now_iso()
        run.context.update({f"{k}_sha": v["sha"] for k, v in ctx.shas.items()})
        run.context.update({f"{k}_dirty": v["dirty"] for k, v in ctx.shas.items()})
        run.context.update({"branch": ctx.shas.get("branch", {}).get("branch"), "kde_tree": str(repo), "main_tree": str(ctx.main_tree)})
        write_manifest(run, out / "manifest.json")
        (out / "summary.md").write_text(render_summary(run))

    def on_signal(signum, _frame):
        raise KeyboardInterrupt(f"signal {signum}")

    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, on_signal)
    ctx.run_started = time.time()
    inhibitor = session.IdleInhibitor().__enter__()
    ctx.inhibitor = inhibitor
    try:
        # guards first: it records the SHAs every other stage's context wants
        order = ["guards"] + [n for n in names if n != "guards"] if "guards" in names or "all" in args.stages else names
        for name in order:
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
        try:
            left = stages.stage_leftovers(ctx)
        except Exception as exc:  # recorded like any stage's error: never a crash that skips the manifest
            left = Stage("leftovers", error=f"{type(exc).__name__}: {exc}")
        run.stages.append(left)
    except KeyboardInterrupt as exc:
        print(f"interrupted: {exc}", file=sys.stderr)
    finally:
        inhibitor.__exit__(None, None, None)
        finish()
    print(f"\n{'FAILED' if run.failed else 'PASSED'}: {out}/summary.md")
    return run.exit_code()


if __name__ == "__main__":
    sys.exit(main())
