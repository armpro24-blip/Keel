"""L2 hidden acceptance: black-box checks run after the session, never shown
to the model. Run from the L2 repository root (or set L2_REPO):

    python <Keel>/docs/dogfood/L2/acceptance/test_acceptance.py -v

Every check goes through the command line, the test runner, or the files.
The two formal gates are A1 (project suite passes) and A2 (every frozen
seed test ID is still present); the rest check the requested behavior.
Expected values for the production log were fixed from the frozen task's
semantics and confirmed by the benchmark author's reference solution before
any model run.
"""

import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.environ.get("L2_REPO", os.getcwd()))
PYTHON = sys.executable
FROZEN = os.path.join(HERE, "..", "frozen")


def qw(*args):
    return subprocess.run(
        [PYTHON, "-m", "queuewatch", *args], cwd=REPO, capture_output=True, text=True
    )


def rows(stdout):
    """``{job: (status, attempt, duration)}`` from a report, plus the summary line."""
    lines = stdout.strip().splitlines()
    summary = lines[-1]
    table = {}
    for line in lines[1:-1]:
        parts = line.split()
        table[parts[0]] = tuple(parts[1:])
    return table, summary


def log_file(text):
    handle = tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8", newline="")
    handle.write(text)
    handle.close()
    return handle.name


def event(ts, job, kind, **extra):
    fields = [f'"ts": "2026-09-05T{ts}"', f'"job": "{job}"', f'"event": "{kind}"']
    fields += [f'"{k}": "{v}"' for k, v in extra.items()]
    return "{" + ", ".join(fields) + "}\n"


