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

Decision requested from the reviewer: (a) accept the execute-based
re-scoring as the operationalization of "stdin materially complete" and have
the lab run `gate_b_rescore.py ~/Desktop/gate_b`, applying the same
pre-registered thresholds to its byte-exact count; or (b) treat 1/10 as the
verdict and stop the stdin candidate. Keel is unchanged either way.
