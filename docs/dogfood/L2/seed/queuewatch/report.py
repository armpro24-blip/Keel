"""Rendering jobs as a report: one row per job, then a summary line."""

from .model import STATUSES
from .timeutil import duration_between, format_duration

COLUMNS = ("job", "status", "attempt", "duration")


def job_row(job):
    """The four cells for one job; duration covers the current attempt."""
    duration = duration_between(job.started_at, job.finished_at)
    return (
        job.id,
        job.status,
        str(job.attempt),
        format_duration(duration) if duration is not None else "-",
    )


def summary_counts(jobs):
    """``{status: count}`` over all jobs, every status present."""
    counts = {status: 0 for status in STATUSES}
    for job in jobs:
        counts[job.status] += 1
    return counts


def render_summary(jobs):
    counts = summary_counts(jobs)
    parts = [f"total {len(jobs)}"] + [f"{status} {counts[status]}" for status in STATUSES]
    return "  ".join(parts)


def render_table(rows):
    """Header plus rows, columns left-aligned to their widest cell."""
    all_rows = [COLUMNS] + [tuple(row) for row in rows]
    widths = [max(len(row[index]) for row in all_rows) for index in range(len(COLUMNS))]
    lines = []
    for row in all_rows:
        cells = [cell.ljust(widths[index]) for index, cell in enumerate(row)]
        lines.append("  ".join(cells).rstrip())
    return "\n".join(lines)


def render_report(jobs):
    """Rows for every job sorted by id, then the summary over all jobs."""
    ordered = sorted(jobs, key=lambda job: job.id)
    return render_table(job_row(job) for job in ordered) + "\n" + render_summary(jobs)
