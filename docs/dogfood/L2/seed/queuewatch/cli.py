"""Command line: ``queuewatch report FILE`` and ``queuewatch check FILE``."""

import argparse
import sys

from .events import EventError, read_events
from .report import render_report
from .state import TransitionError, build_jobs

PROG = "queuewatch"


def build_parser():
    parser = argparse.ArgumentParser(
        prog=PROG, description="Reconstruct job state from an event log and report on it."
    )
    commands = parser.add_subparsers(dest="command", required=True)

    report = commands.add_parser("report", help="one row per job plus a summary line")
    report.add_argument("path", help="JSON Lines event log")

    check = commands.add_parser("check", help="validate a log without printing a report")
    check.add_argument("path", help="JSON Lines event log")
    return parser


def load_jobs(path):
    """Events → jobs, or a data error the caller reports."""
    return build_jobs(read_events(path))


def main(argv=None):
    args = build_parser().parse_args(argv)
    try:
        jobs = load_jobs(args.path)
    except (OSError, EventError, TransitionError) as error:
        print(f"{PROG}: {error}", file=sys.stderr)
        return 1

    if args.command == "check":
        events = sum(len(job.history) for job in jobs.values())
        print(f"ok: {events} events, {len(jobs)} jobs")
        return 0

    print(render_report(list(jobs.values())))
    return 0
