# Gate B raw results: long payload in a `stdin` field (lab, 2026-09-08)

Keel `d18e1d5` (no Keel code involved; diagnostic requests only). vLLM
0.26.0, `nvidia/Qwen3.6-35B-A3B-NVFP4`, `max_model_len` 262144. Script
`docs/dogfood/stdin/gate_b.py`, 10 runs, 13:27:56Z to 13:30:51Z. Request
28,318 bytes; payload 1,835 bytes, 54 lines. Raw responses stay on the lab
machine (`~/Desktop/gate_b/`). Reported by the user; verbatim below.

## Pre-registered scores (`scores.txt`)

```text
metric                         score
valid_shell_call               10/10
argv_array                     10/10
stdin_string                   10/10
stdin_complete                 1/10
fields_separate                10/10
```

Per run (from `scores.txt`):

| run | finish | prompt | completion | reasoning chars | content | stdin bytes | stdin_complete (line containment) |
|---|---|---|---|---|---|---|---|
| 01 | tool_calls | 6487 | 777 | 432 | `Safety: Writing a file to tally/cli.py …` | 2184 | false (5/46 lines) |
| 02 | tool_calls | 6487 | 4001 | 12005 | `**Safety:** Action: overwrite tally/cli.py …` | 1918 | false (1/46) |
| 03 | tool_calls | 6487 | 1707 | 4129 | `Safety: Write — overwrites tally/cli.py …` | 1939 | false (3/46) |
| 04 | tool_calls | 6487 | 1871 | 4666 | null | 1951 | **true** |
| 05 | tool_calls | 6487 | 783 | 534 | null | 2014 | false (3/46) |
| 06 | tool_calls | 6487 | 1417 | 2900 | `**Safety:** Write to tally/cli.py …` | 2006 | false (4/46) |
| 07 | tool_calls | 6487 | 3881 | 12145 | null | 1942 | false (1/46) |
| 08 | tool_calls | 6487 | 1904 | 4513 | `Safety: File write to tally/cli.py …` | 2393 | false (6/46) |
| 09 | tool_calls | 6487 | 993 | 1168 | `Let me first check if tally/cli.py already exists, then write the new content.` | 2181 | false (4/46) |
| 10 | tool_calls | 6487 | 851 | 819 | `Safety: Writing provided Python source …` | 1992 | false (3/46) |

Every response: exactly one tool call, `finish_reason = tool_calls`,
`argv = ["python", "-"]`, `effect = state_changing`, non-empty hidden
`reasoning` (Qwen reasoning parser). No response collapsed `argv` into a
string; none put the arguments object inside a string.

## `arguments` prefixes of the nine runs scored incomplete (first 300 chars)

