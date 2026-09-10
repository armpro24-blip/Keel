# Shell-tool argument contract: transport check, then a description A/B (T25)

Status: **complete. Stage 1: transport not at fault
(`docs/evidence/SHELL_TRANSPORT_2026-09-10.md`). Stage 2 run once,
80/80, 2026-09-10: B did not reach the candidate-advancement threshold
(condition 1 fails on R1, condition 2 fails outright with B 9 vs A 5
argument-error runs, condition 3 fails on D1); the candidate stops, the
description is not merged, `exp/shell-contract-b` stays as the record
(`docs/evidence/SHELL_CONTRACT_AB_2026-09-10.md`). All 18 confirmed
argument errors were `argv` sent as a string; the shell re-parsing class B
addressed produced zero events.** Stage 2 had been approved by the user on
2026-09-10 for exactly one run after the three revisions below were frozen
(B text, error classification, budget and correctness rules). Expectations for the candidate are lowered: much of what
looked like quoting trouble was invalid one-line Python written by the
model.** Decision (user, 2026-09-10) after
the T24 audit: the candidate is to clarify the `shell` tool's
argument-passing contract in its description. No new tool, no change to
execution semantics, no full L2 rerun. The candidate is not expected to fix
everything T24 found: it cannot supply a missing edit anchor and it does not
make the model edit the right test. It tests one thing: whether the tool's
usage is stated clearly enough.

## What T24 showed and what this candidate addresses

T24 separated two problems: the model chose a long verification route
although the evidence was visible (a PIRA-method and model-behavior matter,
not addressed here), and on that route it kept hitting tool-use mechanics:
quoting, helper files, import paths, CRLF. The second is the harness's own
responsibility to explain, so it gets the first minimal improvement.

Reading the L2-R2 timeline more closely, the "quoting failures" split into
two kinds that the description must treat differently:

- **Shell re-parsing.** Calls 39, 54, 92 passed `cmd /C` with the whole
  Python program as one argv element containing double quotes; `cmd` then
  parsed that element under its own rules and Python saw `"import` as an
  unterminated string. This is a contract fact Keel can state: argv goes to
  the program with no shell in between; if you invoke a shell, that shell's
  rules apply to its element.
- **Invalid one-line Python.** Calls 68, 74, 77, 80 (all four, confirmed
  in stage 1 from the echoed tracebacks) put a compound statement (`try:`,
  `def`) after `;` on one line; Python rejected the program itself. Not a
  transport or quoting matter. The description can say that an argument may
  contain newlines; it must not teach Python.

Stage 1 decides whether either class has a transport component.

## Stage 1: does a legal argument arrive unchanged? (deterministic, no model)

`tests/shell_transport.rs` sends fixed argv payloads through the exact path
Keel uses for ordinary commands (`wrap_command` → `pira_ctx … -- python …`
→ `run_process`) and checks what the child received. Ignored by default;
run on the lab machine (PIRA 1.9.0, Python 3.12.10) from the Keel checkout:

```bash
cargo test --test shell_transport -- --ignored --nocapture 2>&1 | tee ~/Desktop/shell_transport.txt
```

| Test | Payload | Pass criterion |
|---|---|---|
| `payload_arrives_unchanged_in_exact_mode` | 7 arguments: `a b`, `c"d"e`, `e\f`, `it's`, `--flag=x y`, empty, `trailing\` | JSON echo of `sys.argv[1:]` equals the payload byte for byte |
| `payload_arrives_unchanged_in_default_mode` | same, through the `pira_ctx` synopsis | the JSON text is present verbatim in the observation |
| `program_text_with_quotes_and_backslashes_arrives_unchanged` | a `-c` program with both quote kinds, a raw backslash, and a newline (the shape of calls 70/72) | prints `a b c"d e\f` then `second line` |
| `compound_statement_after_semicolon_is_pythons_error_not_transports` | the shape of call 80: `def` after `;` | Python's own `SyntaxError` at `def job`; shows the argument arrived intact |
| `cmd_slash_c_reparses_the_element_under_cmd_rules` (Windows) | the shape of calls 39/54/92 | not a pass/fail test; the observation is recorded so the description can state what an explicit shell does |

**Stop rule.** If any of the first three fails, the transport changes legal
arguments; the description experiment does not start and the defect is
fixed first. A more detailed description must not paper over an
implementation fault.

Also in stage 1, read-only, from the L2-R2 log: print calls 68, 74, 77, 80
with the full one-liner visible and classify each `SyntaxError` as
shell-reparse or invalid-Python:

