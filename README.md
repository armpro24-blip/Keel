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

M1: the agent loop (M0) plus one real model adapter, OpenAI Chat Completions
with function calling, behind a stdin/stdout REPL. The only tool is `echo`,
kept to observe real tool-use behavior. No PIRA, shell, permissions,
persistence, async, Skills, MCP, or TUI yet. Each of those arrives only when an
observed need requires it (see `PLAN.md` §7).

```text
cargo test
OPENAI_API_KEY=... cargo run -- --model <model-name>
```

`OPENAI_BASE_URL` overrides the endpoint for OpenAI-compatible servers,
including local ones (for example `http://localhost:11434/v1` for Ollama or a
vLLM / llama.cpp server). Such servers usually ignore the key, but
`OPENAI_API_KEY` must still be set to some placeholder. The model name has no
default; pass `--model` or set `OPENAI_MODEL`.

## License

Apache-2.0, matching PIRA.
