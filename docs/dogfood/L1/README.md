# L1: first long-horizon dogfood workload

Phase: long-horizon Keel dogfooding (PLAN.md §10). Goal: expose the next
real limitation of the harness by running a realistic coding task through
Keel, not by picking a feature from the backlog. No mechanism is selected in
advance; nothing is added to help the model after problems are seen.

## Baseline

| Item | Value |
|---|---|
| Keel code | `1d534ce` |
| Keel docs / T15 closure | `17884ba` |
| PIRA | current validated `master` at `~/agent` (record `git -C ~/agent rev-parse HEAD`) |
| Model | `nvidia/Qwen3.6-35B-A3B-NVFP4` (record vLLM version) |
| Approval mode | `ask` (default) |
| Workload repository | `tally`, created from `seed/` by `init_l1.sh`; expected seed commit `f5b688fc212acd8b5dcca7dc6414e8990a65ded1` (tree `65833582…`) when the tree is byte-identical; a different hash means line-ending drift: record it and proceed |

## The workload

`tally` is a ~150-line Python package (standard library only): read CSV
expense records, print totals per category. It has a 13-test suite that
passes at the seed commit. The task below is an ordinary feature-and-fix
request; it needs repository inspection, understanding three modules,
changes to at least three files (code, tests, README), a test loop, and a
final verification. Expected size: roughly 15–40 tool calls. Nothing in the
repository is a Keel-specific trap.

### Frozen task text

Paste exactly this as the first and only task message:

```text
Two changes are needed in this repository (tally, a small CSV expense summarizer).

1. `tally report` must accept a `--month YYYY-MM` option that limits the report to records whose date falls in that month. Without the option, behavior is unchanged. An invalid value (not YYYY-MM) must be reported as a usage error with a non-zero exit, not a traceback.

2. Amounts written with thousands separators, such as "1,250.00", are currently rejected (try `python -m tally report data/expenses.csv`). They must parse as 1250.00. Amounts that are not numbers must still be rejected with the existing error.

Update the README's usage section for the new option, add tests for both changes, and make sure the whole suite passes with `python -m unittest discover -s tests -v`. Finish with a short summary of what you changed and the test result.
```

### Acceptance (hidden from the model; run afterwards)

`acceptance/test_acceptance.py`, black-box through the command line and the
files: the project's own suite passes; the data file reports `rent 2500.00`
and `total 3051.85`; `--month 2026-07` yields exactly rent 1250.00,
groceries 115.95, transport 32.00, total 1397.95; `--month 2026-08` yields
utilities 90.40, groceries 121.45, total 1493.85; `2026-8` and `August` exit
non-zero without a traceback and with empty stdout; a non-numeric amount
still exits non-zero without a traceback; the README mentions `--month`.
Validated before freezing: on the seed, A2, A3, A4, A7 fail and the rest
pass; with a reference fix (not published) all seven pass.

L1 **passes** when all seven acceptance tests pass on the working tree the
model left behind, with no human edit to the repository. Anything else is a
fail, and the reason is the finding.

## Procedure

1. Update and record the environment:

   ```bash
   cd <Keel> && git pull --ff-only && git rev-parse HEAD && cargo build -q && cargo test 2>&1 | grep -c "test result: ok"
   git -C ~/agent rev-parse HEAD && sha256sum ~/agent/AGENTS.md
   python --version && git --version
   curl -s http://192.168.3.103:8000/version && curl -s -H "Authorization: Bearer dummy" http://192.168.3.103:8000/v1/models
   ```

2. Create the repository outside Keel and confirm the seed suite passes:

   ```bash
   bash <Keel>/docs/dogfood/L1/init_l1.sh ~/Desktop/tally-l1
   ```

3. Run Keel **from inside the L1 repository** (Keel's workspace is the
   nearest Git root of its cwd), interactively, in the default `ask` mode,
   with the wire capture on (local only, for token counts):

   ```bash
   cd ~/Desktop/tally-l1
   export OPENAI_BASE_URL=http://192.168.3.103:8000/v1 OPENAI_API_KEY=dummy
   <Keel>/target/debug/keel --model "nvidia/Qwen3.6-35B-A3B-NVFP4" --trace --record-wire 2>&1 | tee ~/Desktop/l1_session.txt
   ```

   Note the `[log]` and `[wire]` paths printed at start. Paste the frozen
   task text as the first message. Note the wall-clock start.

4. Operator rules while it runs (each deviation is an intervention; record
   it verbatim with the time):

   - Approve (`y`) every action whose working directory is inside the
     repository and whose command is within the task. Decline (`n`) only a
     command that would act outside both the repository and the platform
     temp directory (PIRA's standing exception for task-local temporary
     files), or destroy work unrelated to the task; record the reason.
   - Never type hints, corrections, file contents, or commands.
   - If the model ends a turn with a question, answer only from the frozen
     task text (quote the relevant sentence), nothing more.
   - If the model ends a turn without declaring completion and without a
     question, send exactly `Continue with the task as specified.` This may
     be sent at most three times in the whole session.
   - When the model declares completion, or after the third continue, or
     after 45 minutes of wall clock, send `/quit`.

5. After the session, from the repository root:

   ```bash
   git status --short && git diff > ~/Desktop/l1_diff.patch && git diff --stat
   python -m unittest discover -s tests -v 2>&1 | tail -5
   python <Keel>/docs/dogfood/L1/acceptance/test_acceptance.py -v 2>&1 | tail -15
   pira_ctx history --scope workspace --limit 100
   python <Keel>/docs/dogfood/L1/tools/session_stats.py "<the [log] path>"
   python <Keel>/docs/dogfood/L1/tools/usage_from_wire.py "<the [wire] path>"
   <Keel>/target/debug/keel log show "<the [log] path>" > ~/Desktop/l1_logshow.txt
   ```

   Then commit nothing in the L1 repository; leave the tree as the model
   left it until the report is reviewed.

## Preserve and report

- The session transcript (`l1_session.txt`), `keel log show` output, the
  diff, both test outputs, `pira_ctx history`, `session_stats` and
  `usage_from_wire` output (numbers only; the wire file stays on the
  machine).
- Every intervention: approvals declined, continues sent, questions answered,
  with the time and the exact text.
- Wall clock start and end.
- Environment values from step 1 and the seed commit hash from step 2.
- Anything the model or a tool did that looked wrong, even if the run
  passed.

## Result

L1 was run on 2026-09-07 and **failed** (acceptance 0/7). Evidence and the
classified observations: `docs/evidence/L1_2026-09-07.md`. The corrections
above to rule 4, the history limit, and `session_stats.py` were made after
that run; the task text, seed, and acceptance tests are unchanged.

## Review by observed failure

The report is read against these categories, none of which is presumed:
context degradation or forgetting; unreliable shell editing; poor
navigation; failure recovery; memory usefulness (`pira_ctx`, `pira_dec`);
permission friction; malformed tool calls; inability to resume; other
recurring problems. Each observation is classified with its evidence
(call ids, log lines). A mechanism is justified only by a concrete problem
seen here or in a later realistic workload. If L1 passes cleanly, the next
step is a harder workload, not a feature. The report is
`docs/evidence/L1_<date>.md`; work stops there for review.
