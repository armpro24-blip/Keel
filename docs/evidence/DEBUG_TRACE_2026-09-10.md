# Debug-trace audit evidence: why L2-R2's repair loop took 35 calls (2026-09-10)

Protocol: `docs/dogfood/L2/DEBUG_TRACE_AUDIT.md` (T24), frozen at `3ad8a3a`.
Static and read-only: no model run, no Keel or PIRA change, nothing rerun.
The lab operator delivered the report, the step-0 supplement, and the full
timeline as a page; the three files are stored verbatim in
`docs/dogfood/L2/debug_trace_2026-09-10/` (the trace is byte-identical to the
lab's file apart from CRLF line endings: 54,234 bytes here, 55,133 there).
Sections marked "lab" are the lab's findings; the rest is the Keel author's
reading of the delivered timeline.

## Answers to the three preregistered questions (lab, verified against the timeline)

Loop bounds: **call 66 → call 100** (budget exhausted). Start: the first
full `unittest` run after the model's `--status` tests were added
(`[exact] python -m unittest discover -s tests -v`, 5 failures). End state:
the expectation `'a    failed    1        0:01:04'` in `tests/test_cli.py`,
written at call 63, was never edited again; the actual output is
`'a    failed  1        0:01:04'`; **after call 66 the model never ran
`unittest` again**, neither the suite nor a single test.

**Q1 — was the diagnostic information in the tool output the model could
see? Yes.** Call 66 ran in `exact` mode; the whole 162-line result went into
the `tool_result` as direct output (no `pira_ctx` synopsis, no result ID).
For the target test it carried `AssertionError: Lists differ`, `First
differing element 1`, the expected and actual rows, the `?  ++` marker under
the two extra spaces, and the summary-line mismatch. Calls 70 and 72 later
printed the actual `render_report` strings for the test scenarios, also as
direct output. There is no case in the loop of "the decisive lines were only
in the retained capture".

**Q2 — if not visible, was it retained and retrievable, and did the model
retrieve? The premise does not hold; the facts anyway:** 46 result IDs were
given to the model over the run, 12 of them inside the loop; **0 were ever
retrieved** (no `pira_ctx search/range/transform/exec/raw` call in the whole
session; likewise 0 of 50 in L2 and 0 of 20 in L2-R1). None of the 12 loop
captures was a test run, so no assertion line sits in a capture; retrieval
itself works (the lab's control searches on two captures returned their
stderr lines).

**Q3 — what was each of the 35 calls for?** Lab classification with the
protocol's six categories, one operationalization stated by the lab: the
first attempt to obtain a piece of information is `F` even if it fails on
quoting; later retries that only change quoting, one-liner form, or helper
mechanics are `H`.

| Code | Count |
|---|---|
| R real fix | 3 (67 cli.py summary count; 69 `choices=`; 71 test_report expectations) |
| F reproduction / feedback | 8 |
| I re-acquiring known information | 1 (90: rerun of 83 with the file unchanged) |
| H helper scripts and shell/file mechanics | 22 |
| P policy reads | 0 (all six `read_pira_policy` calls were before the loop) |
| O other | 1 (73: re-reading the two files it had just edited) |

Sequence 66→100: `FRFRFRFOHFFHHHHHHHHFFHHHIHHHHHHHHHH` (35 codes; counts
verified). **`tests/test_cli.py`'s expectation was not edited once in the
loop**; the three real fixes went to the summary count, the argparse
`choices`, and `test_report.py`.

## The 22 `H` calls, regrouped by mechanism (Keel author, from the timeline)

| Mechanism | Calls | Count |
|---|---|---|
| `python -c` one-liner rejected with `SyntaxError` (quoting: Keel executes argv with no shell; `cmd /C` adds a second quoting layer) | 74, 77, 80, 92 (+68 counted `F` as the first attempt) | 4 (+1) |
| `exec(open('tests/test_report.py')…)` tricks to borrow the test helpers | 78, 79 | 2 |
| helper file `_check.py` / `_check2.py`: create, run, edit, dump, clear, re-create | 81, 82, 83, 84, 87, 88, 89, 91, 93, 94, 95, 96, 97, 98, 99, 100 | 16 |

Inside the 16 helper-file calls: `ModuleNotFoundError` ×2 (82, 83; plus 90
as `I`) because `python tests\_check.py` puts `tests/` on `sys.path`, not
the workspace root, while every `python -m` invocation in the run imported
the package fine; `edit_file` zero-match ×2 with the CRLF hint shown (88,
93) because the helper had been written by a `python -c` text-mode write on
Windows; one review-less edit denied and recovered (84); one empty
`old_text` denied (95); the helper's own data bug (`finished=64` seconds)
×2 (79 as `H`, 98). The model's stated goal throughout (its own text at 70,
72, 78, 81, 87) was to "get the exact output strings for the test
assertions": it wanted to recompute what the failing assertion had already
printed at call 66.

## The other 65 calls (Keel author; not part of the preregistered loop)

