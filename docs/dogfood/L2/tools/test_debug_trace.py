"""Self-check for debug_trace.py on a synthetic log:

    python test_debug_trace.py

- assistant messages are numbered as model calls; an empty one is counted;
- a repeated command is flagged with its prior count even when the intent
  prose differs; a different mode is a different command;
- a result ID in an observation is listed, and a later `pira_ctx search ID`
  marks it retrieved and the call as a retrieval;
- --helpers marks calls whose argv or edit path names a helper file;
- --from/--to restrict the printed calls but not the summary;
- output is UTF-8 even when the console encoding cannot represent it
  (the cp1252 failure the lab hit on `→`).
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "debug_trace.py")

RUN_TESTS = {"argv": ["python", "-m", "unittest", "tests.test_cli"], "intent": "run tests", "effect": "read_only"}
RUN_TESTS_AGAIN = {"argv": ["python", "-m", "unittest", "tests.test_cli"], "intent": "run tests once more", "effect": "read_only"}
RUN_TESTS_EXACT = {"argv": ["python", "-m", "unittest", "tests.test_cli"], "intent": "run tests", "effect": "read_only", "mode": "exact"}
EVENTS = [
    {"event": "session_start", "max_turns": 100, "t": 1},
    {"event": "message", "role": "user", "blocks": [{"type": "text", "text": "task"}]},
    # call 1: run tests -> synopsis with a result id; the observation carries a non-cp1252 character
    {"event": "message", "role": "assistant", "blocks": [{"type": "tool_call", "id": "c1", "name": "shell", "input": RUN_TESTS}]},
    {"event": "decision", "call_id": "c1", "tool": "shell", "input": RUN_TESTS, "decision": "allow"},
    {"event": "message", "role": "user", "blocks": [{"type": "tool_result", "call_id": "c1", "output": "Captured: 20260910-135253-73f09dfc2fe7 (exit 1):\nFAIL: test_x → see below\n", "is_error": True, "provenance": "observation"}]},
    # call 2: same command, different intent (repeat x1) and the same command in exact mode (not a repeat)
    {"event": "message", "role": "assistant", "blocks": [
        {"type": "tool_call", "id": "c2", "name": "shell", "input": RUN_TESTS_AGAIN},
        {"type": "tool_call", "id": "c2b", "name": "shell", "input": RUN_TESTS_EXACT},
    ]},
    {"event": "decision", "call_id": "c2", "tool": "shell", "input": RUN_TESTS_AGAIN, "decision": "allow"},
    {"event": "decision", "call_id": "c2b", "tool": "shell", "input": RUN_TESTS_EXACT, "decision": "allow"},
    {"event": "message", "role": "user", "blocks": [
        {"type": "tool_result", "call_id": "c2", "output": "Captured: 20260910-135300-aaaaaaaaaaaa (exit 1):\n", "is_error": True, "provenance": "observation"},
        {"type": "tool_result", "call_id": "c2b", "output": "FAIL: test_x\n", "is_error": True, "provenance": "observation"},
    ]},
    # call 3: retrieval from the first id, plus an edit of a helper file
    {"event": "message", "role": "assistant", "blocks": [
        {"type": "tool_call", "id": "c3", "name": "shell", "input": {"argv": ["pira_ctx", "search", "20260910-135253-73f09dfc2fe7", "FAIL"], "intent": "find failure", "effect": "read_only"}},
        {"type": "tool_call", "id": "c4", "name": "edit_file", "input": {"path": "tests/_check.py", "old_text": "a\nb", "new_text": "c", "safety_review": "ok"}},
    ]},
    {"event": "decision", "call_id": "c3", "tool": "shell", "input": {}, "decision": "allow"},
    {"event": "decision", "call_id": "c4", "tool": "edit_file", "input": {}, "decision": {"deny": "no"}},
    {"event": "message", "role": "user", "blocks": [
        {"type": "tool_result", "call_id": "c3", "output": "1 match", "is_error": False, "provenance": "observation"},
        {"type": "tool_result", "call_id": "c4", "output": "not executed: no", "is_error": True, "provenance": "observation"},
    ]},
    # call 4: empty response
    {"event": "message", "role": "assistant", "blocks": [{"type": "text", "text": ""}]},
    {"event": "run_end", "error": "the model returned an empty response (no tool calls and no text)", "t": 9},
]


def run(*extra):
    handle = tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8")
    for event in EVENTS:
        handle.write(json.dumps(event) + "\n")
    handle.close()
    env = dict(os.environ, PYTHONIOENCODING="cp1252", PYTHONUTF8="0")
    try:
        done = subprocess.run([sys.executable, SCRIPT, handle.name, *extra], capture_output=True, check=True, env=env)
        return done.stdout.decode("utf-8").replace("\r\n", "\n")
    finally:
        os.unlink(handle.name)


class DebugTraceTests(unittest.TestCase):
    def test_timeline_flags_and_summary(self):
        out = run("--helpers", "tests/_check.py,tests_output.txt")
        self.assertIn("call   1", out)
        self.assertIn("FAIL: test_x → see below", out)
        self.assertIn("call   4", out)
        self.assertIn("[repeat x1 id:20260910-135300-aaaaaaaaaaaa]", out)
        self.assertIn("ERR [exact] python -m unittest tests.test_cli\n", out)
        self.assertIn("[id:20260910-135253-73f09dfc2fe7]", out)
        self.assertIn("[retrieve]", out)
        self.assertIn("edit_file        deny  ERR _check.py: 3B -> 1B  old[0]='a'  [helper]", out)
        self.assertIn("deny: no", out)
        self.assertIn("model calls 4  tool calls 5  error results 4  empty responses 1", out)
        self.assertIn("repeated identical calls 1  pira_ctx retrievals 1  helper-related calls 1", out)
        self.assertIn("result IDs given to the model: 2; retrieved from later: 1", out)
        self.assertIn("20260910-135253-73f09dfc2fe7  given at call 1  retrieved at [3]", out)
        self.assertIn("20260910-135300-aaaaaaaaaaaa  given at call 2  retrieved at never", out)

    def test_range_restricts_output_but_not_summary(self):
        out = run("--from", "2", "--to", "2")
        self.assertNotIn("call   1", out)
        self.assertIn("call   2", out)
        self.assertNotIn("call   3", out)
        self.assertIn("model calls 4  tool calls 5", out)


if __name__ == "__main__":
    unittest.main()
