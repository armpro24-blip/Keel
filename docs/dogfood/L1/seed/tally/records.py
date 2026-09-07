"""Reading expense records from a CSV file.

A file has a header row with at least the columns ``date``, ``category`` and
``amount``; ``note`` is optional. Every data row becomes one ``Record`` or
raises ``RecordError`` naming the line, so a bad export fails loudly instead
of producing a wrong total.
"""

import csv
from dataclasses import dataclass
from datetime import date


class RecordError(ValueError):
    """A row that cannot become a record; the message names what is wrong."""


@dataclass(frozen=True)
class Record:
    day: date
    category: str
    amount: float
    note: str = ""


REQUIRED_COLUMNS = ("date", "category", "amount")


def parse_amount(text):
    """Parse an amount such as ``12.50`` into a float."""
    try:
        return float(text.strip())
    except ValueError:
        raise RecordError(f"amount is not a number: {text!r}") from None


def parse_date(text):
    """Parse a ``YYYY-MM-DD`` date."""
    try:
        return date.fromisoformat(text.strip())
    except ValueError:
        raise RecordError(f"date is not YYYY-MM-DD: {text!r}") from None


def read_records(path):
    """Read every record in the CSV file at ``path``, in file order."""
    with open(path, newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        present = reader.fieldnames or []
        missing = [column for column in REQUIRED_COLUMNS if column not in present]
        if missing:
            raise RecordError("missing columns: " + ", ".join(missing))
        records = []
        for line_number, row in enumerate(reader, start=2):
            try:
                records.append(
                    Record(
                        day=parse_date(row["date"]),
                        category=row["category"].strip().lower(),
                        amount=parse_amount(row["amount"]),
                        note=(row.get("note") or "").strip(),
                    )
                )
            except RecordError as error:
                raise RecordError(f"line {line_number}: {error}") from None
        return records
