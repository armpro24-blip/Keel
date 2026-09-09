"""Reading job events from a JSON Lines log.

One JSON object per line with the fields ``ts`` (timestamp), ``job``
(identifier) and ``event`` (kind); ``worker`` and ``reason`` are optional.
A line that cannot become an event raises ``EventError`` naming the line,
so a corrupt export fails loudly rather than producing a wrong report.
"""

import json
from dataclasses import dataclass
from datetime import datetime

from .timeutil import TimeError, parse_timestamp

#: Event kinds the log may contain, in lifecycle order.
KINDS = ("created", "started", "completed", "failed")


class EventError(ValueError):
    """A line that cannot become an event; the message names the line."""


@dataclass(frozen=True)
class Event:
    line: int
    ts: datetime
    job: str
    kind: str
    worker: str = ""
    reason: str = ""


def parse_line(line_number, text):
    """Turn one log line into an ``Event``."""
    try:
        record = json.loads(text)
    except json.JSONDecodeError as error:
        raise EventError(f"line {line_number}: not valid JSON ({error.msg})") from None
    if not isinstance(record, dict):
        raise EventError(f"line {line_number}: expected a JSON object")
    for field in ("ts", "job", "event"):
        if field not in record:
            raise EventError(f"line {line_number}: missing field {field!r}")
        if not isinstance(record[field], str) or not record[field].strip():
            raise EventError(f"line {line_number}: field {field!r} must be a non-empty string")
    kind = record["event"].strip()
    if kind not in KINDS:
        raise EventError(f"line {line_number}: unknown event {kind!r}")
    try:
        ts = parse_timestamp(record["ts"])
    except TimeError as error:
        raise EventError(f"line {line_number}: {error}") from None
    return Event(
        line=line_number,
        ts=ts,
        job=record["job"].strip(),
        kind=kind,
        worker=str(record.get("worker") or "").strip(),
        reason=str(record.get("reason") or "").strip(),
    )


def read_events(path):
    """Read every event in file order; blank lines are skipped."""
    events = []
    with open(path, encoding="utf-8") as handle:
        for line_number, text in enumerate(handle, start=1):
            if text.strip():
                events.append(parse_line(line_number, text))
    return events