| Calls | What happened | Count |
|---|---|---|
| 1–19 | orientation (2 listings, 12 file reads, 3 policy loads) and the implementation: 6 `edit_file` on `events.py`, `state.py`, `cli.py`, `report.py`, all successful after one review-less edit was denied at 12 | 19 |
| 20–30 | seeing the full test output: the suite run three times (20, 21, 22 with `capture`) each returning a `pira_ctx` synopsis (`unselected=4255 B/39 lines`); the model wrote "The output was truncated"; redirect to `tests_output.txt` and `type` it (23–24, 26, 28: synopsis again); a `find /c /v ""` pipe that resolved to GNU `find` and walked the file system (25: 231 MB, 2,042,765 lines, retained and summarized by `pira_ctx`); a PowerShell `-Tail` that printed the command instead of the file (29); then `exact` mode at 30 showed everything: "All 42 tests pass" | 11 |
| 31–33 | tests added to `test_state.py` and `test_report.py` by `edit_file`, first try | 3 |
| 34–65 | inserting tests into `test_cli.py`: `edit_file` zero-match ×4 (34, 41, 43, 46) on the anchor `def test_check_command_is_unaffected(self):`, a test that **did not exist in the seed** (the model added it itself at 63); byte dumps of the file (42, 44, 45, 47) reading the mismatch as an encoding problem ("My old_text has the same pattern but the edit keeps failing"); a direct `python -c` write that mangled the file (48–49), a rewrite that emptied it (50–51: `b''`), reconstruction one-liners failing on quoting (52–55), a generator helper `_write_cli.py` (56–57), one `argv` collapse denied (58), `git checkout` to restore (60), a PowerShell here-string rewrite that worked (63), `py_compile` ×2 (64–65) | 32 |
| 66–100 | the loop above | 35 |

Five result IDs from the 20–30 stretch pointed at complete test output the
model wanted (20, 21, 22, 24, 28); none was retrieved. The model reached
the same information by discovering `exact` mode at call 30 and used
`exact` for the decisive run at 66.

## Cross-run presence (lab, script only)

| Run | Loop on the same `--status` table-format test | Outcome | Retrievals / IDs given |
|---|---|---|---|
| L2 | 42–63 (22 calls): single test rerun at 46; `test_cli.py` expectations edited at 52, 56, 60; single test passes at 63, suite at 64 | fixed | 0 / 50 |
| L2-R1 | 25–40 (16 calls): 27–30 trying to print actual widths (quoting → `chk.py`); expectations edited at 31, 32; 33–39 quoting for `chk.py`; 40 empty response | cut off, unconfirmed | 0 / 20 |
| L2-R2 | 66–100 (35 calls): as above | budget exhausted, unfixed | 0 / 46 |

The same test recurred three times with three endings. In the one run that
fixed it (L2), the model edited the expectation and reran the test; in the
two that did not, it tried to recompute the expected strings with ad-hoc
Python instead.

## Interpretation under the rules fixed in the protocol

- **Rule 1 (information not delivered or hard to get → investigate the
  output and retrieval interface): not triggered for the loop.** Q1 is
  `yes` with direct output. Outside the loop, calls 20–30 show a cost of
  about ten calls before the model found `exact` mode for a 47-line test
  listing whose synopsis had dropped what it wanted; the retained captures
  were retrievable and never retrieved. That is recorded as an observation
  about how this model uses the interface, not as a finding that the
  interface withheld the diagnosis.
- **Rule 2 (information clearly delivered, model still cycled → the model's
  debugging behavior): triggered.** The complete diff of its own wrong
  expectation was on screen at 66; the model fixed the two implementation
  issues that diff also showed (67, 69) and the `test_report.py`
  expectations (71), but for `test_cli.py` it set out to recompute the
  strings, never edited the expectation, and never reran the tests.
- **Rule 3 (dragged by shell/helper-file problems → the tool-use path):
  triggered.** 22 of 35 loop calls, and 32 more in 34–65, went to mechanics
  rather than the business problem. The concrete classes, with counts over
  the whole run: `python -c` one-liners failing on quoting under
  argv-without-shell (68, 74, 77, 80, 92, and 39, 53–54: 8); helper files
  created by text-mode writes then unmatched by `edit_file` because of CRLF
  (88, 93: 2, both with the hint shown); scripts run as files so the package
  was not importable (82, 83, 90: 3); `edit_file` with a nonexistent anchor
  read as a byte problem (34, 41, 43, 46: 4, then 4 byte dumps); one
  runaway pipe (25); one `argv` collapse (58); two review-less edits (12,
  84) and one empty `old_text` (95), all denied and recovered.

Two of the three branches are established at once, and they are not in
tension: the model had the evidence and chose a verification route
(recompute, then compare) whose every step ran into the mechanics of
writing and running ad-hoc Python under this tool contract on Windows.
The audit proposes no mechanism; per the protocol, choosing one minimal,
general candidate for a controlled comparison is the next decision, and it
is the reviewer's.

## What this does not say

Not "the model is not capable enough": the same model fixed the same test
in L2 in 22 calls, and every business change it made in L2-R2 passed the
hidden acceptance except the README it never reached. Not "the `pira_ctx`
synopsis caused the failure": the decisive observation bypassed it. Not a
rate: one loop reconstructed, two more located by call range only.

## Tool notes

- `debug_trace.py` failed on the lab machine when redirected to a file
  (`UnicodeEncodeError`, cp1252, on `→` in an observation); the lab ran it
  under `PYTHONUTF8=1` without changing the script. The script now forces
  UTF-8 output; the self-check covers it.
- The delivered trace's `repeat xK` flag compared the whole tool input,
  intent prose included, so the identical reruns at 20/21 and 83/90 were
  not flagged (the lab classified 90 as `I` by hand). The flag now compares
  the command only (argv and mode; path and texts for `edit_file`). The
  delivered `l2r2_trace.txt` predates that change; its counts are kept as
  delivered.

## Step-0 items received

The rest of `l2r2_diff.patch` (from the `events.py` hunk), the `pira_ctx
history` tail (93 rows), and `wc -l l2r2_logshow.txt` = 2819 are in
`docs/dogfood/L2/debug_trace_2026-09-10/l2r2_step0_supplement.txt` and are
noted in `docs/evidence/L2R2_2026-09-10.md`. The diff confirms: `report.py`
filters inside `render_report(jobs, status_filter=None)` and sums over all
jobs; `state.py` adds the `retry` transition; `events.py` adds the kind;
the `test_cli.py` expectation row is the one the model wrote at 63.
