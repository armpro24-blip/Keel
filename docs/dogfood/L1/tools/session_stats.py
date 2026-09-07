"""Counts from a Keel session log (JSONL), for a dogfood report.

    python session_stats.py <session>.jsonl

Prints runs, model turns, tool calls by tool and verdict, error observations,
denial reasons, and wall time. Reads only the log; changes nothing.
"""

import json
import sys
from collections import Counter


def main(path):
    events = [json.loads(line) for line in open(path, encoding="utf-8") if line.strip()]
    runs = [e for e in events if e.get("event") == "run_end"]
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
    print(f"runs (user turns) {len(runs)}")
    print(f"model calls       {sum(r['turns'] for r in runs)}")
    print(f"tool calls        {len(decisions)}  by tool {dict(by_tool)}")
    print(f"verdicts          {dict(verdicts)}")
    print(f"handshakes        {dict(handshakes)}  (effect, review_present) -> count")
    print(f"tool results      {len(results)}  errors {len(errors)}")
    print(f"wall time         {(max(times) - min(times)) / 1000:.1f} s")
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
