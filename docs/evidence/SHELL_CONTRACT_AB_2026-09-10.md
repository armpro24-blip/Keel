# Shell-description A/B (T25 stage 2): threshold not reached, candidate stops (2026-09-10)

Protocol: `docs/dogfood/shell_contract/SHELL_CONTRACT_AB.md`, stage 2 as
frozen at `fe9229e`. One run of 80 sessions on the lab machine; the lab's
delivery (report, `results.csv`, `notes.txt`, `handshake.txt`,
`run_order.txt`, `t25_serving.txt`) is stored verbatim in
`docs/dogfood/shell_contract/stage2_2026-09-10/`. The lab reported counts
only; the threshold is applied here by the Keel author, recomputed from
`results.csv`.

## Verdict

**B does not reach the preregistered candidate-advancement threshold. The
candidate stops.** Of the three conditions, none holds in full:

| Condition | A | B | Holds? |
|---|---|---|---|
| (1) B correct ≥ A, and in no task fewer | 34/40 | 39/40 | total yes; **R1 no** (A 10, B 9) |
| (2) A ≥ 4 runs with a confirmed argument-usage error; B ≤ ⌊A/2⌋ | 5 runs (6 events) | 9 runs (12 events) | discriminable (A = 5, so B ≤ 2); **no** |
| (3) B helper-file related calls ≤ A, total and per task | 1 (corrected; 2 as delivered) | 1 (corrected; 8 as delivered) | total yes; **D1 no** (A 0, B 1); as delivered no |

Under the frozen rules this is the end of the candidate: the description
is not merged, `exp/shell-contract-b` stays as the experiment record, and
the runtime remains `822fdbc`. The L2-R2 result, the budget of 100, and the
frozen acceptance are unchanged.

## What the 80 runs actually showed

**The error class B was written for did not occur.** All 18 confirmed
argument-usage error events in both arms are the structural denial
`input needs an array field 'argv'` (argv sent as one string instead of an
array). Shell re-parsing, the class B's new sentences address, produced
zero events in 80 runs; `undetermined` is zero. The B text says what an
element is and what a shell does with it; it says nothing that A does not
about the shape of `argv`, and the model collapsed argv more often under B
(12 events in 9 runs) than under A (6 in 5). Of the 14 runs with a
collapse, 13 went on to complete (D2-A-1 completed with a wrongly formatted
answer; the other 12 completed correctly) and D1-A-9 exhausted its budget;
whether each collapse was corrected on the very next call was not checked
against the SessionLogs, which stay on the lab machine.

**Correctness favoured B, for reasons the sample cannot attribute.**
A's six non-correct runs: three are format failures around a right value
(R2-A-2 digest inside prose and a code fence; D2-A-1 prose prefix; D2-A-7
backticks), two are D1 byte-exactness failures (D1-A-3 no trailing LF;
D1-A-5 44 bytes, CRLF/BOM class), one is D1-A-9 exhausting the 12-call
budget with a wrong file. B's one non-correct run is R1-B-1 answering `8`
instead of `42`. The strict, preregistered correctness rule counts these as
they fall. With 40 runs per arm and one experiment, the 39-vs-34 difference
is reported, not interpreted as an effect of the description.

**B used more calls**: 202 against 180, mostly D1 (77 vs 64, with 37
state-changing calls with review against 28: more hex-dump verification)
and R1 (67 vs 57). Total call count never decides, by protocol.

**Program's own errors** (invalid one-line Python, not counted against
either arm): A 2 events (both R1-A-7), B 3 (all D1: B-3 one, B-6 two).
Zero in the D2 quoting task for both arms.

**Handshake and safety**: 80 of 80 `full_mode_check` summaries show
`approve? prompts 0`, `ordering violations 0`; state-changing calls with a
review 28 (A) and 37 (B), all announced before execution; one review-less
edit denied and recovered (D1-B-4). No question from the model in any run,
so the no-answer rule was never exercised; interventions were the task line
and `/quit` only, 80 times.

## Per-task table (recomputed from `results.csv`)

| Task | A correct | B correct | A arg-error runs (events) | B arg-error runs (events) | A helper (corrected) | B helper (corrected) | A calls | B calls | A prog | B prog |
|---|---|---|---|---|---|---|---|---|---|---|
| R1 | 10/10 | 9/10 | 1 (2) | 2 (2) | 1 | 0 | 57 | 67 | 2 | 0 |
| R2 | 9/10 | 10/10 | 0 | 0 | 0 | 0 | 22 | 23 | 0 | 0 |
| D1 | 7/10 | 10/10 | 1 (1) | 3 (4) | 0 | 1 | 64 | 77 | 0 | 3 |
| D2 | 8/10 | 10/10 | 3 (3) | 4 (6) | 0 | 0 | 37 | 35 | 0 | 0 |
| total | 34/40 | 39/40 | 5 (6) | 9 (12) | 1 | 1 | 180 | 202 | 2 | 3 |

