# Direct structured file writing: Gate C

Status: **Gate C prepared, not yet run (2026-09-08).** No Keel code changes.

## Where this comes from

L1 (`docs/evidence/L1_2026-09-07.md`) failed on file editing. The first
candidate, optional stdin on the existing shell tool, was stopped
(`docs/STDIN_GATES.md`): the exact conclusion preserved there is that stdin
transport through `pira_ctx` works, structural transport was 10/10, the
repaired byte-exact writer result was 4/10, and every failure arose in the
model-authored writer and escaping layer. The rejected mechanism is
specifically *a model-authored writer program delivered via stdin as the
solution to reliable file editing*; stdin is not stated to be intrinsically
unreliable, and a future independent workload may justify generic process
stdin on its own.

That evidence reopens the previously deferred file-write capability. Before
any design or code, one diagnostic gate.

## Gate C question

Can the current Qwen/vLLM tool-call channel carry the required file body
directly in a top-level `content` string field, byte for byte, without a
model-authored writer program?

## Protocol (`docs/dogfood/write_file/gate_c.py`)

- Same payload as Gate B, imported from `gate_b.py` (identical by
  construction): the 54-line `cli.py` body, 1,835 bytes.
- Same system prompt as Gate B: `~/agent/AGENTS.md` verbatim, blank line,
  `<keel_host>` block in ask mode for the L1 repository.
- Only one tool exposed, the diagnostic `write_file`:

```json
{
  "name": "write_file",
  "description": "Write the supplied UTF-8 content to the specified file.",
  "parameters": {
    "type": "object",
    "properties": { "path": { "type": "string" }, "content": { "type": "string" } },
    "required": ["path", "content"]
  }
}
```

- User message: "Call write_file exactly once with path tally/cli.py and
  content equal to the following file body, exactly as given." followed by
  the body in a fenced block. The model is not asked to encode, quote, wrap,
  escape for another language, construct Python or shell, or write a patch.
- 10 responses, same serving configuration as Gate B, no sampling
  parameters. Raw responses saved (`response_NN.json`), request saved,
  scores in `scores.txt`.

Per response:

```text
valid_write_file_call   first tool call is write_file and its arguments parse as a JSON object
path_exact              arguments["path"] == "tally/cli.py"
content_is_string       arguments["content"] is a string
content_byte_exact      arguments["content"].encode("utf-8") == PAYLOAD.encode("utf-8")   (gate metric)
```

No newline or CRLF normalization participates in the gate.
`trailing_newline_only_diff` and `looks_like_l1_collapse` (arguments not an
object, or a structured object folded into a string value) are printed as
diagnostics, the latter to answer whether any malformed arguments resemble
the L1 argv collapse.

Thresholds, fixed in advance:

```text
byte-exact 9–10/10 → direct structured file content is credible → write the minimal write_file design for review; do not implement yet
byte-exact 7–8/10  → report and stop for review
byte-exact ≤6/10   → stop the write_file candidate; do not redesign after seeing the failures
```

### Lab instruction

```bash
cd <Keel> && git pull --ff-only && git rev-parse HEAD
python docs/dogfood/write_file/gate_c.py --base-url http://192.168.3.103:8000/v1 \
    --model "nvidia/Qwen3.6-35B-A3B-NVFP4" --out ~/Desktop/gate_c --runs 10
```

Report `scores.txt` in full (it includes each row's `finish`, `usage`,
`reasoning_chars`, `content_text`, and for any non-exact row the 300-char
`arguments_prefix` and the short diff). Keep the raw responses.

## If Gate C passes: what the design review must resolve

The review starts from the smallest conceptual request and does not assume
it is approved:

```rust
struct WriteFileRequest {
    path: String,
    content: String,
    safety_review: Option<String>,
}
```

It must explicitly resolve: (1) workspace and path enforcement; (2) ask/full
permission behavior; (3) pre-execution safety-review behavior; (4)
SessionLog provenance; (5) how a direct file mutation relates to PIRA's
`pira_ctx` activity-memory ownership, since such a write would not pass
through `pira_ctx`.

Not introduced until evidence requires them: `edit_file`, `apply_patch`, a
generic `FileOperation`, an I/O hierarchy, atomic-write machinery, diff
preview, parent-directory creation.

Note for the review: `write_file` is state-changing by tool contract, so the
shell tool's model-declared `effect` field is not copied mechanically;
whether an effect field is needed must be justified.

T19 (empty assistant response) and other L1 fixes are not implemented before
the file-write regression experiment, so that a future L1 comparison stays
single-variable.