```text
run 01: '{"argv": ["python", "-"], "effect": "state_changing", "intent": "Write cli.py to tally/cli.py", "stdin": "import textwrap, pathlib\\n\\nbody = textwrap.dedent(\\"\\"\\"\\\\\\n    \\\\\\"\\\\\\"\\\\\\"Command-line entry point: ``python -m tally report FILE [--top N] [--month YYYY-MM]``.\\\\\\"\\\\\\"\\\\\\"\\n\\n    import argp'
run 02: '{"argv": ["python", "-"], "effect": "state_changing", "intent": "Write new content to tally/cli.py replacing existing file", "safety_review": "Action: overwrite tally/cli.py with new source code. Scope: single file in the tally-l1 workspace. Destructive risk: replaces existing file contents (low bla'
run 03: '{"argv": ["python", "-"], "effect": "state_changing", "intent": "Replace tally/cli.py with the provided Python source body", "stdin": "content = \'\'\'\\\\\\n\\"\\"\\"Command-line entry point: ``python -m tally report FILE [--top N] [--month YYYY-MM]``.\\"\\"\\"\\n\\nimport argparse\\nimport re\\nimport sys\\n\\nfrom'
run 05: '{"argv": ["python", "-"], "effect": "state_changing", "intent": "Replace tally/cli.py with the specified CLI entry-point source.", "safety_review": "Overwrites tally/cli.py with a new implementation. Destructive if the file has uncommitted changes, but scoped to one file. User provided exact content'
run 06: '{"argv": ["python", "-"], "effect": "state_changing", "intent": "Write exact file body to tally/cli.py (UTF-8, LF)", "safety_review": "Write to tally/cli.py — replaces file contents in workspace. Low destructive risk; no system changes, no secrets exposed. Rollback easy via git if tracked.", "stdin"'
run 07: '{"argv": ["python", "-"], "effect": "state_changing", "intent": "Write tally/cli.py with exact new contents", "safety_review": "Safety: File write to tally/cli.py (workspace file). Overwrites existing file with provided source. Low destructive risk, no secrets or external impact. Rollback via git re'
run 08: '{"argv": ["python", "-"], "effect": "state_changing", "intent": "Replace tally/cli.py with the provided source code body.", "stdin": "import pathlib\\npathlib.Path(\'tally/cli.py\').write_text(\\n    \\"\\\\\\"\\\\\\"\\\\\\"Command-line entry point: ``python -m tally report FILE [--top N] [--month YYYY-MM]``.\\\\\\"'
run 09: '{"argv": ["python", "-"], "effect": "state_changing", "intent": "Replace tally/cli.py with the provided Python source code", "safety_review": "Safety: This command overwrites tally/cli.py with new content. It\'s a simple file write in the workspace. No secrets, no destructive system changes.", "stdin'
run 10: '{"argv": ["python", "-"], "effect": "state_changing", "intent": "Write the provided Python source exactly to tally/cli.py", "stdin": "import sys\\n\\nbody = \'\'\'\\\\\\n\\"\\"\\"Command-line entry point: ``python -m tally report FILE [--top N] [--month YYYY-MM]``.\\"\\"\\"\\n\\nimport argparse\\nimport re\\nimport s'
```

Run 04 (the only one scored complete):
`{"argv": ["python", "-"], "effect": "state_changing", "intent": "Write the provided Python source code exactly to tally/cli.py with LF line endings", "safety_review": "State-changing: overwrites tally/cli.py with user-provided content. Overwrite is intentional and replaces the entire file. No secret`.

## Lab observations (verbatim in substance)

- All 10 `argv` are `["python", "-"]`; `stdin` is a Python writer program
  with the payload as a string constant (`textwrap.dedent("""…""")` in 01 and
  02, `content = '''…'''` in 03 and 10, `pathlib.Path(...).write_text(...)`
  in 08), escaped differently from run to run, rather than the file body
  itself.
- The lines scored missing are exactly the ones containing `"""`, `\d`, or
  `\n`: they are escaped inside the string constant and therefore fail the
  verbatim line comparison. Missing counts range from 1 to 6 lines; no run
  truncated a block.
- Runs 02 and 07 produced ~12k characters of reasoning and ~4k completion
  tokens, three to five times the others, with no better completeness.
- Run 09 said in `content` "Let me first check if tally/cli.py already
  exists" but sent only the write call.
- Five runs put a visible `Safety:` in `content`, three had `content = null`;
  all ten carried `safety_review` or went straight to `stdin`.
- Gate A on the lab (`pira_ctx 1.9.0`): all five cases identical to the local
  run (exit 0, `KEEL_STDIN_OK`, A4 hex `636166c3a920e2809420e4b8ade696870a`,
  A5 immediate). Note: A3's direct `python -` output ends `\r\n` (Windows),
  while `pira_ctx` normalizes line endings and appends a trailing space in
  its display format.

## Reviewer's reading

Observations first.

