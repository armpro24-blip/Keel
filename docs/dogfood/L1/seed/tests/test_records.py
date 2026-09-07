import os
import tempfile
import unittest
from datetime import date

from tally.records import Record, RecordError, parse_amount, parse_date, read_records


def write_csv(text):
    handle = tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False, encoding="utf-8", newline="")
    handle.write(text)
    handle.close()
    return handle.name


class ParseTests(unittest.TestCase):
    def test_amount_accepts_plain_decimals(self):
        self.assertEqual(parse_amount("12.50"), 12.5)
        self.assertEqual(parse_amount(" 7 "), 7.0)

    def test_amount_rejects_non_numbers(self):
        with self.assertRaises(RecordError) as caught:
            parse_amount("twelve")
        self.assertIn("twelve", str(caught.exception))

    def test_date_accepts_iso_and_rejects_others(self):
        self.assertEqual(parse_date("2026-08-05"), date(2026, 8, 5))
        with self.assertRaises(RecordError):
            parse_date("05.08.2026")


class ReadRecordsTests(unittest.TestCase):
    def tearDown(self):
        for path in getattr(self, "paths", []):
            os.unlink(path)

    def csv(self, text):
        path = write_csv(text)
        self.paths = getattr(self, "paths", []) + [path]
        return path

    def test_reads_rows_in_order_and_normalizes_category(self):
        path = self.csv(
            "date,category,amount,note\n"
            "2026-08-02,Groceries,48.10,\n"
            "2026-08-05,utilities,90.40,electricity\n"
        )
        self.assertEqual(
            read_records(path),
            [
                Record(date(2026, 8, 2), "groceries", 48.10, ""),
                Record(date(2026, 8, 5), "utilities", 90.40, "electricity"),
            ],
        )

    def test_missing_column_is_reported(self):
        path = self.csv("date,amount\n2026-08-02,1\n")
        with self.assertRaises(RecordError) as caught:
            read_records(path)
        self.assertIn("category", str(caught.exception))

    def test_bad_row_names_its_line(self):
        path = self.csv("date,category,amount\n2026-08-02,a,1\n2026-08-03,b,oops\n")
        with self.assertRaises(RecordError) as caught:
            read_records(path)
        self.assertIn("line 3", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