class Acceptance(unittest.TestCase):
    def assert_data_error(self, result, line, job):
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertNotIn("Traceback", result.stderr)
        self.assertTrue(result.stderr.startswith("queuewatch: "), result.stderr)
        self.assertIn(f"line {line}", result.stderr)
        self.assertIn(job, result.stderr)

    def test_a01_project_suite_passes(self):
        result = subprocess.run(
            [PYTHON, "-m", "unittest", "discover", "-s", "tests"],
            cwd=REPO, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_a02_every_seed_test_id_is_still_present(self):
        with open(os.path.join(FROZEN, "seed_test_ids.txt"), encoding="utf-8") as handle:
            seed_ids = [line.strip() for line in handle if line.strip()]
        result = subprocess.run(
            [PYTHON, os.path.join(HERE, "..", "tools", "seed_test_preservation.py"), REPO],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f"{len(seed_ids)}/{len(seed_ids)} seed tests present", result.stdout)

    def test_a03_retry_after_failure_is_accepted_and_returns_the_job_to_queued(self):
        path = log_file(
            event("06:00:00", "j", "created")
            + event("06:00:01", "j", "started", worker="w")
            + event("06:00:09", "j", "failed", reason="x")
            + event("06:00:10", "j", "retry")
        )
        try:
            result = qw("report", path)
        finally:
            os.unlink(path)
        self.assertEqual(result.returncode, 0, result.stderr)
        table, summary = rows(result.stdout)
        self.assertEqual(table["j"], ("queued", "2", "-"))
        self.assertEqual(summary, "total 1  queued 1  running 0  completed 0  failed 0")

    def test_a04_each_retry_increments_the_attempt_count(self):
        result = qw("report", "data/production_2026-09.jsonl")
        self.assertEqual(result.returncode, 0, result.stderr)
        table, summary = rows(result.stdout)
        self.assertEqual(table["ingest"][:2], ("completed", "2"))
        self.assertEqual(table["transform"][:2], ("completed", "3"))
        self.assertEqual(table["publish"][:2], ("queued", "2"))
        self.assertEqual(table["cleanup"][:2], ("queued", "1"))
        self.assertEqual(summary, "total 4  queued 2  running 0  completed 2  failed 0")

    def test_a05_a_retried_job_can_start_and_complete_with_its_new_duration(self):
        path = log_file(
            event("06:00:00", "j", "created")
            + event("06:00:01", "j", "started", worker="w")
            + event("06:00:09", "j", "failed", reason="x")
            + event("06:00:10", "j", "retry")
            + event("06:01:00", "j", "started", worker="w")
            + event("06:03:05", "j", "completed")
        )
        try:
            result = qw("report", path)
        finally:
            os.unlink(path)
        self.assertEqual(result.returncode, 0, result.stderr)
        table, _ = rows(result.stdout)
        self.assertEqual(table["j"], ("completed", "2", "0:02:05"))

    def test_a06_retry_before_failure_is_rejected_as_a_data_error(self):
        cases = {
            "queued": event("06:00:00", "j", "created") + event("06:00:01", "j", "retry"),
            "running": event("06:00:00", "j", "created")
            + event("06:00:01", "j", "started")
            + event("06:00:02", "j", "retry"),
            "completed": event("06:00:00", "j", "created")
            + event("06:00:01", "j", "started")
            + event("06:00:02", "j", "completed")
            + event("06:00:03", "j", "retry"),
        }
        for status, text in cases.items():
            path = log_file(text)
            try:
                result = qw("report", path)
            finally:
                os.unlink(path)
            with self.subTest(status=status):
                self.assert_data_error(result, line=text.count("\n"), job="j")

    def test_a07_existing_invalid_transitions_still_fail_the_same_way(self):
        path = log_file(
            event("06:00:00", "j", "created")
            + event("06:00:01", "j", "started")
            + event("06:00:02", "j", "failed")
            + event("06:00:03", "j", "started")
        )
        try:
            result = qw("check", path)
        finally:
            os.unlink(path)
        self.assert_data_error(result, line=4, job="j")
        self.assertIn("cannot apply 'started' to a job that is failed", result.stderr)

    def test_a08_status_failed_limits_rows_but_not_the_summary(self):
        result = qw("report", "data/events.jsonl", "--status", "failed")
        self.assertEqual(result.returncode, 0, result.stderr)
        table, summary = rows(result.stdout)
        self.assertEqual(sorted(table), ["build-web", "test-e2e"])
        self.assertEqual(summary, "total 7  queued 1  running 1  completed 3  failed 2")

    def test_a09_status_completed_limits_rows_but_not_the_summary(self):
        result = qw("report", "data/events.jsonl", "--status", "completed")
        self.assertEqual(result.returncode, 0, result.stderr)
        table, summary = rows(result.stdout)
        self.assertEqual(sorted(table), ["build-api", "lint", "test-unit"])
        self.assertEqual(summary, "total 7  queued 1  running 1  completed 3  failed 2")

    def test_a10_status_with_no_matching_jobs_prints_header_and_summary(self):
        result = qw("report", "data/production_2026-09.jsonl", "--status", "running")
        self.assertEqual(result.returncode, 0, result.stderr)
        table, summary = rows(result.stdout)
        self.assertEqual(table, {})
        self.assertEqual(summary, "total 4  queued 2  running 0  completed 2  failed 0")

    def test_a11_unknown_status_is_a_usage_error(self):
        result = qw("report", "data/events.jsonl", "--status", "paused")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertNotIn("Traceback", result.stderr)

    def test_a12_report_without_status_is_unchanged(self):
        with open(os.path.join(FROZEN, "report_events_expected.txt"), encoding="utf-8") as handle:
            expected = handle.read().splitlines()
        result = qw("report", "data/events.jsonl")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), expected)
        check = qw("check", "data/events.jsonl")
        self.assertEqual((check.returncode, check.stdout.strip()), (0, "ok: 18 events, 7 jobs"))

    def test_a13_readme_documents_retry_and_status(self):
        with open(os.path.join(REPO, "README.md"), encoding="utf-8") as handle:
            text = handle.read()
        self.assertIn("retry", text)
        self.assertIn("--status", text)


if __name__ == "__main__":
    unittest.main()
