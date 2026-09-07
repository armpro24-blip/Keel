# Design: pre-execution safety handshake

Status: approved 2026-09-07 with two corrections and **implemented**; live
acceptance on both models recorded in
`docs/evidence/T15_ACCEPTANCE_2026-09-07.md`. Final patch after acceptance
(user decision): a structurally invalid request is denied with the parser's
message instead of passed through as `Allow`, so `Allow` has one meaning;
`keel log show` renders the handshake; the loop's per-call decide/execute
interleaving is pinned by a test. T15 status: **CLOSED 2026-09-07** after the probe-5
rerun passed on Qwen3.6 (`docs/evidence/T15_ACCEPTANCE_2026-09-07.md`,
addendum). Recorded limitation: batched per-call announcement ordering was
not elicited live from either model; it rests on the deterministic AgentLoop
test. The two corrections:

1. `safety_review` is mandatory only when the model declares
   `state_changing` **and** the command would otherwise execute without host
   approval (full mode, inside the workspace). On host-approved paths (ask
   mode; a working directory outside the workspace) the review is optional
   and, when supplied, is shown inside the approval prompt. Keel does not
   reject an ask-mode action because the model omitted a review; that would
   silently strengthen PIRA.
2. `Approver::announce` is a required method. A default no-op would make the
   visibility-before-execution guarantee false for a host that forgot it.

Ownership as implemented:

```text
effect classification                            → PIRA / model
review semantic adequacy                         → PIRA / model
review presence/order on no-approval execution   → Keel
host approval and execution                      → Keel
```

Invariant: **a model-declared state-changing command that would otherwise
execute without host approval never executes without a non-empty
model-provided review artifact, and that artifact is visible before
execution.**

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

> The model owns the semantic classification and review; Keel owns the
> integrity and ordering of the declared pre-execution handshake.

Keel guarantees, on the no-approval path: declared state change → review
artifact present → visible before execution. Keel does not guarantee that the
effect is classified correctly or that the review is adequate; `"Looks
fine."` passes. There is no Keel-side command or risk classifier.

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
    pub safety_review: Option<String>,  // required non-blank when StateChanging on the no-approval path
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

**Definition of `effect`.** `effect` describes *this command as issued*, not
the task it serves. The two models resolved that ambiguity in opposite
directions in the experiments: Mistral declared `echo hello` (which changes
nothing) `state_changing` 4/10 because the task was to create a file; Qwen
declared `echo -n hello` `read_only` 1/10 for the same reason inverted
(`docs/evidence/T15_HANDSHAKE_MISTRAL_2026-09-07.md`, `…QWEN36…`). The
schema description says "this command"; the mechanism tolerates the safe
direction (a review demanded where none was needed) and, by the ownership
split, does not correct the other. Across 40 treatment calls no command that
itself changes state was declared `read_only`. Keel does not infer effect
from `argv` to settle the ambiguity; that would be the classifier this design
excludes.

**Batched calls.** When a model emits several calls in one turn (Mistral
8/10 in T-write: a state-changing write followed by a read-only `type`),
each call carries its own `effect`; the handshake and the announcement apply
per call, in the loop's sequential order, before that call executes.

## Runtime order (deterministic, as implemented)

```text
1. structural validation of the whole request     (argv, operators, intent, mode, timeout, effect)
      invalid → denied with the parser's message (`not executed: <message>`); nothing runs;
                no review surfaced; the tool is never reached
2. which permission path applies                  (ask mode or outside the workspace → host approval)
3. handshake requirement on that path
      host approval path      → review optional
      no-approval path        → ReadOnly: none; StateChanging: non-empty review required,
                                 else "not executed" observation naming the missing review
4. surface                                        (announce `Safety: <review>` on the no-approval path;
                                                   include effect and any review in the approval prompt)
5. execute
```

## Where each part lives

| Part | Component | Change |
|---|---|---|
| Parse `effect`, `safety_review` | `shell::ShellRequest::parse` (structural); `ShellRequest::review()` returns the trimmed non-empty review | Structural failures produce `ToolResult::error` observations via the tool, as today |
| Surface the review before execution | `permission::PermissionEngine::decide_shell` | Approval paths: the command, working directory, declared effect, and any supplied review go into the `approve(summary)` prompt. No-approval path: `Approver::announce` (required method) receives `Safety: <model-provided review>` for a valid state-changing request before `Decision::Allow`; the REPL prints it to stderr. Read-only requests announce nothing. Structurally invalid requests are denied with the parser's message and surface nothing; `Allow` means only that the call may execute |
| Provenance | `log::Recorder` decision event; `keel log show` | For structurally valid `shell` calls, add `handshake: { effect, review_present, review_source: "model", review_validated: "presence_only" }` and render it on the decision line. A malformed call has no handshake; its denial carries the parser's reason. The review text is already in the logged `input` |
| Emission text | REPL | `Safety: <model-provided review>` verbatim; Keel adds only the prefix and never rewrites the text |
| Docs | PLAN §3, §5.4, §5.6, §9 | Ownership rows below; invariants below |

Not touched: `AgentLoop`, `Hooks::decide` signature, the loader, PIRA text,
the host block, `ask` as the default.

## PLAN changes made

§3 ownership rows: effect classification → PIRA/model; review semantic
adequacy → PIRA/model; review presence/order on no-approval execution →
Keel; host approval and execution → Keel. §5.6: the runtime order above.
§9: invariants 11–17 (missing/invalid `effect` → validation observation;
no-approval state-changing without review → not executed; `Safety:` before
execution and never for `read_only`; approval paths never require a review
and show one when supplied; structurally invalid → no review surfaced;
decision log provenance; no `argv`-based inference).

## Tests

`tests/shell.rs`: `effect` required and enum-checked; review trimmed; blank
review reads as absent; schema declares both fields. `tests/permission.rs`:
policy loading never asks or announces; ask mode prompt carries effect and
review and does not require a review; full mode read-only neither asks nor
announces; full mode state-changing announces then allows; full mode
state-changing without or with a blank review is refused with the naming
message; outside the workspace asks even in full mode and needs no review;
malformed input is denied with the parser's message in both modes and
surfaces no review. `tests/log.rs`: decision events carry the handshake
provenance and `log show` renders it. `tests/loop.rs`: two calls in one
turn interleave as decide a, execute a, decide b, execute b (a general
AgentLoop invariant, PLAN §9-2).

## Deliberately not in this design

- No command or risk classifier; no keyword checks on `argv`.
- No scoring of review content; no minimum length; no template.
- No change to PIRA text, host block, or `ask` default.
- No retry policy beyond the existing observation-and-resend behavior.
- No generalization to other tools; `read_pira_policy` stays read-only and
  unreviewed by construction.
