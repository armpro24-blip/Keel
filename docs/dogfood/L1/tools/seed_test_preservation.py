"""seed_test_preservation: are all 13 seed test IDs still present after a run?

    python seed_test_preservation.py [REPO]      (default: current directory)

Enumerates the unittest IDs the way `python -m unittest discover -s tests`
names them (tests/ as the top-level directory) and compares them with the
frozen list of the L1 seed's 13 tests. Diagnostic only: it is not part of the
historical 7-test acceptance gate. Read-only.

Run it twice: on the fresh seed before the session (must report 13/13
present, 0 added) and on the tree the model left behind.
"""

import os
import sys
import unittest

SEED_TEST_IDS = [
    "test_cli.CliTests.test_missing_file_is_an_error_message_not_a_traceback",
    "test_cli.CliTests.test_report_prints_totals_and_exits_zero",
    "test_cli.CliTests.test_top_limits_categories_but_not_the_total",
    "test_records.ParseTests.test_amount_accepts_plain_decimals",
    "test_records.ParseTests.test_amount_rejects_non_numbers",
    "test_records.ParseTests.test_date_accepts_iso_and_rejects_others",
    "test_records.ReadRecordsTests.test_bad_row_names_its_line",
    "test_records.ReadRecordsTests.test_missing_column_is_reported",
    "test_records.ReadRecordsTests.test_reads_rows_in_order_and_normalizes_category",
    "test_report.RenderTests.test_empty_report_is_just_the_total",
    "test_report.RenderTests.test_lines_are_aligned_and_end_with_total",
    "test_report.TotalsTests.test_sums_per_category_largest_first",
    "test_report.TotalsTests.test_ties_are_ordered_by_name",
]


def discovered_ids(repo):
    ids = []

    def walk(suite):
        for item in suite:
            if isinstance(item, unittest.TestSuite):
                walk(item)
            else:
                ids.append(item.id())

    # Discover from inside the repository, as `python -m unittest discover -s tests`
    # does when run from the repository root, so `tally` is importable.
    os.chdir(repo)
    sys.path.insert(0, repo)
    walk(unittest.TestLoader().discover("tests", top_level_dir="tests"))
    return sorted(ids)


def main():
    repo = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.getcwd())
    current = discovered_ids(repo)
    present = [t for t in SEED_TEST_IDS if t in current]
    missing = [t for t in SEED_TEST_IDS if t not in current]
    added = [t for t in current if t not in SEED_TEST_IDS]
    # A load failure appears as unittest.loader._FailedTest.<module>; report it.
    failed_loads = [t for t in current if "_FailedTest" in t]

    print(f"repo: {repo}")
    print(f"discovered {len(current)} test IDs")
    print(f"seed_test_preservation: {len(present)}/{len(SEED_TEST_IDS)} seed tests present; {len(added)} added; {len(missing)} missing")
    for t in missing:
        print(f"  MISSING {t}")
    for t in added:
        print(f"  added   {t}")
    for t in failed_loads:
        print(f"  LOAD FAILURE {t}")
    return 0 if not missing and not failed_loads else 1


if __name__ == "__main__":
    sys.exit(main())
