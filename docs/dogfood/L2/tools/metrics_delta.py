"""metrics_delta: run-local deltas between two saved vLLM /metrics snapshots.

    python metrics_delta.py before.txt after.txt

Reads two Prometheus text snapshots taken immediately before and after a
session and prints, per exact metric name, the before/after values (summed
over label sets) and the delta. The run-local prefix-cache hit rate is derived
only from `vllm:prefix_cache_hits_total` / `vllm:prefix_cache_queries_total`;
`external_*` families are printed on their own lines and never added in;
`*_created` timestamps are ignored. Mean TTFT is derived from
`vllm:time_to_first_token_seconds_sum` / `_count`. Read-only; no server
contact.

Interpretation rules (docs/dogfood/L2/README.md): the deltas are meaningful
only when the server carried no unrelated traffic between the snapshots.
Prompt-token counts from the wire capture are a separate quantity and are
never equated with newly computed prefill tokens.

Revision 2026-09-09: the first version matched families by substring and
summed `vllm:prefix_cache_*`, `vllm:external_prefix_cache_*` and `*_created`
together; the L2 report's derived hit rate is to be re-verified with this
version on the saved snapshots.
"""

import re
import sys

SAMPLE = re.compile(r"^([a-zA-Z_:][a-zA-Z0-9_:]*)(\{[^}]*\})?\s+([-+0-9.eE]+|NaN|[+-]Inf)\s*$")

INTERESTING = ("prefix_cache", "time_to_first_token")

HIT_RATE = ("vllm:prefix_cache_queries_total", "vllm:prefix_cache_hits_total")
TTFT = ("vllm:time_to_first_token_seconds_sum", "vllm:time_to_first_token_seconds_count")


def totals(path):
    """{exact metric name: sum over label sets}, for the interesting families."""
    out = {}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("#"):
                continue
            match = SAMPLE.match(line.strip())
            if not match:
                continue
            name, _labels, value = match.groups()
            if not any(part in name for part in INTERESTING) or name.endswith("_created"):
                continue
            if name.endswith("_bucket"):
                continue
            try:
                out[name] = out.get(name, 0.0) + float(value)
            except ValueError:
                continue
    return out


def main(before_path, after_path):
    before, after = totals(before_path), totals(after_path)
    names = sorted(set(before) | set(after))
    if not names:
        print("no prefix-cache or TTFT metrics found in either snapshot; derived values omitted")
        return
    print(f"{'metric':<50} {'before':>16} {'after':>16} {'delta':>14}")
    delta = {}
    for name in names:
        b, a = before.get(name, 0.0), after.get(name, 0.0)
        delta[name] = a - b
        print(f"{name:<50} {b:>16.3f} {a:>16.3f} {delta[name]:>14.3f}")

    queries, hits = HIT_RATE
    if queries in delta and hits in delta:
        if delta[queries] > 0:
            print(f"run-local prefix-cache hit rate ({hits} / {queries}): "
                  f"{delta[hits] / delta[queries]:.3f} ({delta[hits]:.0f}/{delta[queries]:.0f} tokens)")
        else:
            print("run-local prefix-cache hit rate: undefined (no queries between snapshots)")
    else:
        print("run-local prefix-cache hit rate: omitted (vllm:prefix_cache_{queries,hits}_total not both present)")

    ttft_sum, ttft_count = TTFT
    if ttft_sum in delta and ttft_count in delta:
        if delta[ttft_count] > 0:
            print(f"run-local mean TTFT: {delta[ttft_sum] / delta[ttft_count]:.3f} s over {delta[ttft_count]:.0f} requests")
        else:
            print("run-local mean TTFT: undefined (no requests between snapshots)")
    else:
        print("run-local mean TTFT: omitted (TTFT sum/count not both present)")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