```bash
PYTHONUTF8=1 python <Keel>/docs/dogfood/L2/tools/debug_trace.py "<L2-R2 log>" --from 68 --to 80 --width 600 --lines 8
```

Result: passed on the author's machine (Windows, PIRA 1.8.0, Python 3.14)
and on the lab machine (PIRA 1.9.0, Python 3.12.10) with line-for-line the
same observations; `cmd /C` reproduces the `"import` unterminated-string
failure on both; the four L2-R2 calls are all invalid one-line Python
(`docs/evidence/SHELL_TRANSPORT_2026-09-10.md`). Stop rule not triggered.

## Stage 2 (frozen 2026-09-10, one run approved): A/B on the `shell` description

Amendments after the lab's stage-1 comments are marked *(amended)*; the
user's three revisions before approval are marked *(revised)*. B is
`exp/shell-contract-b`, one commit on top of `main` changing only the
description string in `src/shell.rs`; `cargo test` passes there.

**Arms.** A: the current description (`src/shell.rs` at `822fdbc`). B: the
clarified description below. B lives on an experiment branch that changes
only that string; two binaries are built from the two commits and nothing
else differs: model `nvidia/Qwen3.6-35B-A3B-NVFP4` on the audited serving
configuration, no sampling parameters, PIRA `4e0682dd`, `--full --trace
--record-wire --max-turns 12`, fresh repository per run.

**B text (frozen; identical to the string on `exp/shell-contract-b`):**

> Run a program with arguments. argv is executed directly, with no shell:
> argv[0] is the program and every other element is one argument, delivered
> to the program exactly as written. Do not add quotes that only a shell
> would remove (write `["python", "-c", "print('hi')"]`, not
> `["python", "-c", "\"print('hi')\""]`); quotes, spaces, backslashes, and
> newlines that belong to the argument's content stay in it. Redirection,
> pipes, and && are shell features: as standalone argv elements they are
> rejected; they work only inside the command element of an explicitly
> invoked shell. If you need a shell, {shell_hint}; the element after its
> command flag is then parsed by that shell under its own quoting rules,
> not by Keel. workdir sets the child process's current working directory.
> Relative paths and module imports are then resolved according to that
> program's own rules. *(revised: the earlier "does not change … relative
> paths" could be read as "the working directory does not matter")*
> Provide the actual command in argv and its purpose in the top-level intent
> field. For ordinary commands, Keel automatically runs argv through
> pira_ctx; do not wrap ordinary commands in pira_ctx yourself. Invoke a
> PIRA internal tool directly only when that tool itself is the intended
> command. Returns stdout, stderr, and the exit code. In
> full-permission/no-approval mode a state_changing command needs a
> safety_review before it runs.

Excluded from B by design: any mention of `queuewatch`, tests, table
widths, expected strings, "edit the assertion", Python one-liner syntax, or
CRLF. It is an interface statement, not a solving strategy.

**Tasks (frozen text; each run gets a fresh `init_l2.sh` repository plus,
for D2, the data file below copied to `data/quotes.jsonl`).**

| Id | Class | Task text sent as one line | Correct result |
|---|---|---|---|
| R1 | read code, compute a deterministic result | "Count the test methods (functions whose name starts with `test_`) defined under `tests/` in this repository. Reply with the number only." | `42` |
| R2 | read code, compute a deterministic result | "Compute the SHA-256 of the file `queuewatch/report.py` exactly as stored on disk. Reply with the lowercase hex digest only." | `25ba9483d9cf0c91b81cd8d6576d2df3f04eff7340157269172891b2d51c1a44` |
| D1 | data with quotes, spaces, backslashes | *(amended)* "Create the file `notes/summary.txt` whose entire content is the following line followed by one LF newline, byte for byte, UTF-8 without BOM: `She said "it's done" -- path C:\tmp\x y`" | file bytes `She said "it's done" -- path C:\tmp\x y\n` (UTF-8, LF, no BOM); a file with the right text but CRLF or a BOM is reported in its own column, neither correct nor an argument-usage error |
| D2 | data with quotes, spaces, backslashes | "In `data/quotes.jsonl`, find the record whose `id` is 3 and reply with the exact value of its `name` field and nothing else." | `She said "it's done" -- path C:\tmp\x y` |

`data/quotes.jsonl` is `docs/dogfood/shell_contract/data/quotes.jsonl` (5
records; ids 3 and 4 carry quotes, apostrophes, backslashes, spaces, a
trailing backslash; id 5 is empty).

