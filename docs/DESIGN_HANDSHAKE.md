# Design: pre-execution safety handshake (draft, not implemented)

Status: design for review. Nothing here is in the code. Implementation waits
for the Mistral run to close the preregistered gate and for the decision that
follows (`docs/T15_HANDSHAKE_AB.md`).

## Why this exists

Two local models deliver, retrieve, and acknowledge PIRA's Full-Permission
Behavior and still emit `content = null, tool_calls = […]` when acting, so a
review that must precede execution never appears when it lives only in
assistant prose (`docs/evidence/T15_SYNTHESIS_2026-09-07.md`). On Qwen3.6,
moving the review into structured tool-call metadata restored it 9/10 while
read-only commands stayed unreviewed 10/10
(`docs/evidence/T15_HANDSHAKE_QWEN36_2026-09-07.md`).

PIRA behavior preserved: a visible `Safety:` review precedes every command
the model declares state-changing in full-permission/no-approval mode. What
changes is the channel: the review travels as a field of the tool call and the
host relays it, instead of the model printing prose that these models omit.

## Ownership

```text
effect classification          → PIRA / model
review semantic adequacy       → PIRA / model
review presence and ordering   → Keel
host approval and execution    → Keel
```

> The model owns the semantic classification and review; Keel owns the
> integrity and ordering of the declared pre-execution handshake.

Keel guarantees: declared state change → review artifact present → visible
before execution. Keel does not guarantee that the effect is classified
correctly or that the review is adequate; `"Looks fine."` passes. There is no
Keel-side command or risk classifier.

## Request shape

```rust
pub enum Effect {
    ReadOnly,
    StateChanging,
}

pub struct ShellRequest {
    pub argv: Vec<String>,
    pub intent: String,
    pub effect: Effect,                 // required; the model's judgment
    pub safety_review: Option<String>,  // required non-blank when StateChanging
    pub mode: Option<String>,           // existing
    pub interest: Option<String>,       // existing
    pub workdir: Option<String>,        // existing
    pub timeout_seconds: Option<u64>,   // existing
}
```

Tool schema: `effect` (`enum: ["read_only","state_changing"]`, required) and
`safety_review` (string) are added with the descriptions used in the
experiment; the description gains the one sentence used there. JSON Schema
conditionals are not used; the schema informs, the runtime enforces.

## Runtime order (deterministic)

```text
1. structural validation of the whole request      (argv, operators, intent, mode, timeout, effect present)
      invalid → validation error observation; nothing runs; no review surfaced
2. handshake check
      StateChanging with missing/blank safety_review
        → validation error observation naming the missing review; nothing runs
3. pre-execution path
      ReadOnly                → approval per mode → execute
      StateChanging + review  → review enters the approval/announcement step → execute
```

Step 1 precedes step 3 so a review is never shown for a command that could
not run anyway.

## Where each part lives

| Part | Component | Change |
|---|---|---|
| Parse `effect`, `safety_review`; handshake check | `shell::ShellRequest::parse` (structural) and a small `ShellRequest::handshake() -> Result<(), String>` | Both produce `ToolResult::error` observations via the tool, as today |
| Surface the review before execution | `permission::PermissionEngine` (the one pre-execution point that knows the mode) | Ask mode: the review, the declared effect, and the command go into the `approve(summary)` prompt. Full mode: a new `Approver::announce(text)` (default no-op) receives `Safety: <model-provided review>` for a fully valid state-changing request; the REPL prints it to stderr before `Decision::Allow` is returned. Read-only requests announce nothing. Structurally invalid requests are allowed through unannounced so the tool reports the precise validation message |
| Provenance | `log::Recorder` decision event | For `shell` calls, add `handshake: { effect, review_present, review_source: "model", review_validated: "presence_only" }`. The review text is already in the logged `input` |
| Emission text | REPL | `Safety: <model-provided review>` verbatim; Keel adds only the prefix and never rewrites the text |
| Docs | PLAN §3, §5.4, §5.6, §9 | Ownership rows below; invariants below |

Not touched: `AgentLoop`, `Hooks::decide` signature, the loader, PIRA text,
the host block, `ask` as the default.

## Exact ownership-table changes (PLAN §3)

Replace the three T15 rows with:

```text
| effect classification (does this command change file/repository/tool/user/system state) | PIRA / model | declared per call; Keel does not correct a wrong label; the declaration is logged |
| review semantic adequacy | PIRA / model | Keel validates presence only |
| review presence and execution ordering | Keel (PreExecutionHandshake in PermissionEngine + ShellRequest) | a declared state-changing command never runs without a review artifact, and the artifact is visible before it runs |
| host approval and execution | Keel | unchanged |
```

## Invariants added (PLAN §9)

1. A `shell` call with `effect = state_changing` and no non-blank
   `safety_review` never executes; the observation names the missing review.
2. In full-permission/no-approval mode, `Safety: <review>` is emitted before
   any declared state-changing command executes, and never for `read_only`.
3. In ask mode, the approval prompt for a declared state-changing command
   contains the model's review and the declared effect.
4. A structurally invalid request surfaces no review.
5. Every logged shell decision records `review_source = model` and
   `review_validated = presence_only`; Keel never logs a review it authored.
6. Keel evaluates neither `effect` nor the review text; no code path inspects
   `argv` to infer either.

## Tests that would accompany it

Parse: `effect` required and enum-checked; blank review with
`state_changing` rejected with the naming message; `read_only` with or without
review accepted; existing structural failures still reported first.
Permission: full mode announces for valid state-changing, not for read-only,
not for invalid; ask mode prompt contains effect and review. Recorder: decision
events carry the handshake provenance fields. Loop: unchanged tests pass.

## Deliberately not in this design

- No command or risk classifier; no keyword checks on `argv`.
- No scoring of review content; no minimum length; no template.
- No change to PIRA text, host block, or `ask` default.
- No retry policy beyond the existing observation-and-resend behavior.
- No generalization to other tools; `read_pira_policy` stays read-only and
  unreviewed by construction.
