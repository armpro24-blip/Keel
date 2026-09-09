import contextlib
import io
import os
import tempfile
import unittest

from queuewatch.cli import main

LOG = (
    '{"ts": "2026-09-01T08:00:00", "job": "b", "event": "created"}\n'
    '{"ts": "2026-09-01T08:00:01", "job": "a", "event": "created"}\n'
    '{"ts": "2026-09-01T08:00:02", "job": "b", "event": "started", "worker": "w1"}\n'
    '{"ts": "2026-09-01T08:00:05", "job": "a", "event": "started", "worker": "w2"}\n'
    '{"ts": "2026-09-01T08:01:02", "job": "b", "event": "completed"}\n'
    '{"ts": "2026-09-01T08:01:09", "job": "a", "event": "failed", "reason": "oom"}\n'
    '{"ts": "2026-09-01T08:02:00", "job": "c", "event": "created"}\n'
)


class CliTests(unittest.TestCase):
    def setUp(self):
        self.paths = []
        self.path = self.log(LOG)

    def tearDown(self):
        for path in self.paths:
            os.unlink(path)

    def log(self, text):
        handle = tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8", newline="")
        handle.write(text)
        handle.close()
        self.paths.append(handle.name)
        return handle.name

    def run_main(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                code = main(list(argv))
            except SystemExit as exit:  # argparse usage errors
                code = exit.code
        return code, out.getvalue(), err.getvalue()

    def test_report_prints_rows_and_summary(self):
        code, out, err = self.run_main("report", self.path)
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(
            out.split("\n"),
            [
                "job  status     attempt  duration",
                "a    failed     1        0:01:04",
                "b    completed  1        0:01:00",
                "c    queued     1        -",
                "total 3  queued 1  running 0  completed 1  failed 1",
                "",
            ],
        )

    def test_check_counts_events_and_jobs(self):
        code, out, err = self.run_main("check", self.path)
        self.assertEqual((code, out, err), (0, "ok: 7 events, 3 jobs\n", ""))

    def test_missing_file_is_a_data_error(self):
        code, out, err = self.run_main("report", "no-such.jsonl")
        self.assertEqual((code, out), (1, ""))
        self.assertTrue(err.startswith("queuewatch: "), err)

    def test_malformed_line_is_a_data_error_with_its_number(self):
        path = self.log('{"ts": "2026-09-01T08:00:00", "job": "a", "event": "created"}\nnope\n')
        code, out, err = self.run_main("report", path)
        self.assertEqual((code, out), (1, ""))
        self.assertTrue(err.startswith("queuewatch: line 2: not valid JSON"), err)

    def test_invalid_transition_is_a_data_error_naming_line_and_job(self):
        path = self.log(
            '{"ts": "2026-09-01T08:00:00", "job": "a", "event": "created"}\n'
            '{"ts": "2026-09-01T08:00:01", "job": "a", "event": "completed"}\n'
        )
        code, out, err = self.run_main("check", path)
        self.assertEqual((code, out), (1, ""))
        self.assertEqual(err, "queuewatch: line 2: job a: cannot apply 'completed' to a job that is queued\n")

    def test_unknown_event_kind_is_a_data_error(self):
        path = self.log('{"ts": "2026-09-01T08:00:00", "job": "a", "event": "paused"}\n')
        code, out, err = self.run_main("report", path)
        self.assertEqual((code, out), (1, ""))
        self.assertEqual(err, "queuewatch: line 1: unknown event 'paused'\n")

    def test_missing_subcommand_is_a_usage_error(self):
        code, out, err = self.run_main()
        self.assertEqual((code, out), (2, ""))
        self.assertIn("usage:", err)

    def test_sample_data_file_reports_seven_jobs(self):
        here = os.path.dirname(os.path.abspath(__file__))
        sample = os.path.join(here, "..", "data", "events.jsonl")
        code, out, err = self.run_main("report", sample)
        self.assertEqual((code, err), (0, ""))
        self.assertTrue(out.endswith("total 7  queued 1  running 1  completed 3  failed 2\n"), out)


if __name__ == "__main__":
    unittest.main()
