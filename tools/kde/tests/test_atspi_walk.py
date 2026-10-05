# SPDX-License-Identifier: GPL-3.0-or-later
"""AT-SPI failure reporting and independent action coverage without an accessibility bus."""

import contextlib
import io
import json
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.dont_write_bytecode = True

import atspi_walk  # noqa: E402


class Node:
    def __init__(self, role, name="", children=(), states=()):
        self.role, self.name, self.children, self.states = role, name, list(children), states
        self.text = ""
        self.focused = 0
        self.actions = 0
        self.on_focus = lambda: None

    @property
    def childCount(self):
        return len(self.children)

    def getChildAtIndex(self, index):
        return self.children[index]

    def getRoleName(self):
        return self.role

    def getState(self):
        return self

    def contains(self, state):
        return state in self.states

    def queryComponent(self):
        return self

    def grabFocus(self):
        self.focused += 1
        self.on_focus()

    def queryEditableText(self):
        return self

    def setTextContents(self, text):
        self.text = text

    def queryAction(self):
        return self

    def doAction(self, index):
        assert index == 0
        self.actions += 1
        self.name = {"Replay": "Play", "Play": "Pause", "Pause": "Play"}[self.name]


class EditableFieldCoverage(unittest.TestCase):
    def run_walk(self, app):
        output = io.StringIO()
        fake_atspi = SimpleNamespace(STATE_EDITABLE="editable", STATE_FOCUSABLE="focusable")
        with mock.patch.dict(sys.modules, {"pyatspi": fake_atspi}), mock.patch.object(sys, "argv", ["atspi_walk.py"]), \
                mock.patch.object(atspi_walk, "find_app", return_value=app), mock.patch.object(atspi_walk.time, "sleep"), \
                contextlib.redirect_stdout(output):
            atspi_walk.main()
        return json.loads(output.getvalue())

    def fixture(self, count):
        fields = [Node("text", states=("editable", "focusable")) for _ in range(count)]
        replay = Node("button", "Replay")
        label = Node("label", "17 matching")
        app = Node("application", "rowplay", [*fields, label, replay])
        return app, fields, label, replay

    def test_missing_fields_fail_once_and_replay_play_pause_still_run(self):
        for count in range(3):
            with self.subTest(editable_fields=count):
                app, fields, _, replay = self.fixture(count)
                report = self.run_walk(app)
                self.assertIsNone(report["error"])
                failed = [step for step in report["steps"] if not step["ok"]]
                self.assertEqual(len(failed), 1)
                self.assertEqual(failed[0]["name"], "the search field and both date fields are exposed as editable text")
                self.assertEqual(failed[0]["detail"], f"{count} editable text fields")
                self.assertEqual(replay.actions, 3)
                self.assertTrue(all(field.focused == 0 for field in fields))
                by_name = {step["name"]: step for step in report["steps"]}
                for name in ("the workout's Replay button is exposed", "the replay screen shows a Play button",
                             "Press Play turns it into Pause", "Press Pause turns it back into Play",
                             "the app is still on the bus after every step"):
                    self.assertTrue(by_name[name]["ok"], name)
                self.assertNotIn("leaving From with a date resets the list without aborting", by_name)

    def test_three_or_more_fields_keep_the_date_and_action_checks(self):
        for count in (3, 4):
            with self.subTest(editable_fields=count):
                app, fields, label, replay = self.fixture(count)
                for field in fields:
                    field.on_focus = lambda: setattr(label, "name", "9 matching" if fields[1].text else "17 matching")
                report = self.run_walk(app)
                self.assertIsNone(report["error"])
                self.assertTrue(all(step["ok"] for step in report["steps"]), report)
                self.assertEqual(len(report["steps"]), 9)
                self.assertTrue(all(field.focused > 0 for field in fields[:3]))
                self.assertTrue(all(field.focused == 0 for field in fields[3:]))
                self.assertEqual(fields[1].text, "")
                self.assertEqual(replay.actions, 3)

    def test_noneditable_or_nonfocusable_text_does_not_satisfy_the_date_requirement(self):
        app, _, _, replay = self.fixture(2)
        app.children.extend([Node("text", states=("editable",)), Node("text", states=("focusable",))])
        report = self.run_walk(app)
        self.assertIsNone(report["error"])
        self.assertFalse(report["steps"][1]["ok"])
        self.assertEqual(report["steps"][1]["detail"], "2 editable text fields")
        self.assertEqual(replay.actions, 3)


if __name__ == "__main__":
    unittest.main()
