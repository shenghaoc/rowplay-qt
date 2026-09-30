# SPDX-License-Identifier: GPL-3.0-or-later
"""The acceptance baseline: the commit this integration is compared with, and proof that a checkout is it.

The comparison is against a *commit*, never against "whatever branch is called main": ADR 0018
was accepted against the post-#144, pre-KDE `main`, and once the stack's layers merge `main` moves on
and contains some or all of the change. A baseline checkout may therefore be a detached HEAD, a
branch of any name or a worktree; what counts is its commit. Everything that needs the baseline
(guards, native, generic, package, visual) takes it from here so there is one definition.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .shell import host_env

# Post-#144 `main`, before any Plasma integration change: what every measurement in ADR 0018 compares with.
BASELINE_SHA = "046c2e30062f7ca38c725307ecc73c6ea62777d2"
BASELINE_NOTE = "post-#144 main, before the Plasma integration"


def _git(runner, tree, *args):
    done = runner.run(["git", "-C", str(tree), *args], env=host_env(), tag="baseline-git")
    return done.rc, done.out.strip()


@dataclass
class BaselineCheck:
    name: str
    ok: bool
    detail_ok: str
    detail_bad: str


def head_of(runner, tree):
    rc, out = _git(runner, tree, "rev-parse", "HEAD")
    return out if rc == 0 else None


def find_baseline_tree(runner, repo, expected=BASELINE_SHA):
    """A worktree of `repo`'s repository whose HEAD is the expected baseline commit, whatever its branch (or none)."""
    rc, out = _git(runner, repo, "worktree", "list", "--porcelain")
    if rc != 0:
        return None
    path = None
    for line in out.splitlines():
        if line.startswith("worktree "):
            path = line.split(" ", 1)[1]
        elif line.startswith("HEAD ") and path and line.split(" ", 1)[1] == expected:
            if Path(path).resolve() != Path(repo).resolve():
                return Path(path)
    return None


def check_baseline(runner, subject, baseline_tree, expected=BASELINE_SHA):
    """Whether `baseline_tree` is the acceptance baseline for `subject`. Returns a list of BaselineCheck.

    * the baseline checkout's HEAD is exactly the expected commit (branch name irrelevant);
    * the subject is not the baseline itself: not the same directory, not the same commit;
    * the subject descends from the baseline, so the comparison is a change made on top of it."""
    want_rc, want = _git(runner, baseline_tree, "rev-parse", "--verify", "--quiet", f"{expected}^{{commit}}")
    have = head_of(runner, baseline_tree)
    subject_head = head_of(runner, subject)
    checks = [BaselineCheck(
        f"the baseline checkout is at the acceptance baseline {expected[:7]} ({BASELINE_NOTE})",
        bool(have) and want_rc == 0 and have == want,
        f"HEAD {have[:7] if have else '?'}",
        f"the checkout is at {have[:7] if have else 'no commit'}, not {expected[:7]}: it is a stale, unrelated or later checkout "
        "(a detached worktree at the expected commit is the intended baseline; its branch name does not matter)")]
    same_dir = Path(baseline_tree).resolve() == Path(subject).resolve()
    same_commit = bool(have) and have == subject_head
    checks.append(BaselineCheck(
        "the tree under test is not its own baseline", not same_dir and not same_commit,
        f"the baseline ({(have or '')[:7]}) and the tree under test ({(subject_head or '')[:7]}) are different commits",
        "the tree under test is the baseline, so every comparison would be a comparison with itself" +
        (" (the same directory)" if same_dir else " (the same commit)")))
    rc, _ = _git(runner, subject, "merge-base", "--is-ancestor", expected, "HEAD")
    checks.append(BaselineCheck(
        f"the tree under test descends from the baseline {expected[:7]}", rc == 0,
        "the baseline commit is an ancestor of HEAD",
        f"{expected[:7]} is not an ancestor of the tree under test: the change is not built on the acceptance baseline"))
    return checks


def describe_requirement(expected=BASELINE_SHA):
    return (f"{expected} ({BASELINE_NOTE}): pass --baseline-tree <a checkout of it, detached is fine>, "
            "or have a worktree already at that commit")
