import os
import tempfile
import unittest
from datetime import datetime

from queuewatch.events import KINDS, Event, EventError, parse_line, read_events


def write_log(text):
    handle = tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8", newline="")
    handle.write(text)
    handle.close()
    return handle.name


class ParseLineTests(unittest.TestCase):
    def test_parses_every_field(self):
        event = parse_line(
            3,
            '{"ts": "2026-09-01T08:00:05", "job": " build-api ", "event": "failed", '
            '"worker": "w1", "reason": "out of memory"}',
        )
        self.assertEqual(
            event,
            Event(3, datetime(2026, 9, 1, 8, 0, 5), "build-api", "failed", "w1", "out of memory"),
        )

    def test_optional_fields_default_to_empty_strings(self):
        event = parse_line(1, '{"ts": "2026-09-01T08:00:00", "job": "a", "event": "created"}')
        self.assertEqual((event.worker, event.reason), ("", ""))

    def test_lifecycle_kinds_are_known(self):
        for kind in ("created", "started", "completed", "failed"):
            self.assertIn(kind, KINDS)

    def test_invalid_json_names_the_line(self):
        with self.assertRaises(EventError) as caught:
            parse_line(7, "{not json")
        self.assertTrue(str(caught.exception).startswith("line 7: not valid JSON"))

    def test_non_object_is_rejected(self):
        with self.assertRaises(EventError) as caught:
            parse_line(2, "[1, 2]")
        self.assertEqual(str(caught.exception), "line 2: expected a JSON object")

    def test_missing_or_blank_required_fields_are_rejected(self):
        with self.assertRaises(EventError) as caught:
            parse_line(4, '{"ts": "2026-09-01T08:00:00", "event": "created"}')
        self.assertEqual(str(caught.exception), "line 4: missing field 'job'")
        with self.assertRaises(EventError) as caught:
            parse_line(5, '{"ts": "2026-09-01T08:00:00", "job": "  ", "event": "created"}')
        self.assertIn("'job' must be a non-empty string", str(caught.exception))

    def test_unknown_kind_is_rejected(self):
        with self.assertRaises(EventError) as caught:
            parse_line(9, '{"ts": "2026-09-01T08:00:00", "job": "a", "event": "paused"}')
        self.assertEqual(str(caught.exception), "line 9: unknown event 'paused'")

    def test_bad_timestamp_is_reported_with_the_line(self):
        with self.assertRaises(EventError) as caught:
            parse_line(6, '{"ts": "08:00", "job": "a", "event": "created"}')
        self.assertTrue(str(caught.exception).startswith("line 6: timestamp is not"))


class ReadEventsTests(unittest.TestCase):
    def tearDown(self):
        for path in getattr(self, "paths", []):
            os.unlink(path)

    def log(self, text):
        path = write_log(text)
        self.paths = getattr(self, "paths", []) + [path]
        return path

    def test_reads_in_order_and_skips_blank_lines(self):
        path = self.log(
            '{"ts": "2026-09-01T08:00:00", "job": "a", "event": "created"}\n'
            "\n"
            '{"ts": "2026-09-01T08:00:01", "job": "a", "event": "started"}\n'
        )
        events = read_events(path)
        self.assertEqual([(e.line, e.kind) for e in events], [(1, "created"), (3, "started")])

    def test_a_bad_line_stops_the_read_with_its_number(self):
        path = self.log(
            '{"ts": "2026-09-01T08:00:00", "job": "a", "event": "created"}\n'
            '{"ts": "2026-09-01T08:00:01", "job": "a"}\n'
        )
        with self.assertRaises(EventError) as caught:
            read_events(path)
        self.assertEqual(str(caught.exception), "line 2: missing field 'event'")


if __name__ == "__main__":
    unittest.main()
