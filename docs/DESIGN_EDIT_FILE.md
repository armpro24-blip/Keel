# Design: `edit_file`, exact replacement of one block in an existing file

Status: **for review, not approved, not implemented** (2026-09-08). Gate D
passed 10/10 (`docs/evidence/EDIT_FILE_GATE_D_2026-09-08.md`). The
alternatives stopped under their own gates are not revisited here.

## Problem this answers

L1 (`docs/evidence/L1_2026-09-07.md`): with an argv-only shell the model
could not reliably modify an existing source file; 28 of 42 calls went to
one file write, none succeeded, one destroyed the file. Gate B (writer
program via stdin) failed in the model's escaping layer; Gate C (whole file
as `content`) failed only on the final newline; Gate D (exact replacement
of an existing block) reproduced the intended file 10/10 with no
normalization. The smallest mechanism that fits this evidence is a tool
that replaces exactly one occurrence of an exact byte sequence in an
existing file with another, and does nothing else.

## Ownership (PLAN §3 rows to add)

```text
which block to replace, and with what          → PIRA / model   (old_text, new_text; Keel never alters either)
exact-one-match, byte preservation, path scope → Keel           (edit_file tool)
approval / handshake / execution order         → Keel           (PermissionEngine, as for shell)
review semantic adequacy                       → PIRA / model
edit provenance                                → Keel SessionLog (tool input + decision + observation)
command activity memory                        → PIRA pira_ctx  (unchanged; edits are not commands, see §7)
```

## Request

```rust
pub struct EditFileRequest {
    pub path: String,           // relative to the workspace root, or absolute
    pub old_text: String,       // must be non-empty
    pub new_text: String,       // may be empty (deletion of the block); must differ from old_text
    pub safety_review: Option<String>,
}
```

`parse(&Value) -> Result<EditFileRequest, String>`: every field a string
(`safety_review` optional), `old_text` non-empty, `old_text != new_text`;
each failure is one actionable message, as `ShellRequest::parse` does. No
`effect` field: the tool's contract is a file mutation, so a model-declared
effect would add nothing the host does not already know (PLAN §3 gives
effect classification to the model only where the host cannot know it, as
for an arbitrary command). The handshake therefore applies to every
`edit_file` call on the no-approval path, unconditionally.

Tool schema:

```json
{
  "name": "edit_file",
  "description": "Replace one exact block of text in an existing file inside the workspace. old_text must occur exactly once in the file, byte for byte; bytes outside that block, including line endings, are left untouched. Read the file first and copy old_text exactly. To create a file or run a program, use shell. In full-permission/no-approval mode a safety_review is required before the edit runs.",
  "parameters": {
    "type": "object",
    "properties": {
      "path": {"type": "string", "description": "File to edit, relative to the workspace root."},
      "old_text": {"type": "string", "description": "The exact existing text to replace; must occur exactly once."},
      "new_text": {"type": "string", "description": "The exact replacement text."},
      "safety_review": {"type": "string", "description": "Required in full-permission/no-approval mode: the review PIRA's Full-Permission Behavior requires before this edit. Keel shows it as 'Safety: ...' before executing."}
    },
    "required": ["path", "old_text", "new_text"]
  }
}
```

## Runtime semantics (the tool, `src/edit.rs`)

Bytes, not text: the file is read as bytes; `old_text` and `new_text` are
used as their UTF-8 bytes. This is what preserves untouched bytes and
existing line endings by construction, and it needs no decoding of the
file (a Latin-1 or mixed-encoding file is edited without being reinterpreted).

```text
1. resolve path against the workspace root (absolute stays as is)
2. the path must be an existing regular file          else: "no such file: <path>"           (no creation, no directories)
3. read all bytes
4. count non-overlapping occurrences of old_text bytes
      0  → "old_text not found in <path>"  + hint when the file contains CRLF and old_text does not (or vice versa)
      ≥2 → "old_text occurs N times in <path>; include more surrounding text so it occurs once"
      1  → continue
5. new bytes = prefix + new_text bytes + suffix
6. write the file (truncate + write; a failure is reported as an error observation)
7. observation: "replaced 1 occurrence in <path>: <old bytes> bytes (<old lines> lines) -> <new bytes> bytes (<new lines> lines); file <before> -> <after> bytes"
```

No fuzzy matching, regex, replace-all, whitespace or newline normalization,
parent-directory creation, new-file creation, diff preview, or patch
language. The CRLF hint in step 4 is an error message computed from the
file's bytes; it changes nothing and matches nothing. Step 6 is a plain
write; atomic replacement is excluded by decision and the risk (a crash
between truncate and write) is recorded as a known ceiling with the upgrade
path (write to a sibling temp file, then rename).

