import unittest
from datetime import date

from tally.records import Record
from tally.report import render, totals_by_category


class TotalsTests(unittest.TestCase):
    def test_sums_per_category_largest_first(self):
        records = [
            Record(date(2026, 8, 1), "groceries", 10.0),
            Record(date(2026, 8, 2), "rent", 500.0),
            Record(date(2026, 8, 3), "groceries", 5.5),
        ]
        self.assertEqual(
            list(totals_by_category(records).items()),
            [("rent", 500.0), ("groceries", 15.5)],
        )

    def test_ties_are_ordered_by_name(self):
        records = [Record(date(2026, 8, 1), "b", 1.0), Record(date(2026, 8, 1), "a", 1.0)]
        self.assertEqual(list(totals_by_category(records)), ["a", "b"])


class RenderTests(unittest.TestCase):
    def test_lines_are_aligned_and_end_with_total(self):
        text = render({"rent": 500.0, "groceries": 15.5})
        self.assertEqual(
            text.split("\n"),
            [
                "rent            500.00",
                "groceries        15.50",
                "total           515.50",
            ],
        )

    def test_empty_report_is_just_the_total(self):
        self.assertEqual(render({}), "total             0.00")


if __name__ == "__main__":
    unittest.main()
