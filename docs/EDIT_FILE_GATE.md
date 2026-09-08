# Exact-replace editing: Gate D

Status: **Gate D prepared, not yet run (2026-09-08).** No Keel code changes.

## Where this comes from

L1 (`docs/evidence/L1_2026-09-07.md`) failed on modifying existing files.
Two candidates have been stopped under their pre-registered rules and their
results are preserved unchanged:

- Optional shell stdin (`docs/STDIN_GATES.md`): transport through
  `pira_ctx` works; structural 10/10; repaired byte-exact writer result
  4/10; failures entirely in the model-authored writer/escaping layer.
- Whole-file `write_file` (`docs/WRITE_FILE_GATE.md`): structural validity
  10/10; no L1-style argument collapse 10/10; whole-file byte exactness
  6/10; the only observed difference in the four failures was the final
  newline; stopped under the written threshold, not reinterpreted, not
  redesigned.

Not implemented: stdin, `write_file`, `apply_patch`, T19.

The new candidate asks whether a smaller edit representation avoids both a
model-authored writer/escaping layer and whole-file EOF/trailing-newline
ambiguity:

```json
{ "path": "tally/cli.py", "old_text": "<exact existing text>", "new_text": "<exact replacement text>" }
```

This is a new candidate with a new pre-registered gate, not a redesign of
Gate C.

## The frozen edit

One real change the frozen L1 task needs in the seed `tally/cli.py`: replace
the block from `import argparse` through the end of `build_parser`
(`    return parser`; 21 lines, 681 bytes) with the same block plus
`import re`, a `month()` validator, and the `--month` option on the `report`
subcommand (35 lines, 1,074 bytes). The new text contains double and single
quotes, a raw regex with `\d`, an f-string with `!r`, nested calls, and
indentation. The old block occurs exactly once in the seed and ends 22
lines before EOF, so neither the file's final newline nor the EOF boundary
is inside the replacement.

Frozen before any model call, in `docs/dogfood/edit_file/frozen/`:
`original_cli.py` (the seed file, 1,434 bytes), `old_text.txt`,
`new_text.txt`, `expected_cli.py` (seed with the one replacement applied,
1,827 bytes; compiles; verified: `--month 2026-8` exits 2 with
`argument --month: month must be YYYY-MM, got '2026-8'` and no traceback). `gate_d.py` re-derives these from its constants at start and refuses
to run if the committed copies differ.

## Protocol (`docs/dogfood/edit_file/gate_d.py`)

- System prompt as in Gates B and C (`~/agent/AGENTS.md` verbatim, blank
  line, `<keel_host>` block, ask mode, L1 repository).
- One tool exposed, the diagnostic `edit_file`:

```json
{
  "name": "edit_file",
  "description": "Replace one exact block of text in an existing file.",
  "parameters": {
    "type": "object",
    "properties": { "path": {"type": "string"}, "old_text": {"type": "string"}, "new_text": {"type": "string"} },
    "required": ["path", "old_text", "new_text"]
  }
}
```

- User message: "In tally/cli.py, replace the following existing block of
  text:" + the old block in a fenced code block + "with this text:" + the
  new block in a fenced code block + "Call edit_file exactly once to make
  this change." No patch, writer, shell, encoding, escaping strategy, or
  whole-file rewrite is requested.
- 10 responses, same serving configuration as the previous gates, no
  sampling parameters, raw responses saved.

Per response:

```text
valid_edit_file_call                 first tool call is edit_file and its arguments parse as a JSON object
path_exact                           path == "tally/cli.py"
old_text_byte_exact                  old_text bytes == frozen old_text bytes
new_text_byte_exact                  new_text bytes == frozen new_text bytes
old_text_unique_in_seed              old_text occurs exactly once in the frozen seed text
replacement_produces_expected_file   (gate metric) parse; require exactly one occurrence; replace it with new_text;
                                     compare the result byte for byte with expected_cli.py
looks_like_l1_collapse               arguments not an object, or a structured object folded into a string value
```

No CRLF, whitespace, EOF or trailing-newline normalization anywhere. The
primary metric is insensitive to one presentation ambiguity by construction:
if the model includes the newline after the block in both `old_text` and
`new_text`, the replacement result is identical, while the two `*_byte_exact`
columns record it.

Thresholds, fixed in advance:

```text
9–10/10 → exact-replace editing is a credible candidate → write a minimal design for review; do not implement yet
7–8/10  → report and stop for review
≤6/10   → stop the edit_file candidate
```

The candidate and the scoring are not changed after seeing failures.

### Lab instruction

```bash
cd <Keel> && git pull --ff-only && git rev-parse HEAD
python docs/dogfood/edit_file/gate_d.py --base-url http://192.168.3.103:8000/v1 \
    --model "nvidia/Qwen3.6-35B-A3B-NVFP4" --out ~/Desktop/gate_d --runs 10
```

Report `scores.txt` in full (per row: `finish`, `usage`, `reasoning_chars`,
`content_text`, `old_text_occurrences`, byte counts; for any non-passing
row the 300-char `arguments_prefix` and the short diff of produced versus
expected). Keep the raw responses.

## If Gate D passes: what the design review must resolve

The review starts from, and does not assume approval of:

```rust
struct EditFileRequest {
    path: String,
    old_text: String,
    new_text: String,
    safety_review: Option<String>,
}
```

It must resolve: workspace/path enforcement; exact-one-match semantics;
zero-match and ambiguous-match errors; permission behavior; full-mode
safety-review ordering; SessionLog provenance; relationship to PIRA
activity memory; preservation of untouched file bytes and existing line
endings.

Not added without new evidence: fuzzy matching, regex replacement,
replace-all, automatic whitespace normalization, parent-directory creation,
new-file creation, diff preview, a general patch language.

T19 remains deferred until the file-edit regression experiment is finished,
so that a future L1-R1 stays a single-variable comparison.
