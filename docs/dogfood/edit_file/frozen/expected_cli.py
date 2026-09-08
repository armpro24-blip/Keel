"""Command-line entry point: ``python -m tally report FILE [--top N]``."""

import argparse
import re
import sys

from .records import RecordError, read_records
from .report import render, totals_by_category


def month(text):
    """Validate a ``YYYY-MM`` month for argparse."""
    if not re.fullmatch(r"\d{4}-(0[1-9]|1[0-2])", text):
        raise argparse.ArgumentTypeError(f"month must be YYYY-MM, got {text!r}")
    return text


def build_parser():
    parser = argparse.ArgumentParser(
        prog="tally", description="Summarize CSV expense records by category."
    )
    commands = parser.add_subparsers(dest="command", required=True)
    report = commands.add_parser("report", help="print totals per category")
    report.add_argument("path", help="CSV file with date, category, amount[, note]")
    report.add_argument(
        "--top",
        type=int,
        metavar="N",
        help="show only the N largest categories (the total still covers all)",
    )
    report.add_argument(
        "--month",
        type=month,
        metavar="YYYY-MM",
        help="only records whose date falls in this month",
    )
    return parser


def main(argv=None):
    args = build_parser().parse_args(argv)
    try:
        records = read_records(args.path)
    except (OSError, RecordError) as error:
        print(f"tally: {error}", file=sys.stderr)
        return 1
    totals = totals_by_category(records)
    shown = totals
    if args.top is not None:
        shown = dict(list(totals.items())[: args.top])
    output = render(shown)
    if shown is not totals:
        # The total line must still cover every category, not just the shown ones.
        lines = output.split("\n")
        lines[-1] = f"{'total':<12}{sum(totals.values()):>10.2f}"
        output = "\n".join(lines)
    print(output)
    return 0
