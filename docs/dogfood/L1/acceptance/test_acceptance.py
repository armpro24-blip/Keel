"""L1 acceptance: black-box checks run after the Keel session, never shown
to the model. Run from the L1 repository root (or set L1_REPO):

    python <Keel>/docs/dogfood/L1/acceptance/test_acceptance.py -v

Every check goes through the command line or the files, so the model is free
to structure the code as it likes.
"""

import os
import subprocess
import sys
import tempfile
import unittest

REPO = os.path.abspath(os.environ.get("L1_REPO", os.getcwd()))
PYTHON = sys.executable


def tally(*args):
    return subprocess.run(
        [PYTHON, "-m", "tally", *args], cwd=REPO, capture_output=True, text=True
    )


def table(stdout):
    """``{name: amount}`` from the report's two-column lines."""
    rows = {}
    for line in stdout.strip().splitlines():
        parts = line.split()
        if len(parts) != 2:
            raise AssertionError(f"not a two-column line: {line!r}")
        rows[parts[0]] = parts[1]
    return rows


class Acceptance(unittest.TestCase):
    def test_a1_project_suite_passes(self):
        result = subprocess.run(
            [PYTHON, "-m", "unittest", "discover", "-s", "tests"],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_a2_thousands_separators_parse(self):
        result = tally("report", "data/expenses.csv")
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = table(result.stdout)
        self.assertEqual(rows["rent"], "2500.00")
        self.assertEqual(rows["total"], "3051.85")
        self.assertEqual(len(rows), 5)

    def test_a3_month_filter_july(self):
        result = tally("report", "data/expenses.csv", "--month", "2026-07")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            table(result.stdout),
            {"rent": "1250.00", "groceries": "115.95", "transport": "32.00", "total": "1397.95"},
        )

    def test_a4_month_filter_august(self):
        result = tally("report", "data/expenses.csv", "--month", "2026-08")
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = table(result.stdout)
        self.assertEqual(rows["utilities"], "90.40")
        self.assertEqual(rows["groceries"], "121.45")
        self.assertEqual(rows["total"], "1493.85")

    def test_a5_invalid_month_is_a_usage_error_not_a_traceback(self):
        for bad in ("2026-8", "August"):
            result = tally("report", "data/expenses.csv", "--month", bad)
            self.assertNotEqual(result.returncode, 0, bad)
            self.assertNotIn("Traceback", result.stderr, bad)
            self.assertEqual(result.stdout, "", bad)

    def test_a6_non_numbers_are_still_rejected(self):
        handle = tempfile.NamedTemporaryFile(
            "w", suffix=".csv", delete=False, encoding="utf-8", newline=""
        )
        handle.write("date,category,amount\n2026-08-02,a,1\n2026-08-03,b,abc\n")
        handle.close()
        try:
            result = tally("report", handle.name)
        finally:
            os.unlink(handle.name)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("Traceback", result.stderr)
        self.assertEqual(result.stdout, "")

    def test_a7_readme_documents_the_option(self):
        with open(os.path.join(REPO, "README.md"), encoding="utf-8") as handle:
            self.assertIn("--month", handle.read())


if __name__ == "__main__":
    unittest.main()