1. **The structural half of Gate B is 10/10.** With a dedicated `stdin`
   string of 1.9–2.4 KB, comparable in size to the L1 payloads that arrived
   as collapsed `argv` strings, no response malformed the arguments. This
   is against the hypothesis that long tool arguments as such collapse on
   this serving stack. It does not settle it: L1's context was 10–27k tokens
   and multi-turn; this was 6.5k and single-turn. The collapses in L1 all
   had the long body inside an `argv` array element; here the long body sits
   in a top-level string field.
2. **The `stdin_complete` metric, as I operationalized it, did not measure
   what the gate asked.** I scored "every non-blank payload line appears
   verbatim in the stdin text". The task text I wrote asked for "a short
   Python program on stdin that writes the body", so the body necessarily
   sits inside a Python string literal, where lines containing `"""`, `\d`
   and `\n` must be escaped. Those are exactly the lines scored missing.
   The recorded value is therefore 1/10 on a metric that penalizes correct
   escaping. The pre-registered rule for ≤6/10 is "stop"; I am stopping and
   reporting, not implementing.
3. **The faithful measure is executable.** Each response's stdin program can
   be run in a throwaway directory and the `tally/cli.py` it writes compared
   byte for byte with the payload. `docs/dogfood/stdin/gate_b_rescore.py`
   does this (checked on synthetic perfect and no-write programs). It also
   surfaces the residual risk that matters for the mechanism: the model
   re-introduces one layer of escaping inside its own wrapper, and any slip
   there yields a wrong file with exit 0.
4. My task text biased the model toward wrapping. The zero-escaping usage
   (`python -c "import sys,pathlib; pathlib.Path('tally/cli.py').write_bytes(sys.stdin.buffer.read())"`
   with the body itself as stdin) was available and unused in 10/10. No
   instruction is added; this is noted for L1-R1 interpretation only.

## Reviewer decision (2026-09-08): (a), repair analysis

Preserved permanently: preregistered line-containment score = 1/10;
preregistered action = stop and report. The repair analysis executes the
frozen responses' stdin programs and compares the written file byte for
byte with the payload (`gate_b_rescore.py`); byte-exact is the gate
metric, normalized equality is diagnostic only; thresholds 9–10 → design
review (no implementation), 7–8 → report and stop, ≤6 → stop the
candidate. No new model calls; no frozen response changed.

## Repair analysis result (lab, 2026-09-08, Keel `8e34a81`)

Command run exactly as authorized: `python docs/dogfood/stdin/gate_b_rescore.py ~/Desktop/gate_b`.
No new model calls; frozen responses unchanged. Output verbatim:

```text
run 01: exit 0; wrote 1837 bytes; exact=False normalized=False
    @@ -51 +51 @@
    -        lines[-1] = f"{'total':<12}{sum(totals.values()):>10.2f}"
    +        lines[-1] = f"{{'total':<12}}{sum(totals.values()):>10.2f}"
run 02: exit 0; wrote 1830 bytes; exact=False normalized=False
    @@ -1 +1 @@
    -"""Command-line entry point: ``python -m tally report FILE [--top N] [--month YYYY-MM]``."""
    +"""Command-line entry point: ``python -m tally report FILE [--top N] [--month YYYY-MM]``.
    @@ -50 +50,2 @@
    -        lines = output.split("\n")
    +        lines = output.split("
    +")
    @@ -52 +53,2 @@
run 03: exit 0; wrote 1835 bytes; exact=True normalized=True
run 04: exit 0; wrote 1835 bytes; exact=True normalized=True
run 05: exit 0; wrote 1835 bytes; exact=True normalized=True
run 06: exit 0; wrote 1835 bytes; exact=True normalized=True
run 07: exit 1; no tally/cli.py written; stderr: ['SyntaxError: unterminated triple-quoted string literal (detected at line 59)']
run 08: exit 0; wrote 1889 bytes; exact=False normalized=True (differs only in line endings or trailing newline: written ends b' 0\r\n')
run 09: exit 0; wrote 1889 bytes; exact=False normalized=True (differs only in line endings or trailing newline: written ends b' 0\r\n')
run 10: exit 0; wrote 1837 bytes; exact=False normalized=False
    @@ -51 +51 @@
    -        lines[-1] = f"{'total':<12}{sum(totals.values()):>10.2f}"
    +        lines[-1] = f"{{'total':<12}}{sum(totals.values()):>10.2f}"

programs run 10/10; file byte-exact 4/10 (gate metric); equal after CRLF/trailing-newline normalization 6/10 (diagnostic only)
exited 0 but wrote incorrect bytes: 5/10; failed to create tally/cli.py: 1/10; non-zero exit: 1/10
```