D1 extra columns: A `file_correct` 7, `line_ending` 1 (D1-A-5); B
`file_correct` 10, `line_ending` 0. End reasons: 79 `completion`, 1
`budget` (D1-A-9). Model calls 382 in total, equal to the serving host's
`time_to_first_token_seconds_count` delta over the run.

Non-correct runs and their final texts, as delivered:

```text
R1-B-1  completion | 8
R2-A-2  completion | The SHA-256 digest is:\n\n```\n25ba9483…1a44\n```
D1-A-3  completion | file 39 bytes (no trailing LF)
D1-A-5  completion | line_ending=True (44 bytes)
D1-A-9  budget     | (no final text)
D2-A-1  completion | The name field for id 3 is: `She said "it's done" -- path C:\tmp\x y`
D2-A-7  completion | `She said "it's done" -- path C:\tmp\x y`
```

Confirmed argument-usage error events (all `input needs an array field
'argv'`): A: R1-A-9 ×2, D1-A-9, D2-A-1, D2-A-5, D2-A-9. B: R1-B-5, R1-B-8,
D1-B-2, D1-B-5, D1-B-8 ×2, D2-B-3 ×2, D2-B-4, D2-B-6 ×2, D2-B-7.

## Run conditions (lab, verified against the delivery)

A = `main` at `fe9229e` (runtime `822fdbc`), B = `exp/shell-contract-b` at
`4f71fe7`; `git diff main exp/shell-contract-b --stat -- src Cargo.toml
Cargo.lock` is `src/shell.rs` only (+16/−7); the binaries differ in that
string alone (`grep -c "argv\[0\] is the program"`: A 0, B 1). Model
`nvidia/Qwen3.6-35B-A3B-NVFP4` snapshot `1355db6a`, vLLM 0.26.0 image
`v0.26.0` build `ffd46bf`, parsers `qwen3`/`qwen3_xml`, template
`e84f32a2…`, sampling defaults from `generation_config.json`; PIRA
`4e0682dd`; `--full --trace --record-wire --max-turns 12`; fresh
`init_l2.sh` repository per run (80 × `5d668de`, 42 tests OK); D2 with
`data/quotes.jsonl` copied in. Order: R1, R2, D1, D2; within each task ten
pairs alternating A→B, B→A. 20:21:29Z to 20:49:33Z, 28 min 04 s. Task
lines taken from the protocol table (the instruction's paste had joined
words; the lab noticed and used the table).

Deviations recorded by the lab: `git diff main --stat` on the branch listed
four files because `main` had gained `fe9229e` (scorer fix, docs/tools
only) after the branch point `cbe1923`; the code difference is
`src/shell.rs` only. Scoring used `fe9229e`'s script.

## Scorer defect found by the lab, corrected in the tool, corrected by hand here

`score_shell_ab.py` listed tracked seed files as helper paths in three runs
(R1-B-7: five `tests/test_*.py`; R2-A-3 and R2-B-6: `queuewatch/report.py`).
Cause: an absolute Windows path loses its drive letter in the file-token
regex and then starts with `/Users/…`; the tracked-file comparison tested
the wrong direction, and a doubled slash (`queuewatch//report.py`) was not
normalized. The delivered CSV is kept as delivered (A 2, B 8); the
corrected counts used for condition (3) are the lab's own reading (real
helpers: R1-A-1 `f.py`; D1-B-8 `notes/create.py`), which the recount
confirms. The script now compares suffixes and collapses repeated slashes;
the self-check covers both shapes. The correction does not change the
verdict: (3) fails on D1 either way, and (1) and (2) fail independently.

## Reading, within the fixed rules

The candidate set out to test whether the tool's usage was stated clearly
enough, expecting shell re-parsing as the failure to reduce. In 80 small
runs that failure did not appear at all, so the experiment could not
observe the effect B was written for; the argument error that did appear
(argv as a string) is a schema-shape error neither text addresses, and B
did not reduce it. B's higher correctness is real in this sample and
unexplained; nothing in the design lets it be credited to the two changed
sentences. Per the protocol the candidate stops here; whether the
argv-collapse class deserves its own, different candidate is a new
decision, not a continuation of this one.

No Keel change follows. `exp/shell-contract-b` is not merged.
