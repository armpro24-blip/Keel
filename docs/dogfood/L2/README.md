# L2: a harder coding-maintenance workload (`queuewatch`)

Status: **benchmark-design package ready for review (2026-09-09). Not yet
run.** Keel is unchanged; no model has been called for L2.

## Purpose

Test whether the current Keel baseline supports cross-file understanding,
state reasoning, iterative repair, regression preservation, verification,
and documentation in one autonomous coding task. If it fails, the observed
failure selects the next mechanism. L2 is not built around any proposed
feature (stdin, `write_file`, `apply_patch`, compaction, MCP, Skills,
subagents, retries) and contains no trap for Keel.

## Repository

`queuewatch` reconstructs batch-job state from a JSON Lines event log and
reports on it. Standard library only; deterministic on Windows and Linux;
no network; no generated files.

```text
queuewatch/            production (8 files, ~330 lines)
  __init__.py  __main__.py
  timeutil.py          timestamps, H:MM:SS durations
  events.py            JSONL parsing, Event, EventError with line numbers, KINDS
  model.py             Job (status, attempt, timestamps, history), STATUSES
  state.py             the state machine: apply(), build_jobs(), TransitionError
  report.py            rows, summary line, table rendering, render_report()
  cli.py               `report FILE`, `check FILE`, exit codes 0/1/2
tests/                 5 files, 42 tests
  test_timeutil.py  test_events.py  test_state.py  test_report.py  test_cli.py
data/
  events.jsonl                 7 jobs, no retries; the README sample
  production_2026-09.jsonl     contains `retry` events; `report` on it currently fails at line 5
README.md
```

Seed state machine: `(no job) --created--> queued --started--> running
--completed--> completed | --failed--> failed`; `attempt` is 1 from creation;
any other event, unknown kind, or malformed line is a data error naming the
line and job, exit 1, reported as `queuewatch: <message>`; usage errors exit
2. The requested feature is absent: `retry` is an unknown event kind and
`--status` does not exist.

## Frozen artifacts (`frozen/`)

| Item | Value |
|---|---|
| seed commit | `5d668de873a86ec0e07a967d850133167a4919bb` |
| seed tree | `c050c5d3edc5b96864eca11366d39da2508ed485` (reproduced by two independent `init_l2.sh` runs) |
| seed tests | 42, IDs in `frozen/seed_test_ids.txt` |
| authoring Python | 3.14.0 (Windows); the lab runs 3.12.10 |
| task text | `frozen/task.md` |
| legacy report | `frozen/report_events_expected.txt` (`report data/events.jsonl` at the seed) |

Creation: `bash <Keel>/docs/dogfood/L2/init_l2.sh TARGET` (fixed author and
date; prints commit and tree; runs the seed suite).

## Frozen task

See `frozen/task.md`. In substance: add `retry` (valid only from `failed`;
increments `attempt`; returns the job to `queued` so it can be started and
finished again; a retry in any other status is an invalid transition
reported like the existing ones, no traceback); add `--status STATUS` to
`report` (filters rows; the summary still counts all jobs; unknown STATUS is
a usage error; output unchanged without the option); update the README; add
tests; existing tests must keep passing and must not be removed or renamed.
No file names, hints, or hidden-test expectations are given. The task needs
edits in the parser's kind list, the state machine, the report or CLI
filter, the CLI option, the README, and the tests, and is solvable without
creating any new source or test file.

## Hidden acceptance (`acceptance/test_acceptance.py`, 13 checks)

Formal gates: A01 the project suite passes; A02 **every frozen seed test ID
is still present** (identity, not count; `tools/seed_test_preservation.py`).
Behavior: A03 retry after failure → queued, attempt 2; A04 each retry
increments the attempt (production log: ingest completed/2,
transform completed/3, publish queued/2, cleanup queued/1, summary
`total 4  queued 2  running 0  completed 2  failed 0`); A05 a retried job
starts and completes with the new attempt's duration; A06 retry from queued,
running, completed → data error naming line and job, exit 1, no traceback;
A07 existing invalid transitions unchanged; A08 `--status failed` and A09
`--status completed` limit rows, summary unchanged; A10 `--status` with no
matching jobs → header and summary only; A11 unknown STATUS → exit 2, no
traceback; A12 report without `--status` equals the frozen legacy output and
`check` still says `ok: 18 events, 7 jobs`; A13 README mentions `retry` and
`--status`.

Validation before freezing (2026-09-09): on the unmodified seed 10 of 13
fail (A01, A02, A12 pass); with the benchmark author's reference solution,
applied to a scratch copy and never committed into the seed, the seed suite
is 45/45, acceptance 13/13, preservation 42/42 with 3 added. The reference
diff touches 6 files (`events.py`, `state.py`, `report.py`, `cli.py`,
`README.md`, `tests/test_state.py`; 59 insertions, 6 deletions), all
ordinary `edit_file`-shaped changes plus test runs, so the task is solvable
with the current Keel tool set. A fresh `init_l2.sh` after validation
reproduced the frozen tree hash. `grep retry` over the seed's `.py` and
`.md` files returns nothing; only the data file mentions it.