*(amended)* R2's answer is the digest of the file as `init_l2.sh` writes it
to disk. It equals the git blob only because the seed carries
`.gitattributes` with `* text eol=lf` and `init_l2.sh` initializes with
`core.autocrlf=false` (lab check: CR count 0 on disk; the CRLF variant would
hash to `53424d76…e03e`). Scoring uses the disk file of the run's own
repository, never a blob hash from another machine; if either setting
changes, the frozen answer is void.

**Design.** 4 tasks × 2 arms × 10 repetitions = 80 runs, at most 960
model calls in total (the "about 1.5 hours" is an estimate, not a limit).
*(revised)* Within each task the 10 pairs alternate order, AB, BA, AB, BA,
…, so neither arm always runs first. Budget `--max-turns 12` per run; a run
that exhausts it is incomplete. Zero Continue, no retry, no hint.
*(revised)* **A question from the model is not answered**: it ends the run
as incomplete and the question text is kept, because an answer would start
a new count and the runs would no longer share one budget. Scoring is
mechanical from the SessionLog and the repository
(`tools/score_shell_ab.py`, one CSV row per run; self-check
`tools/test_score_shell_ab.py`):

| Outcome | Definition |
|---|---|
| correctness (primary) | *(revised)* the run **completed normally** (final text, no error, budget not exhausted) **and** the result check passes: R1/R2/D2 the final text, trimmed, equals the correct result; D1 the file has exactly the expected bytes. D1 with a correct file but an exhausted budget is reported as "file correct, run incomplete" and is not a correct run |
| confirmed argument-usage errors (primary) | *(revised)* structural denials (`input needs an array field 'argv'`; a standalone shell operator rejected) and **output evidence** that an explicitly invoked shell changed or split an element: Python's traceback echoes a line that is not a line of any sent argv element (for example `"import`), or `can't open file` after a shell. "Shell invoked + SyntaxError" alone proves nothing: invalid Python stays invalid Python through a shell |
| program's own errors (reported, not an argument error) | *(revised)* a Python `SyntaxError` with no shell in argv (stage 1 verified the transport, so the program is what the model wrote: a compound statement after `;`, an inner `exec` string, …), or, through a shell, one whose echoed line is a line of the program the model meant to run. Not counted against either arm as an argument error, but it costs calls and can cost correctness, and both are reported per arm |
| undetermined (reported) | *(revised)* a `SyntaxError` through a shell with no echoed line, or an echoed line matching neither rule. Never folded into the confirmed class. A `pira_ctx` synopsis that dropped the `SyntaxError:` line is not classified at all (known undercount; the error flag still counts the failed call) |
| helper-file related calls (primary) | *(revised: renamed; creating a helper is not an error)* tool calls naming a file the task did not ask for and the seed does not track (scripts under `tests/`, `_check*.py`, redirect targets); counted per run, the paths listed for hand review, and *(amended)* reported per task as well as in total |
| model calls, tool calls, denials, handshake announcements, ordering violations | as in the dogfood reports; kept in full |

**Candidate-advancement threshold (frozen).** *(revised: this is the
preregistered threshold for taking the candidate to the next discussion,
not a claim of general stable improvement; the sample supports the former,
not the latter.)* B reaches it only if, over the 40 runs per arm,
(1) B's correct runs ≥ A's, and in no task fewer than A's; (2) A shows at
least 4 runs with a confirmed argument-usage error and B shows ≤ ⌊A/2⌋
such runs *(A = 4 or 5 → B ≤ 2)*; (3) B's helper-file related calls do not
exceed A's, in total or in any task. If A shows fewer than 4 confirmed
argument-usage error runs the experiment cannot discriminate and is
reported as such; **no samples are added to reach the threshold**. Total
call count is reported but never decides.

*(amended)* Expected duration from L2-R2's mean of 5.75 s per model call:
at most about 1.5 hours if every run used its 12 calls, likely 30–45
minutes.

**If B reaches the threshold**, the next discussion is merging the
description and a full task regression; **if not**, the candidate stops.
Either way the L2-R2 result, the budget of 100, and the frozen acceptance
stay as they are. Nothing in the runtime and nothing in the tasks is
changed to raise the error rate; after the 80 runs the experiment stops.

## Reporting

Stage 1: `docs/evidence/SHELL_TRANSPORT_<date>.md` with the test output
verbatim and the four-call classification. Stage 2, if approved:
`docs/evidence/SHELL_CONTRACT_AB_<date>.md` with the per-run table (task,
arm, correct, argument-usage errors, helper calls, invalid-Python, calls,
denials), the gate evaluation, and every intervention. Then stop.