| run | exit | bytes | byte-exact | normalized | cause |
|---|---|---|---|---|---|
| 01 | 0 | 1837 | no | no | f-string braces doubled (`{{'total':<12}}`): one escaping layer too many |
| 02 | 0 | 1830 | no | no | `\n` inside the body became a real newline; closing `"""` of the docstring lost |
| 03 | 0 | 1835 | **yes** | yes | |
| 04 | 0 | 1835 | **yes** | yes | |
| 05 | 0 | 1835 | **yes** | yes | |
| 06 | 0 | 1835 | **yes** | yes | |
| 07 | 1 | none | no | no | writer program itself invalid: unterminated triple-quoted string |
| 08 | 0 | 1889 | no | yes | content correct, written in text mode on Windows: CRLF |
| 09 | 0 | 1889 | no | yes | same |
| 10 | 0 | 1837 | no | no | f-string braces doubled, as run 01 |

Separately, as required: programs that exited 0 but wrote incorrect bytes
**5/10** (01, 02, 08, 09, 10); programs that created no file **1/10** (07);
non-zero exit 1/10 (07). **Every failure arose in the model's own
writer-program layer**: braces (01, 10), backslash and quote escaping (02),
an unterminated literal (07), text-mode line-ending translation (08, 09).
None arose in the tool-call channel: all ten stdin strings reached the
interpreter intact, run 07 included (its SyntaxError is in the program the
model wrote, at line 59 of that program).

### Statement

The preregistered line-containment metric scored 1/10 and correctly
triggered stop-and-report under the written protocol. Review found that this
operationalization penalized escaping required by the writer-program task.
On the same frozen responses, reviewer-authorized executable byte-exact
rescoring scored **4/10**. This repair analysis answers the intended
stdin-feasibility question: under the pre-registered thresholds (≤6/10),
**the stdin candidate is stopped.** `normalized` equality (6/10) is
diagnostic only and is not substituted.

### What this does and does not show

- Shown: a 1.9–2.4 KB `stdin` string survives the tool-call channel
  structurally intact 10/10 and reaches the interpreter unchanged. The L1
  collapse of long `argv` payloads did not recur with a dedicated string
  field (with the context-size caveat recorded above).
- Shown: when the model carries the payload inside a Python string literal
  of its own writer program, it reproduces the file byte-exactly 4/10 and
  content-exactly 6/10. The added escaping layer, not the channel, is where
  it fails, and three of the six failures are silent (exit 0, wrong bytes).
- Not shown: what the model does when the body itself is the stdin and the
  program is a fixed copier (`sys.stdin.buffer.read()` → file), the
  zero-escaping pattern. The task text I wrote asked for "a short Python
  program on stdin that writes the body", which selected the wrapper pattern
  in 10/10 runs. Gate B therefore measured "stdin + model-authored writer
  program", not "stdin as the file body". This is a confound in my design,
  recorded here, not a reason to reinterpret the score; whether a separately
  designed experiment on the zero-escaping pattern is warranted is a new
  decision for the reviewer, not a continuation of this gate.

Retained limitation regardless of score: Gate B establishes that long
structured stdin can survive the tool-call channel and can carry a writer
program that reproduces the requested payload. It does not establish that
models will naturally choose the zero-escaping stdin pattern or that stdin
alone will make real coding edits reliable. That question belongs to L1-R1,
which is not run because the candidate is stopped.
