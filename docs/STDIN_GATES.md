# Optional stdin on the shell tool: feasibility gates

Status: **Gate A passed locally (2026-09-08); Gate B prepared, not yet run.**
Nothing in Keel is changed. The candidate mechanism under evaluation, and
the only one, is:

> Add optional UTF-8 stdin to the existing shell invocation, preserving the
> existing permission, PIRA, handshake, workspace, logging, and
> process-execution path.

Why this candidate: L1 (`docs/evidence/L1_2026-09-07.md`) showed that
`run_process` gives the child `Stdio::null()`, so every multi-line file body
had to travel inside a doubly quoted argv element; 28 of 42 calls were spent
on that and none succeeded, and the model itself reached for `python -`,
which received nothing. Excluded by decision: `write_file`, `edit_file`,
`apply_patch`, a second mutation path, retry, new shell semantics, an I/O
abstraction hierarchy, async, streaming, file-writing heuristics.

## Gate A: does `pira_ctx` forward stdin to the wrapped child?

Script: `docs/dogfood/stdin/gate_a.py` (Python `subprocess.run` with exact
bytes on stdin, no shell, no quoting; a fresh temp directory as the
workspace). Local run, 2026-09-08 07:31 UTC, Windows, `pira_ctx 1.8.0`,
Python 3.14.0. **Lab must repeat once on `pira_ctx 1.9.0`** (the version L1
used); the script is 30 seconds.

| Case | argv | stdin bytes | exit | stdout (pira_ctx synopsis) |
|---|---|---|---|---|
| A1 wrapped, exact bytes, no trailing newline | `pira_ctx --intent "Verify stdin passthrough" -- python -` | `print("KEEL_STDIN_OK")` | 0 | `L1 stdout: KEEL_STDIN_OK` |
| A2 wrapped, with trailing newline | same, intent "…with newline" | `print("KEEL_STDIN_OK")\n` | 0 | `L1 stdout: KEEL_STDIN_OK` |
| A3 control, no pira_ctx | `python -` | `print("KEEL_STDIN_OK")` | 0 | `KEEL_STDIN_OK\r\n` |
| A4 wrapped, multi-line UTF-8 script writing a file | `pira_ctx --intent … -- python -` | 6-line script containing `café — 中文` | 0 | `KEEL_STDIN_OK 17 636166c3a920e2809420e4b8ade696870a`; file on disk has the same 17 bytes |
| A5 wrapped, child ignores 200,000 bytes of stdin | `pira_ctx --intent … -- python -c "print('ignored stdin')"` | 200,000 × `x` | 0 | `L1 stdout: ignored stdin`; returned immediately |

`pira_ctx history` in that workspace shows the four wrapped runs with exit 0
and their intents. stderr was empty in every case.

**Gate A: pass.** `pira_ctx` forwards stdin to the wrapped program
byte-for-byte (A4's hex equals the UTF-8 of the script's string), with or
without a trailing newline, and does not block when the child never reads
it (A5). Every case ran in `pira_ctx` automatic mode, which retained the
output (`Captured: …`). No special case for direct PIRA-internal-tool
invocations is needed: none of the four internal tools reads stdin, and the
direct path already passes `Stdio` unchanged; A3 shows plain `python -`
behaves the same as the wrapped run apart from `pira_ctx`'s framing.

Raw output of the local run is in `docs/evidence/STDIN_GATE_A_2026-09-08.md`.

## Gate B: can the model put a long payload in a `stdin` field and keep `argv` an array?

L1 saw 8/42 calls whose `argv` arrived as one JSON string, always on long
payloads; five of them had the whole arguments object inside that string.
Before assuming stdin helps, the serving stack must show it can carry a
multi-line body in a dedicated string field while the rest of the call stays
structured.

Script: `docs/dogfood/stdin/gate_b.py`. It builds the request Keel would send
at the start of an `ask`-mode session in the L1 repository (system =
`~/agent/AGENTS.md` verbatim + blank line + `<keel_host>` block; tools =
`read_pira_policy` with the routing-table names and `shell` with the current
Keel schema plus only the `stdin` property below), sends it 10 times with no
sampling parameters (Keel sets none), saves every raw response, and scores.

```json
"stdin": {
  "type": "string",
  "description": "Optional UTF-8 text supplied verbatim to the program's standard input. Use for non-interactive programs that read from stdin."
}
```

The user message gives the model a 54-line `cli.py` body (the `--month`
version, with single and double quotes, an f-string with braces, a regex,
backslashes; 1,835 bytes) and asks for one shell call, `argv = ["python", "-"]`,
with a short writer program on stdin that writes the body to
`tally/cli.py` byte for byte.

