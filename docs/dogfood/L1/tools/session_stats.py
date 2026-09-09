"""Counts from a Keel session log (JSONL), for a dogfood report.

    python session_stats.py <session>.jsonl

Prints runs (including runs that ended in an error), model calls, tool calls
by tool and verdict, error observations, denial reasons, and wall time.
Reads only the log; changes nothing.

A run is completed when its run_end has no `error`, failed otherwise; the
presence of `turns` says nothing about success (since T22 an exhausted
budget records its `turns` too). Model calls are summed over every run_end
that carries `turns`; a failed run without `turns` (older logs, or a
non-budget failure) makes the count incomplete, which is reported instead
of guessed.
"""

import json
import sys
from collections import Counter


def main(path):
    events = [json.loads(line) for line in open(path, encoding="utf-8") if line.strip()]
    runs = [e for e in events if e.get("event") == "run_end"]
    completed = [r for r in runs if "error" not in r]
    failed = [r for r in runs if "error" in r]
    failed_runs = [r["error"] for r in failed]
    counted = [r for r in runs if isinstance(r.get("turns"), int)]
    uncounted = len(runs) - len(counted)
    budget = next((e.get("max_turns") for e in events if e.get("event") == "session_start"), None)
    decisions = [e for e in events if e.get("event") == "decision"]
    messages = [e for e in events if e.get("event") == "message"]

    by_tool = Counter(d["tool"] for d in decisions)
    verdicts = Counter("allow" if d["decision"] == "allow" else "deny" for d in decisions)
    denials = [d["decision"]["deny"] for d in decisions if d["decision"] != "allow"]
    handshakes = Counter(
        (d["handshake"]["effect"], d["handshake"]["review_present"])
        for d in decisions
        if "handshake" in d
    )
    results = [
        block
        for m in messages
        for block in m["blocks"]
        if block["type"] == "tool_result"
    ]
    errors = [r for r in results if r["is_error"]]
    times = [e["t"] for e in events if "t" in e]

    print(f"events            {len(events)}")
    print(f"runs (user turns) {len(runs)}  completed {len(completed)}  failed {len(failed)}  max_turns {budget if budget is not None else 'not recorded'}")
    calls = sum(r["turns"] for r in counted)
    if uncounted:
        print(f"model calls       at least {calls}  (INCOMPLETE: {uncounted} run(s) recorded no call count)")
    else:
        print(f"model calls       {calls}")
    print(f"tool calls        {len(decisions)}  by tool {dict(by_tool)}")
    print(f"verdicts          {dict(verdicts)}")
    print(f"handshakes        {dict(handshakes)}  (effect, review_present) -> count")
    print(f"tool results      {len(results)}  errors {len(errors)}")
    print(f"wall time         {(max(times) - min(times)) / 1000:.1f} s")
    for error in failed_runs:
        print(f"run error: {error}")
    if denials:
        print("denials:")
        for reason in denials:
            print(f"  - {reason}")
    if errors:
        print("error observations (first line each):")
        for r in errors:
            print(f"  - {r['call_id']}: {r['output'].splitlines()[0][:120]}")


if __name__ == "__main__":
    main(sys.argv[1])
