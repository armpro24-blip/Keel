import unittest
from datetime import datetime, timedelta

from queuewatch.timeutil import TimeError, duration_between, format_duration, parse_timestamp


class ParseTimestampTests(unittest.TestCase):
    def test_accepts_iso_with_t_and_with_space(self):
        self.assertEqual(parse_timestamp("2026-09-01T08:00:05"), datetime(2026, 9, 1, 8, 0, 5))
        self.assertEqual(parse_timestamp(" 2026-09-01 08:00:05 "), datetime(2026, 9, 1, 8, 0, 5))

    def test_rejects_other_shapes(self):
        for bad in ("01.09.2026 08:00", "yesterday", "", None):
            with self.assertRaises(TimeError):
                parse_timestamp(bad)


class DurationTests(unittest.TestCase):
    def test_formats_hours_minutes_seconds(self):
        self.assertEqual(format_duration(timedelta(seconds=39)), "0:00:39")
        self.assertEqual(format_duration(timedelta(hours=1, minutes=2, seconds=3)), "1:02:03")
        self.assertEqual(format_duration(timedelta(hours=27)), "27:00:00")

    def test_negative_durations_clamp_to_zero(self):
        self.assertEqual(format_duration(timedelta(seconds=-5)), "0:00:00")

    def test_duration_between_handles_missing_and_reversed_ends(self):
        start, end = datetime(2026, 9, 1, 8, 0, 0), datetime(2026, 9, 1, 8, 0, 9)
        self.assertEqual(duration_between(start, end), timedelta(seconds=9))
        self.assertEqual(duration_between(end, start), timedelta(0))
        self.assertIsNone(duration_between(None, end))
        self.assertIsNone(duration_between(start, None))


if __name__ == "__main__":
    unittest.main()