## Permission and handshake (PermissionEngine)

The order is the one fixed after T15, applied to a second tool:

```text
structural validation        parse fails → Deny(parser message); nothing announced; tool never reached
path scope                   resolve → Workspace::classify_resolved → Outside?
approval path                mode == Ask || Outside → ask; the prompt shows the edit's size, never its body
no-approval path             review absent/blank → Deny("PIRA Full-Permission Behavior: an edit needs a non-empty safety_review before it runs without host approval; resend with the review")
                             review present     → announce("Safety: <review>") → Allow
```

Prompt on the approval path (L1 lesson: multi-kilobyte bodies are
unreviewable; the full input is in the log):

```text
approve? edit_file tally/cli.py
  in C:\Users\LM\Desktop\tally-l1
  replaces 681 bytes (21 lines) with 1074 bytes (35 lines)
  Safety: <model-provided review, if any>
  (outside the workspace)            ← only when Outside
[y/N]
```

`Temp` counts as not outside, as for shell's working directory (PIRA: the
platform temp directory is the standing exception). A workspace-relative
path that resolves outside (`..`, symlink) is Outside via
`classify_resolved`, as today.

To avoid a second copy of the handshake, `decide_shell`'s tail (needs
approval → prompt; else review check → announce) is extracted into one
private method taking the prompt summary, the `outside` flag, and the
optional review; `decide_shell` and `decide_edit_file` both call it. That is
the only refactor, inside `permission.rs`, and it removes duplication
rather than adding abstraction.

## Logging and provenance

Nothing new in the log format. The tool call (with full `old_text`,
`new_text`, `safety_review`) is already recorded as the assistant message
and in the decision event's `input`; the observation names the byte and
line counts. The decision event's `handshake` object for `edit_file` is:

```json
{"effect": "state_changing", "effect_source": "tool_contract", "review_present": true, "review_source": "model", "review_validated": "presence_only"}
```

`effect_source` is the one addition, so a reader can tell a contract-fixed
effect from shell's model-declared one; shell's object is unchanged.
`keel log show` renders it through the existing decision-line code.

## Relationship to PIRA activity memory (`pira_ctx`)

`pira_ctx` records *commands*: PIRA master's rule is to wrap every
shell/exec invocation. An `edit_file` call is not a shell or exec
invocation, so wrapping does not apply and the edit will not appear in
`pira_ctx history`. This matches PIRA's existing hosts: under Claude Code,
PIRA's own instructions route edits to the host's native edit tool and only
commands to `pira_ctx`, so PIRA's activity memory already excludes host
edits by design. Keel's SessionLog is the owner of edit provenance (input,
decision, observation, in order). Recorded as the ownership split above,
not as a gap to be closed. The alternative (having Keel spawn itself
through `pira_ctx` to apply the edit so that history shows it) was
considered and rejected: it adds a subcommand and a subprocess round-trip to
manufacture a history row for something PIRA does not classify as a
command.

## Not in this design

`write_file`, `apply_patch`, `stdin`, a generic `FileOperation`, an I/O
hierarchy, atomic writes, diff preview, new-file creation, directory
creation, fuzzy or regex matching, replace-all, normalization, an `intent`
field (purpose is carried by the review on the paths where it matters; the
approval prompt shows the concrete edit), any change to `shell`, T19.

## Tests that would accompany implementation

Tool: exact one match replaced and all other bytes identical (LF file);
CRLF file edited with CRLF `old_text` keeps CRLF elsewhere; zero matches →
error with the CRLF hint when applicable; two matches → error naming the
count; missing file → error; directory → error; empty `old_text` rejected;
`old_text == new_text` rejected; non-UTF-8 bytes elsewhere in the file
preserved; observation reports the counts; `new_text` empty deletes the
block. Permission: malformed → Deny with parser message, no prompt, no
announcement; ask mode prompts with byte and line counts and without the
body; full mode inside requires a review, announces `Safety:`, then allows;
blank review → Deny naming `safety_review`; outside asks in both modes and
needs no review; `Temp` does not force a prompt. Log: decision carries the
`edit_file` handshake object with `effect_source`; `input` retains the full
texts. Existing shell, T15, and loop tests unchanged and green.

## Regression after implementation

Rerun the frozen L1 as L1-R1 with `edit_file` registered and nothing else
changed (same seed, task, hidden acceptance, model configuration, `ask`
mode, PIRA baseline, operator rules; no instruction pointing the model at
`edit_file` beyond the tool description). Compare with L1: acceptance,
calls spent on file modification, malformed calls, approvals, tool errors,
and how the model handles zero- or multi-match errors if they occur. T19
stays deferred until then.