## Pre-registered tested configuration

```text
Keel: post-L1 baseline b0312d2 (code ba47934) or later reviewed baseline, unchanged
PIRA: canonical baseline at ~/agent (record commit and AGENTS.md sha256)
model: nvidia/Qwen3.6-35B-A3B-NVFP4; vLLM: existing lab configuration (record version)
mode: full          flags: --trace --record-wire          max_turns: existing default (32)
```

No compaction, retries, extra tools, or other mechanisms. No hint about
`edit_file`, safety reviews, PIRA tools, or workflow beyond the normal Keel
and PIRA instruction and tool schemas. If the fuse or context behavior
becomes the failure, that is evidence.

## Procedure

1. Environment: `git pull --ff-only && git rev-parse HEAD && cargo build -q && cargo test 2>&1 | grep -c "test result: ok"`;
   `git -C ~/agent rev-parse HEAD && sha256sum ~/agent/AGENTS.md`; `python --version`;
   `curl -s http://192.168.3.103:8000/version`.
2. `bash <Keel>/docs/dogfood/L2/init_l2.sh ~/Desktop/queuewatch-l2` (commit must be `5d668de`);
   `python <Keel>/docs/dogfood/L2/tools/seed_test_preservation.py ~/Desktop/queuewatch-l2` → 42/42, 0 added.
3. Serving metrics, only if the server exposes them and carries no other
   traffic during the run: `curl -s http://192.168.3.103:8000/metrics > ~/Desktop/l2_metrics_before.txt`
   immediately before starting Keel (and `_after.txt` immediately after `/quit`).
   Do not restart or reconfigure vLLM for this.
4. From inside the repository:
   `<Keel>/target/debug/keel --model "nvidia/Qwen3.6-35B-A3B-NVFP4" --trace --record-wire --full 2>&1 | tee ~/Desktop/l2_session.txt`.
   Send `frozen/task.md`'s text as one line (paragraphs joined by single
   spaces, content unchanged). Note start time.
5. Operator rules as in L1: no hints, corrections, or commands; answer a
   question only from the frozen text; `Continue with the task as specified.`
   at most three times when a turn ends without completion or question; an
   `approve?` prompt (outside-workspace only, in full mode) gets `n` and is
   recorded; `/quit` on declared completion, after the third Continue, or at
   45 minutes. Record every intervention with time and text.
6. Afterwards, from the repository root:

   ```bash
   git status --short --untracked-files=all && git diff > ~/Desktop/l2_diff.patch && git diff --stat
   python -m unittest discover -s tests -v 2>&1 | tail -5
   python <Keel>/docs/dogfood/L2/acceptance/test_acceptance.py -v 2>&1 | tail -20
   python <Keel>/docs/dogfood/L2/tools/seed_test_preservation.py ~/Desktop/queuewatch-l2
   python <Keel>/docs/dogfood/L1/tools/session_stats.py "<[log] path>"
   python <Keel>/docs/dogfood/L1/tools/usage_from_wire.py "<[wire] path>"
   python <Keel>/docs/dogfood/L1/tools/full_mode_check.py "<[log] path>" ~/Desktop/l2_session.txt
   python <Keel>/docs/dogfood/L2/tools/metrics_delta.py ~/Desktop/l2_metrics_before.txt ~/Desktop/l2_metrics_after.txt
   pira_ctx history --scope workspace --limit 100
   <Keel>/target/debug/keel log show "<[log] path>" > ~/Desktop/l2_logshow.txt
   ```

   Leave the repository uncommitted as the model left it; the wire file
   stays on the machine.

## Pre-registered evidence

Primary: hidden acceptance (13 checks, A01–A13) and seed-test preservation
(A02, formal). Operational: model calls; total tool calls and by tool;
`edit_file` successes and errors; zero-match and multi-match recovery;
malformed calls; PIRA policy loads; shell errors; tests run by the model;
final project suite; final diff; prompt-token maximum and sum;
completion-token sum; context growth; wall time; human interventions;
questions and Continue messages; untracked or temp leftovers; SessionLog
evidence; `pira_ctx history`; full-mode handshake ordering for every
state-changing action. No new scoring metric is created after the run.

Serving performance, separately from the prompt-token workload: run-local
deltas of prefix-cache queries and hits (and the hit rate) and TTFT sum and
count (and the mean), from the before/after snapshots, only if the counters
were not shared with other traffic. Prompt-token sums are never equated with
newly computed prefill tokens. Without a clean baseline the derived values
are omitted and the reason stated.

## Interpretation rule

A concrete failure → freeze the evidence, classify the dominant failure
before proposing a mechanism, stop; no harder workload first. A complete
pass → record the pass and the remaining frictions, stop for review; no
features added. The report is `docs/evidence/L2_<date>.md`.