Scoring per response (`scores.txt`):

```text
valid shell tool call          first tool call is shell and its arguments parse as a JSON object
argv remains JSON array        argv is a non-empty array of strings
stdin is JSON string           stdin is a string
stdin materially complete      every non-blank line of the 54-line body occurs in stdin
effect/intent remain separate  intent is a string, effect is one of the two values, argv is not a string
```

Interpretation, fixed in advance:

- 9–10/10 structurally valid and materially complete: stdin is a credible
  minimal mechanism; write the design (next section) for review.
- 7–8/10: report; do not implement without review.
- ≤6/10, or the long payload still collapses the arguments: stop. The
  failure would sit in the model's or the tool parser's handling of long
  structured arguments, which a `write_file(content=…)` tool would share.

### Lab instruction

```bash
cd <Keel> && git pull --ff-only && git rev-parse HEAD
python docs/dogfood/stdin/gate_a.py                 # 30 s; report the whole output (pira_ctx 1.9.0)
python docs/dogfood/stdin/gate_b.py --base-url http://192.168.3.103:8000/v1 \
    --model "nvidia/Qwen3.6-35B-A3B-NVFP4" --out ~/Desktop/gate_b --runs 10
```

Report: the Gate A output; `gate_b/scores.txt` in full; for every response
that is not fully valid, the raw `tool_calls[0].function.arguments` string
(first 300 characters) and `finish_reason`; the `usage` of each response;
vLLM version and the model's `max_model_len`. Keep the raw responses.

## If both gates pass: the design to be reviewed before any code

Kept to the shape given in the review; recorded here so the design review
has one place to look.

- `ShellRequest` gains `pub stdin: Option<String>` and nothing else changes
  in its shape. `parse` reads `stdin` with the existing `optional_string`
  (a non-string value is rejected before execution with `'stdin' must be a
  string`). No `ProcessRequest`, `IoSpec`, or `InputChannel`.
- Schema: the property above, verbatim. Keel does not interpret stdin. It
  changes nothing about `effect` ownership, review semantics, host approval,
  workspace classification, `pira_ctx` wrapping, or command classification.
  The model still declares `effect` for the command as issued; a
  state-changing `python -` follows the existing review/approval path. Keel
  never inspects stdin to infer effect.
- `run_process` takes `stdin: Option<&str>`. Absent → `Stdio::null()` as
  today. Present → `Stdio::piped()`; one small writer thread owns the child's
  stdin handle, writes exactly `stdin.as_bytes()`, then drops the handle
  (closing the pipe). No newline is added; no newline translation. A write
  error (child exited before reading everything: broken pipe) is ignored in
  the writer, so a child that closes stdin early neither hangs nor panics.
  The existing wait/timeout loop and stdout/stderr collection are unchanged;
  the writer thread cannot block the loop because the loop never joins it.
- Wrapping: the request still becomes `pira_ctx … -- argv…` (or the direct
  argv for a PIRA internal tool); stdin is handed to that process and reaches
  the target through `pira_ctx` (Gate A).
- Approval rendering: the summary shows `stdin: N UTF-8 bytes` instead of
  the body; the full body stays in the logged tool-call `input`. No preview,
  diff, highlighting, hashing, or truncation UI.
- Logging: nothing new; stdin is part of `input`, verified by a test. No
  duplicate field.
- Tests, at minimum, the twelve listed in the review: absent stdin unchanged;
  exact delivery to a direct child; delivery through `pira_ctx`; multi-line
  UTF-8 byte-preserved; no appended newline; ignoring/closing child does not
  hang or panic; timeout still works with stdin; non-string stdin rejected
  before execution; state-changing stdin command follows the handshake;
  ask-mode prompt shows the byte count and not the body; SessionLog retains
  stdin in `input`; existing shell and T15 tests green.
- Regression: rerun the frozen L1 (`L1-R1`) with the same seed, task,
  acceptance, model configuration, `ask` mode, PIRA baseline, and no added
  instruction about stdin beyond the tool description. If editing is still
  unreliable with a working stdin channel, stop and report; that would be
  independent evidence for a dedicated file-edit capability.

## Separate issue, not part of this mechanism

An assistant response with no tool calls and blank text was treated as a
successful final answer three times in L1. Registered in PLAN §8 as T19 with
the proposed smallest fix (`LoopError::EmptyAssistantResponse`, no retry).
It will be reviewed and, if approved, committed separately from stdin.
