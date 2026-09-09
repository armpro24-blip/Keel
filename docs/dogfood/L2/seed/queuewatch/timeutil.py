"""Timestamps and durations.

Event timestamps are naive ISO-8601 (``YYYY-MM-DDTHH:MM:SS``); the log is
assumed to come from one clock. Durations are rendered as ``H:MM:SS`` so the
report is stable across platforms and locales.
"""

from datetime import datetime, timedelta


class TimeError(ValueError):
    """A timestamp that cannot be read."""


def parse_timestamp(text):
    """Parse ``YYYY-MM-DDTHH:MM:SS`` (a space instead of ``T`` is accepted)."""
    try:
        return datetime.fromisoformat(text.strip())
    except (ValueError, AttributeError):
        raise TimeError(f"timestamp is not YYYY-MM-DDTHH:MM:SS: {text!r}") from None


def format_duration(delta):
    """``timedelta`` → ``H:MM:SS``; negative durations are clamped to zero."""
    seconds = max(int(delta.total_seconds()), 0)
    hours, remainder = divmod(seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    return f"{hours}:{minutes:02d}:{seconds:02d}"


def duration_between(start, end):
    """Duration from ``start`` to ``end``, or ``None`` when either is missing."""
    if start is None or end is None:
        return None
    return end - start if end >= start else timedelta(0)
