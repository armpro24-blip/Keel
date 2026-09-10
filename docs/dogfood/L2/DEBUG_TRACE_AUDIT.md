# Debug-trace audit: why the repair loop took so many calls (after L2-R2)

Status: **complete (2026-09-10, `docs/evidence/DEBUG_TRACE_2026-09-10.md`).**
Loop 66–100 of L2-R2. Q1 yes (direct output, full assertion diff visible at
call 66); Q2 premise not met, and 0 of 46 result IDs were ever retrieved;
Q3 R3 F8 I1 H22 P0 O1. Rules 2 (model debugging behavior) and 3 (tool-use
path) triggered, rule 1 not; no mechanism proposed. Lab delivery stored
verbatim in `debug_trace_2026-09-10/`. Original status line follows.

Protocol frozen 2026-09-10; static only, no model run, no Keel or
PIRA change. Decision (user, 2026-09-10): the L2-R2 classification is
accepted; no further sampling for now, the budget stays at 100, no
empty-response retry is implemented. The next step is to read the logs that
already exist and reconstruct one repair loop from its first relevant test
failure to the end, answering three questions about how the model obtained,
understood, and used failure evidence.

## Why this loop

L2, L2-R1, and L2-R2 all spent a large share of their calls repairing the
model's own table-formatting test for `--status`. In L2-R2 the run ended
with that test still disagreeing with the model's own implementation, while
the hidden acceptance (A08–A12) accepted the implementation: the format
requirement the model was chasing was its own, not the task's. The loop is
chosen because it recurred, not to design a "table whitespace" patch.
Nothing here says "the model is not capable enough" or "the PIRA synopsis
caused the failure"; both remain unsupported.

## Material (already on the lab machine; nothing is rerun)

- L2-R2 SessionLog `~/.keel/sessions/19b4f12b539d766d/2962a799a0520c68.jsonl`,
  its `keel log show` rendering (`~/Desktop/l2r2_logshow.txt`), the wire
  (local only), `~/Desktop/l2r2_session.txt`, the repository
  `~/Desktop/queuewatch-l2r2` as left, and the `pira_ctx` store of that
  workspace (retained captures with their result IDs).
- For the presence check only: the L2 and L2-R1 SessionLogs.
- Tool: `tools/debug_trace.py` (read-only; one block per model call with
  tool, verdict, error flag, mechanical flags, first observation lines;
  summary of repeats, retrievals, helper-related calls, and result IDs given
  vs retrieved). Self-check: `python tools/test_debug_trace.py`.

## Step 0: complete the L2-R2 evidence

Deliver the three items cut from the L2-R2 report by the transport limit:
the rest of `~/Desktop/l2r2_diff.patch` (from the `queuewatch/events.py`
hunk on), the `pira_ctx history --scope workspace --limit 100` tail, and
`wc -l ~/Desktop/l2r2_logshow.txt`.

## Step 1: the timeline

```bash
python <Keel>/docs/dogfood/L2/tools/debug_trace.py "<L2-R2 log>" --helpers tests/_check.py,tests/_check2.py,tests/_write_cli.py,tests_output.txt --lines 6 > ~/Desktop/l2r2_trace.txt
```

Deliver the file in full (100 calls; observation text is the first 6 lines
per result, which is what the model saw first). The `--helpers` list is the
untracked leftovers from `git status`; it marks calls that touch them.

## Step 2: bound the loop

From the timeline and `log show`, record:

- **Start**: the first tool result in which a test the model wrote for the
  `--status` output fails (call number, test name, the observation's first
  lines).
- **End**: call 100.
- The state of that test at the end (the L2-R2 report: expected
  `'a    failed    1        0:01:04'`, actual `'a    failed  1        0:01:04'`).

## Step 3: answer three questions, each with call numbers as evidence

**Q1. Was the information needed for a correct diagnosis present in the
tool output the model could see?** For every failing-test observation in
the loop: did the returned observation contain the assertion's expected
and actual lines (or the traceback line that names the mismatch)? Record
per occurrence: call, `yes` / `partial` / `no`, and whether the observation
was a `pira_ctx` synopsis (`Captured:` / `Result:` header) or direct
output. If the decisive lines were only in the retained capture and not in
the synopsis, say so with the result ID.

**Q2. If not visible, was it retained in `pira_ctx` with a retrievable
result ID, and did the model try to retrieve it?** From the timeline's
summary: result IDs given, IDs retrieved later, and the calls that
retrieved (`pira_ctx search/range/transform/exec/raw`). Check one or two
retained captures directly (`pira_ctx search <ID> 'AssertionError|Lists differ' --context 3`)
to confirm the lines were retrievable. Read-only.

**Q3. What was each call in the loop for?** Classify every model call in
the loop into exactly one of the pre-registered categories:

| Code | Category |
|---|---|
| R | real fix attempt on business code or on the test's expectation (an `edit_file` or write that changes `queuewatch/*` or the relevant test) |
| F | legitimate failure reproduction or test feedback (running the suite or the failing test once after a change) |
| I | re-acquiring information the model already had (re-running the same tests or re-reading the same file without an intervening change; the `repeat xK` flag is the mechanical hint) |
| H | creating, fixing, or working around helper scripts and shell/file mechanics (`_check.py`, `_write_cli.py`, quoting, CRLF, `argv` shape) rather than the business problem |
| P | reading PIRA policy (`read_pira_policy`) |
| O | other (state what) |

Report the count per category, the sequence as a string of codes in call
order, and for each `H` and `I` call one line on what it was. Errors are
not counted as waste by category: an `F` call that fails is doing its job;
34 tool errors in the run include normal reproduction.

## Step 4: presence check in L2 and L2-R1 (script only)

```bash
python <Keel>/docs/dogfood/L2/tools/debug_trace.py "<L2 log>"    | tail -20
python <Keel>/docs/dogfood/L2/tools/debug_trace.py "<L2-R1 log>" | tail -20
```

Deliver the summaries, and for each log the call range in which the same
`--status` table-format test was being repaired (start/end call numbers
only, from a quick read of the timeline). No hand classification for these
two.

## What the answers decide (fixed in advance)

- Q1 mostly `no`, or `partial` because the synopsis dropped the decisive
  lines → the next investigation is the output and retrieval interface
  (what the observation carries, how the model is told to retrieve).
- Q1 `yes` and the model still cycled → the finding is about the model's
  debugging behavior under this configuration; no interface change is
  indicated by this audit.
- Q3 dominated by `H` → the next investigation is the tool-use path
  (shell/file mechanics on Windows), not the business repair.

Only after one of these is established is a single minimal, general
candidate chosen for a controlled comparison. This audit itself proposes no
mechanism. The L2-R2 result (11/13, budget exhausted), the budget of 100,
and the frozen acceptance stay as they are.

## Reporting

`docs/evidence/DEBUG_TRACE_<date>.md`: step 0 items appended to the L2-R2
report; the timeline file; the loop bounds; the three answers with call
numbers; the two presence summaries. Raw observation text is quoted only as
far as the answers need. Then stop.
