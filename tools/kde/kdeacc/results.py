# SPDX-License-Identifier: GPL-3.0-or-later
"""Results of an acceptance run: statuses, checks, stages, the manifest and the summary.

Every check ends in one of five statuses. Only FAIL fails a run; the others say
precisely why a check did not count as a pass:

  PASS             measured, and it met its criterion
  FAIL             measured, and it did not (or could not be measured when it must be)
  SKIPPED          not run: the mode, or a flag that permits it, was not chosen
  MANUAL_OPTIONAL  automation cannot do it safely; a person may look, and nothing blocks on it
  UNAVAILABLE      the machine lacks what the check needs (a tool, a display, a session)

The manifest (JSON) is the source of truth and the summary (Markdown) is
generated from it, so no number is typed twice.
"""

from __future__ import annotations

import json
import time
from dataclasses import asdict, dataclass, field
from enum import Enum


class Status(str, Enum):
    PASS = "PASS"
    FAIL = "FAIL"
    SKIPPED = "SKIPPED"
    MANUAL_OPTIONAL = "MANUAL_OPTIONAL"
    UNAVAILABLE = "UNAVAILABLE"


@dataclass
class Check:
    name: str
    status: Status
    detail: str = ""
    # Measured values worth keeping in the manifest (numbers, strings, small dicts).
    data: dict = field(default_factory=dict)
    # Evidence files, relative to the run directory.
    evidence: list = field(default_factory=list)


@dataclass
class Stage:
    name: str
    checks: list = field(default_factory=list)
    seconds: float = 0.0
    error: str = ""  # an exception that ended the stage early

    def add(self, name, status, detail="", data=None, evidence=None):
        check = Check(name, status, detail, data or {}, evidence or [])
        self.checks.append(check)
        return check

    def passed(self, name, detail="", **data):
        return self.add(name, Status.PASS, detail, data)

    def failed(self, name, detail="", **data):
        return self.add(name, Status.FAIL, detail, data)

    def expect(self, name, ok, detail_ok="", detail_bad="", **data):
        """PASS with detail_ok when ok, else FAIL with detail_bad."""
        return self.add(name, Status.PASS if ok else Status.FAIL, detail_ok if ok else detail_bad, data)

    @property
    def status(self) -> Status:
        if self.error:
            return Status.FAIL
        statuses = [check.status for check in self.checks]
        if Status.FAIL in statuses:
            return Status.FAIL
        if not statuses:
            return Status.SKIPPED
        if all(s in (Status.SKIPPED, Status.UNAVAILABLE, Status.MANUAL_OPTIONAL) for s in statuses):
            # Nothing was measured: say what dominates, worst first.
            for s in (Status.UNAVAILABLE, Status.SKIPPED, Status.MANUAL_OPTIONAL):
                if s in statuses:
                    return s
        return Status.PASS


@dataclass
class Run:
    started: str = ""
    finished: str = ""
    argv: list = field(default_factory=list)
    stages: list = field(default_factory=list)
    context: dict = field(default_factory=dict)  # SHAs, tree state, host summary
    interrupted: str = ""        # why the run stopped early ("" when it ran to the end): a signal, after the current stage was recorded
    interrupt_signal: int = 0    # that signal's number, for the exit status

    def stage(self, name) -> Stage:
        stage = Stage(name)
        self.stages.append(stage)
        return stage

    @property
    def failed(self) -> bool:
        return bool(self.interrupted) or any(stage.status == Status.FAIL for stage in self.stages)

    def exit_code(self) -> int:
        """Nonzero when any check failed, any stage died or the run was interrupted (128 + the signal, as a shell reports it)."""
        if self.interrupted:
            return 128 + self.interrupt_signal if self.interrupt_signal else 130
        return 1 if self.failed else 0

    def to_dict(self):
        def encode(obj):
            if isinstance(obj, Status):
                return obj.value
            raise TypeError(type(obj))

        body = asdict(self)
        for stage, live in zip(body["stages"], self.stages):
            stage["status"] = live.status.value
        body["overall"] = "INTERRUPTED" if self.interrupted else "FAIL" if self.failed else "PASS"
        return json.loads(json.dumps(body, default=encode))


def now_iso() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S%z")


def write_manifest(run: Run, path) -> None:
    with open(path, "w") as handle:
        json.dump(run.to_dict(), handle, indent=2, sort_keys=False)
        handle.write("\n")


def render_summary(run: Run) -> str:
    """Markdown generated from the run: a table per stage, and what failed, first."""
    data = run.to_dict()
    lines = [f"# KDE Plasma native acceptance: {data['overall']}", ""]
    lines.append(f"- started {data['started']}, finished {data['finished']}")
    if data["interrupted"]:
        lines.append(f"- **interrupted: {data['interrupted']}**: the stage that was running was recorded (with any desktop restoration), and no later stage ran")
    for key, value in data["context"].items():
        if isinstance(value, (str, int, float, bool)) or value is None:
            lines.append(f"- {key}: {value}")
    failures = [
        (stage["name"], check)
        for stage in data["stages"]
        for check in stage["checks"]
        if check["status"] == "FAIL"
    ] + [(stage["name"], {"name": "stage error", "detail": stage["error"]}) for stage in data["stages"] if stage["error"]]
    if failures:
        lines += ["", "## FAILED", ""]
        for stage_name, check in failures:
            lines.append(f"- **{stage_name}: {check['name']}**: {check['detail']}")
    lines.append("")
    lines.append("| stage | status | checks | seconds |")
    lines.append("| --- | --- | --- | --- |")
    for stage in data["stages"]:
        lines.append(f"| {stage['name']} | {stage['status']} | {len(stage['checks'])} | {stage['seconds']:.1f} |")
    for stage in data["stages"]:
        lines += ["", f"## {stage['name']} ({stage['status']})", ""]
        if stage["error"]:
            lines.append(f"Stage error: {stage['error']}")
            lines.append("")
        lines.append("| check | status | detail |")
        lines.append("| --- | --- | --- |")
        for check in stage["checks"]:
            detail = check["detail"].replace("|", "\\|").replace("\n", " ")
            lines.append(f"| {check['name']} | {check['status']} | {detail} |")
    lines.append("")
    return "\n".join(lines)
