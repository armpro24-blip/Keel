# Shell-tool argument contract: transport check, then a description A/B (T25)

Status: **stage 1 complete 2026-09-10 on both machines, transport not at
fault (`docs/evidence/SHELL_TRANSPORT_2026-09-10.md`); stage 2 (A/B) is an
amended draft for review after the lab's comments, not approved to run.** Decision (user, 2026-09-10) after
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

## Stage 2 (amended draft for review): A/B on the `shell` description

Not approved to run. Every number below is a proposal for the reviewer.
Amendments after the lab's stage-1 comments are marked *(amended)*.

**Arms.** A: the current description (`src/shell.rs` at `822fdbc`). B: the
clarified description below. B lives on an experiment branch that changes
only that string; two binaries are built from the two commits and nothing
else differs: model `nvidia/Qwen3.6-35B-A3B-NVFP4` on the audited serving
configuration, no sampling parameters, PIRA `4e0682dd`, `--full --trace
--record-wire --max-turns 12`, fresh repository per run.

**B text (proposed; final wording is part of the review):**

> Run a program with arguments. argv is executed directly, with no shell:
> argv[0] is the program and every other element is one argument, delivered
> to the program exactly as written. Do not add quotes that only a shell
> would remove (write `["python", "-c", "print('hi')"]`, not
> `["python", "-c", "\"print('hi')\""]`); quotes, spaces, backslashes, and
> newlines that belong to the argument's content stay in it; this holds
> just the same when the program text itself contains quotes. Redirection,
> pipes, and && are shell features: as standalone argv elements they are
> rejected; they work only inside the command element of an explicitly
> invoked shell. If you need a shell, {shell_hint}; the element after its
> command flag is then parsed by that shell under its own quoting rules,
> not by Keel. workdir sets the process's working directory only; it does
> not change how the program itself resolves modules or relative paths.
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

**Design.** 4 tasks × 2 arms × 10 repetitions = 80 runs, interleaved
A,B,A,B per task so drift affects both arms alike. Budget `--max-turns 12`
per run; a run that exhausts it is a failed run for correctness. Zero
Continue, no retry, no hint; a question is answered only from the task text
and recorded. Scoring is mechanical from the SessionLog and the repository:

| Outcome | Definition |
|---|---|
| correctness (primary) | R1/R2/D2: the final assistant text, trimmed, equals the correct result; D1: the file exists with exactly the expected bytes |
| argument-usage error runs (primary) | a run with at least one of: `argv` collapse denial (`input needs an array field 'argv'`); standalone shell-operator rejection; *(amended)* a shell-reparse failure, defined mechanically as: argv invokes a shell (`cmd`, `powershell`, `pwsh`, `sh`, `bash`) and the child's stderr shows a Python `SyntaxError`, `can't open file`, or a command-not-found from a fragment of the element (the shell split or re-quoted the element); a result whose stderr shows the program received a different argument than intended (`No such file` on a path that exists) |
| helper-file repair calls (primary) | tool calls that create, edit, read back, or run a file the task did not ask for (scripts under `tests/`, `_check*.py`, redirect targets); counted per run and *(amended)* reported per task as well as in total, since R1/R2/D2 are expected near zero in both arms and D1 decides this outcome |
| invalid-Python runs (informational) | *(amended)* a run with a Python `SyntaxError` on a `python -c` program where argv invoked no shell (the echoed line is the sent element; compound statement after `;` etc.); not counted against either arm. Stage 1 confirmed this class is disjoint from shell re-parsing by the presence or absence of a shell in argv, not by the error text |
| model calls, tool calls, denials, handshake announcements, ordering violations | as in the dogfood reports; kept in full |

**Decision gate (proposed).** B is a stable improvement only if, over the
40 runs per arm, (1) B's correct runs ≥ A's, and (2) B's argument-usage
error runs ≤ ⌊A/2⌋ *(amended: A = 4 or 5 → B ≤ 2)*, and (3) B's helper-file
repair calls ≤ A's in total and in no task class more than A's, with no
task class where B is worse on (1). If A shows fewer than 4
argument-usage error runs in total, the experiment cannot discriminate and
is reported as such (not as "B has no effect"). Total call count is
reported but never decides.

*(amended)* Expected duration from L2-R2's mean of 5.75 s per model call:
at most about 1.5 hours if every run used its 12 calls, likely 30–45
minutes.

**If B passes**, the next discussion is merging the description and a full
task regression; **if B does not pass**, the candidate stops. Either way
the L2-R2 result, the budget of 100, and the frozen acceptance stay as
they are.

## Reporting

Stage 1: `docs/evidence/SHELL_TRANSPORT_<date>.md` with the test output
verbatim and the four-call classification. Stage 2, if approved:
`docs/evidence/SHELL_CONTRACT_AB_<date>.md` with the per-run table (task,
arm, correct, argument-usage errors, helper calls, invalid-Python, calls,
denials), the gate evaluation, and every intervention. Then stop.
