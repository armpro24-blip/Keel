# Keel

Keel is a minimal, rigorous agent harness.

## Goal

Build a small but complete agent runtime that helps us understand agent systems
by implementing them from first principles.

Keel is a new, independent harness. It is designed to be more naturally
compatible with [PIRA](https://github.com/AlgebraLoveme/PIRA) than a generic
host such as Codex, while preserving PIRA's original design and behavior
wherever possible:

```text
PIRA + Codex Harness
PIRA + Claude Code Harness
PIRA + Keel Harness
```

PIRA provides the agent's identity, methods, memory design, and native tools.
Keel provides the runtime that gives PIRA agency: model interaction, the agent
loop, tool execution, context management, permissions, and workspace
boundaries. PIRA stays an external dependency; Keel reads and validates the
installed PIRA and never copies, forks, or rewrites it.

The design document is [`PLAN.md`](PLAN.md). Every mechanism there answers:
why does this exist, who owns it, and which PIRA behavior does it preserve or
support.

## Design principles

1. Prefer the smallest correct design.
2. Keep responsibilities explicit and avoid duplicate ownership.
3. Use simple primitives before abstractions.
4. Add a dependency only when it clearly reduces complexity.
5. Keep components independently understandable and testable.
6. Separate policy from enforcement.
7. Preserve evidence and provenance instead of hiding failures behind summaries.
8. Do not reproduce Codex, Claude Code, or PIRA blindly; understand the problem
   each mechanism solves first.
9. Build incrementally and keep the system runnable after each meaningful step.
10. Treat this repository as both a real software project and a learning
    project. Explain non-obvious architectural choices in concise comments or
    documentation.

## Architecture

```text
User (REPL)
  |
AgentLoop ── Model (trait) ── FakeModel | OneRealModelAdapter
  |
  +-- ContextManager
  +-- ToolRegistry / Dispatch
  |     +-- shell (every invocation through pira_ctx, as PIRA master requires)
  |     +-- edit_file (exact one-match replacement in an existing file, on bytes)
  |     +-- PIRA policy loader
  +-- PermissionEngine
  +-- WorkspaceManager
  +-- SessionLog
```

## Status

M0–M3 done; T15 closed. Current phase: long-horizon dogfooding
(`docs/dogfood/L1/`). L1 failed on file editing; three gated experiments led
to `edit_file` (`docs/DESIGN_EDIT_FILE.md`); the L1-R1 rerun with only that
tool added passed 7/7, and L1-R2 in full mode passed 7/7 with zero host
prompts and every state-changing action preceded by the model's announced
review (`docs/evidence/L1R1_2026-09-08.md`, `L1R2_2026-09-08.md`). L2, a harder
maintenance task on a second repository, passed 13/13 in full mode with all
seed tests preserved; its dominant friction was the default 32-call fuse
(`docs/evidence/L2_2026-09-09.md`). The L1
investigation is closed; an empty model response is now an explicit loop
error, and the shell description states that Keel does the `pira_ctx`
wrapping. Done so far:

- M0: the agent loop, a scripted `FakeModel`, a `Tool` trait with one
  deterministic tool, deterministic tests.
- M1: OpenAI Chat Completions adapter (synchronous HTTP, tested against a
  loopback server) behind a stdin/stdout REPL; validated against a local vLLM
  deployment (`docs/evidence/`).
- M2 A: workspace identity with `pira_ctx`'s rule; `keel pira check [--lock]`
  reads and validates the PIRA installation at `~/agent` and records the
  verified state in `~/.keel/pira.lock` (VERIFIED / UNVERIFIED-COMPATIBLE /
  INCOMPATIBLE).
- M2 B: the REPL runs PIRA: `AGENTS.md` verbatim as the system instruction
  plus a short host block; `read_pira_policy` loads declared modules exactly;
  tool results carry their provenance; one session id per process exported
  as `PIRA_CTX_THREAD_ID`.
- M2 C: the `shell` tool runs every command through `pira_ctx` unless the
  program is a PIRA internal tool (PIRA master's rule as a runtime
  invariant); the PermissionEngine asks before each action by default,
  `--full` runs without asking except through the handshake below, and a
  working directory outside the workspace always asks.
- M3: every session is recorded as append-only JSONL under
  `~/.keel/sessions/<workspace>/<session>.jsonl` (messages, gate decisions,
  run outcomes, in the order they happened); `keel log show FILE` renders it
  for inspection without re-running anything. Separately, `--record-wire` is
  an opt-in diagnostic capture for instruction-path audits
  (`docs/AUDIT_T15.md`): it writes the complete model-visible context and
  every response body. It is high-sensitivity by nature: local only, never
  on by default, inspect and redact before sharing, never commit.

Keel owns the instruction-delivery path. When the model does not follow a
PIRA instruction, that is investigated as a system problem first; it is
attributed to the model only after Keel has verified that the instruction
reached the model with the correct content, precedence, runtime state, and
tool semantics (`PLAN.md` §2).

`edit_file` replaces exactly one occurrence of `old_text` with `new_text` in
an existing file, on bytes: zero or several matches are errors the model
sees, every byte outside the block is preserved, nothing is created, and the
same approval and handshake rules apply as for `shell` (the effect is fixed
by the tool's contract). Evidence: `docs/evidence/L1_*`,
`docs/evidence/*GATE*`.

Pre-execution handshake (T15): every `shell` call carries the model's own
`effect` (`read_only` | `state_changing`) and, for state-changing commands
that would run without host approval, the model's `safety_review`. Keel
shows that review as `Safety: …` before the command runs and refuses to run
a declared state-changing command without one on that path. Keel checks the
review's presence and order only: the classification and the review's
content remain the model's, applying PIRA's Full-Permission Behavior. A
structurally invalid `shell` call is refused with the parser's message and
never reaches the tool. Live acceptance on two local models closed the
investigation (T15) on 2026-09-07. Evidence: `docs/evidence/T15_*`.

```text
cargo test
keel pira check --lock
OPENAI_API_KEY=... cargo run -- --model <model-name> [--trace]
OPENAI_API_KEY=... cargo run -- --model <model-name> --full --max-turns 64
OPENAI_API_KEY=... cargo run -- --model <model-name> --record-wire
keel --help                    # all flags and their descriptions
```

`OPENAI_BASE_URL` overrides the endpoint for OpenAI-compatible servers,
including local ones (for example `http://localhost:11434/v1` for Ollama or a
vLLM / llama.cpp server). Such servers usually ignore the key, but
`OPENAI_API_KEY` must still be set to some placeholder. The model name has no
default; pass `--model` or set `OPENAI_MODEL`. The REPL requires a readable,
compatible PIRA installation at `~/agent`.

`--record-wire` is the opt-in diagnostic capture described in the M3 paragraph above; read its privacy caveat there.

`--max-turns` bounds the model calls spent on one user message (default 32, legal range 1–1000). The count restarts with each user message. When it is exhausted the run stops with an explicit error ("model-call budget exhausted (max_turns = N); the run is incomplete; the transcript is kept"), the transcript is kept, and nothing is resent. There is no unlimited setting.

## License

Apache-2.0, matching PIRA.
