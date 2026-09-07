"""Aggregating records into a per-category report."""

from collections import defaultdict


def totals_by_category(records):
    """Sum amounts per category; largest total first, ties by name."""
    totals = defaultdict(float)
    for record in records:
        totals[record.category] += record.amount
    ordered = sorted(totals.items(), key=lambda item: (-item[1], item[0]))
    return dict(ordered)


def render(totals):
    """One line per category, then a ``total`` line.

    Each line is the name left-aligned in 12 columns followed by the amount
    with two decimals right-aligned in 10 columns, so the output can be read
    by eye and split on whitespace by scripts.
    """
    lines = [f"{name:<12}{amount:>10.2f}" for name, amount in totals.items()]
    lines.append(f"{'total':<12}{sum(totals.values()):>10.2f}")
    return "\n".join(lines)
