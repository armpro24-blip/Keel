"""metrics_delta: run-local deltas between two saved vLLM /metrics snapshots.

    python metrics_delta.py before.txt after.txt

Reads two Prometheus text snapshots taken immediately before and after a
session, sums every sample of each relevant metric family across labels,
and prints the deltas: prefix-cache queries and hits (and the run-local hit
rate), and time-to-first-token sum/count (and the mean) where present. Metric
names vary between vLLM versions, so families are matched by substring and
the matched names are printed. Read-only; no server contact.

Interpretation rules (docs/dogfood/L2/README.md): the deltas are meaningful
only when the server carried no unrelated traffic between the snapshots.
Prompt-token counts from the wire capture are a separate quantity and are
never equated with newly computed prefill tokens.
"""

import re
import sys

FAMILIES = {
    "prefix_cache_queries": "prefix_cache_queries",
    "prefix_cache_hits": "prefix_cache_hits",
    "ttft_sum": "time_to_first_token_seconds_sum",
    "ttft_count": "time_to_first_token_seconds_count",
}

SAMPLE = re.compile(r"^([a-zA-Z_:][a-zA-Z0-9_:]*)(\{[^}]*\})?\s+([-+0-9.eE]+|NaN|[+-]Inf)\s*$")


def sums(path):
    totals = {}
    names = {}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("#"):
                continue
            match = SAMPLE.match(line.strip())
            if not match:
                continue
            name, _labels, value = match.groups()
            for key, needle in FAMILIES.items():
                if needle in name:
                    try:
                        totals[key] = totals.get(key, 0.0) + float(value)
                    except ValueError:
                        continue
                    names.setdefault(key, set()).add(name)
    return totals, names


def main(before_path, after_path):
    before, names_before = sums(before_path)
    after, names_after = sums(after_path)
    keys = sorted(set(before) | set(after))
    if not keys:
        print("no prefix-cache or TTFT metric families found in either snapshot; derived values omitted")
        return
    print("family                 before          after           delta        matched names")
    delta = {}
    for key in keys:
        b, a = before.get(key, 0.0), after.get(key, 0.0)
        delta[key] = a - b
        matched = ", ".join(sorted(names_before.get(key, set()) | names_after.get(key, set())))
        print(f"{key:<22} {b:>14.3f}  {a:>14.3f}  {delta[key]:>11.3f}  {matched}")
    queries, hits = delta.get("prefix_cache_queries"), delta.get("prefix_cache_hits")
    if queries is not None and hits is not None:
        if queries > 0:
            print(f"run-local prefix-cache hit rate: {hits / queries:.3f} ({hits:.0f}/{queries:.0f} tokens)")
        else:
            print("run-local prefix-cache hit rate: undefined (no queries between snapshots)")
    if "ttft_sum" in delta and "ttft_count" in delta:
        if delta["ttft_count"] > 0:
            print(f"run-local mean TTFT: {delta['ttft_sum'] / delta['ttft_count']:.3f} s over {delta['ttft_count']:.0f} requests")
        else:
            print("run-local mean TTFT: undefined (no requests between snapshots)")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
