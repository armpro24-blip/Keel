# L2: a harder coding-maintenance workload (`queuewatch`)

Status: **L2 run 2026-09-09: passed under the protocol (acceptance 13/13,
seed tests 42/42 preserved, handshake 16/16, host approvals 0), not an
autonomous completion under the default budget: the `max_turns = 32` fuse
blew twice and 2 of 3 `Continue` messages were used**
(`docs/evidence/L2_2026-09-09.md`). **L2-R1 protocol frozen below (T22,
`--max-turns 100`, zero Continue); its run awaits separate approval.**

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
finished again; on retry the duration resets to `-` and afterwards covers
only the new attempt; a retry in any other status is an invalid transition
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
Behavior: A03 retry after failure → queued, attempt 2, duration reset to `-`; A04 each retry
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

Validation before freezing (2026-09-09, repeated after the review
corrections): on the unmodified seed A03–A06, A08–A10 and A13 fail (8
checks; A06's three subtests make unittest report 10 failures) while A01,
A02, A07, A11 and A12 pass, as they should at the seed; with the benchmark author's reference solution,
applied to a scratch copy and never committed into the seed, the seed suite
is 45/45, acceptance 13/13, preservation 42/42 with 3 added. The reference
diff touches 6 files (`events.py`, `state.py`, `report.py`, `cli.py`,
`README.md`, `tests/test_state.py`; 59 insertions, 6 deletions), all
ordinary `edit_file`-shaped changes plus test runs, so the task is solvable
with the current Keel tool set. A fresh `init_l2.sh` after validation
reproduced the frozen tree hash. `grep retry` over the seed's `.py` and
`.md` files returns nothing; only the data file mentions it.

## Pre-registered tested configuration

Frozen values; the run does not start if any of them differs on the lab
machine (stop before sending the task, report the mismatch):

```text
L2 package:   the Keel commit recorded in frozen/seed_info.txt under "L2 package commit"
Keel runtime: src/, tests/ and Cargo files last changed at ba47934 (cargo test: 15 suites, 95 tests)
PIRA:         4e0682dd745f1dbafa772d9c11b369132db4c1a8 at ~/agent (AGENTS.md sha256 e6c7d630…)
model:        nvidia/Qwen3.6-35B-A3B-NVFP4
vLLM:         0.26.0 (the L1-R2 configuration, unchanged)
mode/flags:   full; --trace --record-wire; max_turns = 32 (default, not raised)
```

No compaction, retries, extra tools, hints, or serving-configuration
changes. No hint about
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

## L2-R1: the same task under an explicit budget, no operator continuation

Purpose: L2 passed only with two operator `Continue` messages after the
default fuse (`max_turns = 32`) blew twice. L2-R1 asks whether the same task
completes autonomously under an explicitly authorized, finite budget with no
continuation at all. The original L2 result stays as recorded.

Frozen values (the run does not start if any differs on the lab machine):

```text
L2 seed / task / acceptance / preservation:  unchanged (seed 5d668de, frozen/task.md, 13 checks, 42 IDs)
Keel runtime:  15181da (T22 only on top of ba47934; cargo test 15 suites)
PIRA:          4e0682dd745f1dbafa772d9c11b369132db4c1a8 (AGENTS.md sha256 e6c7d630…)
model / vLLM:  nvidia/Qwen3.6-35B-A3B-NVFP4 / 0.26.0
mode / flags:  full; --trace --record-wire; --max-turns 100
Continue:      none. A run that exhausts the budget is the result; no budget is added during the run.
```

Budget basis, fixed in advance: **"This experiment pre-authorizes at most
100 model calls for the single task. It is an experimental resource
allowance, not a prediction of the calls needed to complete, and it
guarantees no time or context bound."** 100 was chosen after L2's 76 calls
were known; it is not a fitted success threshold, and a single success under
it is not to be read as an optimal budget.

Procedure differences from L2: step 1 also checks `git log -1 --format=%h --
src tests Cargo.toml Cargo.lock` = `15181da` and that the REPL prints
`[max_turns] 100 per user message` at startup; step 4 adds `--max-turns
100`; step 5 sends no `Continue`: when the run ends with
`error: model-call budget exhausted (max_turns = 100); the run is
incomplete; the transcript is kept`, or with a declared completion, send
`/quit`. A question from the model is still answered only from the frozen
text. Everything else, including the metrics snapshots, is as in L2.

Pre-registered evidence: as for L2 plus: whether the budget was exhausted;
`session_start.max_turns`; `run_end` (`turns`, and `error` if exhausted);
if exhausted, the state of the repository (acceptance and preservation are
still run on it and reported as-is). Comparison against L2: acceptance,
preservation, handshake, calls by tool, model calls, tokens, wall time,
fused or not, interventions (expected: task and `/quit` only).

Interpretation, fixed in advance: completion with acceptance 13/13 and
preservation 42/42 under 100 calls with zero continuation → record that the
task completes autonomously under an explicit budget on this model; not a
statement about the default. Exhaustion → record the incomplete state as
the result; it is evidence about the task and the model's call efficiency,
not a reason to raise N afterwards. Report: `docs/evidence/L2R1_<date>.md`;
stop for review.

## Interpretation rule

A concrete failure → freeze the evidence, classify the dominant failure
before proposing a mechanism, stop; no harder workload first. A complete
pass → record the pass and the remaining frictions, stop for review; no
features added. The report is `docs/evidence/L2_<date>.md`.
