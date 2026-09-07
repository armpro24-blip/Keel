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
  |     +-- PIRA policy loader
  +-- PermissionEngine
  +-- WorkspaceManager
  +-- SessionLog
```

## Status

M2 in progress. Done so far:

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
OPENAI_API_KEY=... cargo run -- --model <model-name> [--trace] [--full]
```

`OPENAI_BASE_URL` overrides the endpoint for OpenAI-compatible servers,
including local ones (for example `http://localhost:11434/v1` for Ollama or a
vLLM / llama.cpp server). Such servers usually ignore the key, but
`OPENAI_API_KEY` must still be set to some placeholder. The model name has no
default; pass `--model` or set `OPENAI_MODEL`. The REPL requires a readable,
compatible PIRA installation at `~/agent`.

## License

Apache-2.0, matching PIRA.
