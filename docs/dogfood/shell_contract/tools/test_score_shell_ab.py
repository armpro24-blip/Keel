"""Self-check for score_shell_ab.py:

    python test_score_shell_ab.py

- a SyntaxError whose echoed line is a line of the sent element is the program's
  own error, with or without a shell; an echoed fragment after an explicit shell
  is a confirmed argument-usage error; a SyntaxError with no echoed line is
  undetermined;
- structural denials (argv not an array, standalone operator) are confirmed
  argument-usage errors; a missing-review denial is not;
- correctness needs a completed run: a correct final text after budget
  exhaustion is not correct; D1 with the right file but an exhausted budget is
  file_correct but not correct; CRLF-only differences are line_ending;
- helper paths exclude tracked files and the task's own files.
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import score_shell_ab as s  # noqa: E402


class ClassifyTests(unittest.TestCase):
    def test_program_error_direct(self):
        argv = ["python", "-c", "import x; def f(): pass"]
        out = '[stderr]\n  File "<string>", line 1\n    import x; def f(): pass\n              ^^^\nSyntaxError: invalid syntax\n'
        self.assertEqual(s.classify_syntax_error(argv, out), "prog")

    def test_program_error_through_shell_is_still_program_error(self):
        argv = ["cmd", "/C", "python -c \"import x; def f(): pass\""]
        out = '[stderr]\n  File "<string>", line 1\n    import x; def f(): pass\n              ^^^\nSyntaxError: invalid syntax\n'
        self.assertEqual(s.classify_syntax_error(argv, out), "prog")

    def test_shell_fragment_is_confirmed_argument_error(self):
        argv = ["cmd", "/C", "python -c \"import sys; print('hi')\""]
        out = '[stderr]\n  File "<string>", line 1\n    "import\n    ^\nSyntaxError: unterminated string literal (detected at line 1)\n'
        self.assertEqual(s.classify_syntax_error(argv, out), "arg")

    def test_synopsis_prefixed_traceback_is_parsed(self):
        argv = ["python", "-c", "exec('try: f() except E: pass')"]
        out = ('Captured: 20260910-135905-514ca5dae94d (exit 1):\nPROGRAM data:\nL1 stderr: Traceback (most recent call last): \n'
               'L2 stderr:   File "<string>", line 1, in <module> \nL3 stderr:   File "<string>", line 1 \n'
               'L4 stderr:     try: f() except E: pass \nL5 stderr:          ^^^^^^ \n')
        # the echoed line is the inner exec string, not a line of the sent element: nothing proves a shell changed it
        self.assertEqual(s.classify_syntax_error(argv, out), "undetermined")

    def test_no_echo_is_undetermined_and_no_error_is_none(self):
        self.assertEqual(s.classify_syntax_error(["python", "-c", "x"], "[stderr]\nSyntaxError: invalid syntax\n"), "undetermined")
        self.assertIsNone(s.classify_syntax_error(["python", "-c", "x"], "ok\n[exit 0]\n"))


def events(final_text, run_end, tool_calls=(), denials=()):
    ev = [{"event": "session_start", "max_turns": 12}, {"event": "message", "role": "user", "blocks": [{"type": "text", "text": "t"}]}]
    for i, (tool, value, output) in enumerate(tool_calls):
        cid = f"c{i}"
        ev.append({"event": "message", "role": "assistant", "blocks": [{"type": "tool_call", "id": cid, "name": tool, "input": value}]})
        verdict = {"deny": denials[i]} if i < len(denials) and denials[i] else "allow"
        ev.append({"event": "decision", "call_id": cid, "tool": tool, "input": value, "decision": verdict})
        ev.append({"event": "message", "role": "user", "blocks": [{"type": "tool_result", "call_id": cid, "output": output, "is_error": False, "provenance": "observation"}]})
    ev.append({"event": "message", "role": "assistant", "blocks": [{"type": "text", "text": final_text}]})
    ev.append(run_end)
    return ev


class ScoreTests(unittest.TestCase):
    def setUp(self):
        self.repo = tempfile.mkdtemp()
        subprocess.run(["git", "-C", self.repo, "init", "-q"], check=True)
        with open(os.path.join(self.repo, "a.py"), "w") as f:
            f.write("x")
        subprocess.run(["git", "-C", self.repo, "add", "a.py"], check=True)

    def test_correct_needs_completion(self):
        end, correct, *_ = s.score(events("42", {"event": "run_end", "turns": 3}), "R1", self.repo)
        self.assertEqual((end, correct), ("completion", True))
        end, correct, *_ = s.score(events("42", {"event": "run_end", "error": "model-call budget exhausted (max_turns = 12); the run is incomplete; the transcript is kept", "turns": 12}), "R1", self.repo)
        self.assertEqual((end, correct), ("budget", False))

    def test_structural_denials_count_and_review_denials_do_not(self):
        calls = [
            ("shell", {"argv": "python -c x"}, "not executed: input needs an array field 'argv'"),
            ("shell", {"argv": ["echo", ">", "f.txt"]}, "not executed: '>' ... without a shell"),
            ("edit_file", {"path": "a.py", "old_text": "x", "new_text": "y"}, "not executed: PIRA Full-Permission Behavior: an edit needs a non-empty safety_review"),
        ]
        denials = ["input needs an array field 'argv'", "'>' is a shell operator; argv runs without a shell", "PIRA Full-Permission Behavior: an edit needs a non-empty safety_review before it runs"]
        *_, counts, helpers, notes, _ = s.score(events("42", {"event": "run_end", "turns": 4}, calls, denials), "R1", self.repo)
        self.assertEqual(counts["arg_errors"], 2)
        self.assertEqual(counts["denials_review"], 1)
        self.assertEqual(helpers, [])  # a.py is tracked; f.txt is a redirect target -> helper
        self.assertEqual(counts["helper_calls"], 1)

    def test_d1_file_states(self):
        os.makedirs(os.path.join(self.repo, "notes"))
        with open(os.path.join(self.repo, "notes", "summary.txt"), "wb") as f:
            f.write(s.D1_BYTES)
        end, correct, file_correct, line_ending, *_ = s.score(events("done", {"event": "run_end", "error": "model-call budget exhausted (max_turns = 12)", "turns": 12}), "D1", self.repo)
        self.assertEqual((end, correct, file_correct, line_ending), ("budget", False, True, False))
        with open(os.path.join(self.repo, "notes", "summary.txt"), "wb") as f:
            f.write(b"\xef\xbb\xbf" + s.D1_BYTES.replace(b"\n", b"\r\n"))
        end, correct, file_correct, line_ending, *_ = s.score(events("done", {"event": "run_end", "turns": 2}), "D1", self.repo)
        self.assertEqual((end, correct, file_correct, line_ending), ("completion", False, False, True))

    def test_helper_paths_exclude_task_files(self):
        calls = [("shell", {"argv": ["python", "-c", "open('data/quotes.jsonl'); open('tests/_check.py')"]}, "[exit 0]")]
        *_, counts, helpers, _, _ = s.score(events(s.EXPECTED["D2"], {"event": "run_end", "turns": 2}, calls), "D2", self.repo)
        self.assertEqual(helpers, ["tests/_check.py"])
        self.assertEqual(counts["helper_calls"], 1)


if __name__ == "__main__":
    unittest.main()
