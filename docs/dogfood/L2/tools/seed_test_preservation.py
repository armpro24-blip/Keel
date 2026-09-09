"""seed_test_preservation (L2, formal): are all frozen seed test IDs still present?

    python seed_test_preservation.py [REPO]      (default: current directory)

Reads the frozen ID list from ../frozen/seed_test_ids.txt, enumerates the
repository's tests the way `python -m unittest discover -s tests` names them,
and reports present / added / missing IDs and load failures. Exit 0 only when
every seed ID is present and nothing failed to load. Read-only. Unlike L1,
this is part of the L2 acceptance gate (A2).
"""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
FROZEN_IDS = os.path.join(HERE, "..", "frozen", "seed_test_ids.txt")


def discovered_ids(repo):
    ids = []

    def walk(suite):
        for item in suite:
            if isinstance(item, unittest.TestSuite):
                walk(item)
            else:
                ids.append(item.id())

    os.chdir(repo)
    sys.path.insert(0, repo)
    walk(unittest.TestLoader().discover("tests", top_level_dir="tests"))
    return sorted(ids)


def main():
    repo = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.getcwd())
    with open(FROZEN_IDS, encoding="utf-8") as handle:
        seed_ids = [line.strip() for line in handle if line.strip()]
    current = discovered_ids(repo)
    present = [t for t in seed_ids if t in current]
    missing = [t for t in seed_ids if t not in current]
    added = [t for t in current if t not in seed_ids]
    failed_loads = [t for t in current if "_FailedTest" in t]

    print(f"repo: {repo}")
    print(f"discovered {len(current)} test IDs")
    print(f"seed_test_preservation: {len(present)}/{len(seed_ids)} seed tests present; {len(added)} added; {len(missing)} missing")
    for t in missing:
        print(f"  MISSING {t}")
    for t in added:
        print(f"  added   {t}")
    for t in failed_loads:
        print(f"  LOAD FAILURE {t}")
    return 0 if not missing and not failed_loads else 1


if __name__ == "__main__":
    sys.exit(main())
