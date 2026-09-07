import contextlib
import io
import os
import tempfile
import unittest

from tally.cli import main


class CliTests(unittest.TestCase):
    def setUp(self):
        handle = tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False, encoding="utf-8", newline="")
        handle.write(
            "date,category,amount\n"
            "2026-08-02,groceries,48.10\n"
            "2026-08-05,utilities,90.40\n"
            "2026-08-19,groceries,73.35\n"
        )
        handle.close()
        self.path = handle.name

    def tearDown(self):
        os.unlink(self.path)

    def run_main(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = main(list(argv))
        return code, out.getvalue(), err.getvalue()

    def test_report_prints_totals_and_exits_zero(self):
        code, out, err = self.run_main("report", self.path)
        self.assertEqual(code, 0)
        self.assertEqual(err, "")
        self.assertEqual(
            out.split("\n"),
            [
                "groceries       121.45",
                "utilities        90.40",
                "total           211.85",
                "",
            ],
        )

    def test_top_limits_categories_but_not_the_total(self):
        code, out, _ = self.run_main("report", self.path, "--top", "1")
        self.assertEqual(code, 0)
        self.assertEqual(out.split("\n"), ["groceries       121.45", "total           211.85", ""])

    def test_missing_file_is_an_error_message_not_a_traceback(self):
        code, out, err = self.run_main("report", "no-such-file.csv")
        self.assertEqual(code, 1)
        self.assertEqual(out, "")
        self.assertTrue(err.startswith("tally: "), err)


if __name__ == "__main__":
    unittest.main()
