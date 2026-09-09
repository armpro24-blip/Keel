import unittest
from datetime import datetime

from queuewatch.model import Job
from queuewatch.report import job_row, render_report, render_summary, render_table, summary_counts

T = datetime(2026, 9, 1, 8, 0, 0)


def job(id, status, attempt=1, started=None, finished=None):
    return Job(
        id=id,
        status=status,
        attempt=attempt,
        started_at=T.replace(second=started) if started is not None else None,
        finished_at=T.replace(second=finished) if finished is not None else None,
    )


class RowTests(unittest.TestCase):
    def test_finished_job_shows_its_duration(self):
        self.assertEqual(job_row(job("a", "completed", started=5, finished=17)), ("a", "completed", "1", "0:00:12"))

    def test_unfinished_job_shows_a_dash(self):
        self.assertEqual(job_row(job("a", "queued")), ("a", "queued", "1", "-"))
        self.assertEqual(job_row(job("a", "running", started=5)), ("a", "running", "1", "-"))


class SummaryTests(unittest.TestCase):
    def test_counts_every_status_even_when_zero(self):
        counts = summary_counts([job("a", "failed"), job("b", "failed"), job("c", "queued")])
        self.assertEqual(counts, {"queued": 1, "running": 0, "completed": 0, "failed": 2})

    def test_summary_line_lists_statuses_in_lifecycle_order(self):
        self.assertEqual(
            render_summary([job("a", "completed"), job("b", "running")]),
            "total 2  queued 0  running 1  completed 1  failed 0",
        )


class TableTests(unittest.TestCase):
    def test_columns_align_to_the_widest_cell_and_lines_are_rstripped(self):
        text = render_table([("alpha", "queued", "1", "-"), ("b", "completed", "12", "0:00:01")])
        self.assertEqual(
            text.split("\n"),
            [
                "job    status     attempt  duration",
                "alpha  queued     1        -",
                "b      completed  12       0:00:01",
            ],
        )

    def test_report_sorts_rows_by_job_id_and_ends_with_the_summary(self):
        text = render_report([job("zeta", "queued"), job("alpha", "completed", started=0, finished=3)])
        self.assertEqual(
            text.split("\n"),
            [
                "job    status     attempt  duration",
                "alpha  completed  1        0:00:03",
                "zeta   queued     1        -",
                "total 2  queued 1  running 0  completed 1  failed 0",
            ],
        )

    def test_empty_report_is_header_and_zero_summary(self):
        self.assertEqual(
            render_report([]).split("\n"),
            ["job  status  attempt  duration", "total 0  queued 0  running 0  completed 0  failed 0"],
        )


if __name__ == "__main__":
    unittest.main()
