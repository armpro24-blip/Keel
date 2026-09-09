"""Self-check for session_stats.py on a mixed log:

    python test_session_stats.py

- a completed run (`turns`, no `error`) counts as completed;
- an exhausted-budget run (`error` and `turns`) counts as failed, and its
  calls are counted;
- an old-style failed run (`error`, no `turns`) counts as failed and makes
  the call total incomplete, never zero or the budget.
"""

import io
import json
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import session_stats  # noqa: E402

EVENTS = [
    {"event": "session_start", "max_turns": 100, "t": 1},
    {"event": "message", "role": "user", "blocks": [{"type": "text", "text": "a"}], "t": 2},
    {"event": "run_end", "turns": 12, "t": 3},
    {"event": "run_end", "error": "model-call budget exhausted (max_turns = 100); the run is incomplete; the transcript is kept", "turns": 100, "t": 4},
    {"event": "run_end", "error": "model provider error: HTTP request failed: timeout: global", "t": 5},
]


class SessionStatsTests(unittest.TestCase):
    def run_stats(self, events):
        handle = tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8")
        for event in events:
            handle.write(json.dumps(event) + "\n")
        handle.close()
        out = io.StringIO()
        try:
            with redirect_stdout(out):
                session_stats.main(handle.name)
        finally:
            os.unlink(handle.name)
        return out.getvalue()

    def test_mixed_log_classifies_by_error_and_reports_an_incomplete_count(self):
        text = self.run_stats(EVENTS)
        self.assertIn("runs (user turns) 3  completed 1  failed 2  max_turns 100", text)
        self.assertIn("model calls       at least 112  (INCOMPLETE: 1 run(s) recorded no call count)", text)
        self.assertIn("run error: model-call budget exhausted", text)
        self.assertIn("run error: model provider error", text)

    def test_all_runs_counted_gives_an_exact_total(self):
        text = self.run_stats(EVENTS[:4])
        self.assertIn("runs (user turns) 2  completed 1  failed 1  max_turns 100", text)
        self.assertIn("model calls       112\n", text)

    def test_missing_session_budget_is_reported_not_assumed(self):
        text = self.run_stats(EVENTS[1:3])
        self.assertIn("max_turns not recorded", text)


if __name__ == "__main__":
    unittest.main()
