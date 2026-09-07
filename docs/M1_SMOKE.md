# M1 smoke test against a real model

> Historical protocol, closed on 2026-09-07 (see `docs/evidence/`). Since M2 B
> the REPL also requires a compatible PIRA installation at `~/agent`; the M2
> smoke protocol supersedes this one.

Purpose: close the M1 gate in `PLAN.md` §7 by observing a real model's
tool-use behavior through Keel. This is an opt-in manual run, not part of
`cargo test`. It needs a machine that can reach the model server.

## 1. Prerequisites

- Git.
- Rust toolchain via rustup (`https://rustup.rs`). Check with `cargo --version`.
- Network access to the OpenAI-compatible server.

Confirm the server first. It should list the model you intend to use:

```bash
curl -s -H "Authorization: Bearer dummy" http://192.168.3.103:8000/v1/models
```

## 2. Build and test

```bash
git clone https://github.com/armpro24-blip/Keel.git
cd Keel
cargo test
```

All suites must pass before the smoke run; otherwise report the failure and stop.

## 3. Run the REPL with tracing

Linux / macOS:

```bash
export OPENAI_BASE_URL=http://192.168.3.103:8000/v1
export OPENAI_API_KEY=dummy
cargo run -- --model mistral-small-4-119b --trace
```

Windows PowerShell:

```powershell
$env:OPENAI_BASE_URL = "http://192.168.3.103:8000/v1"
$env:OPENAI_API_KEY = "dummy"
cargo run -- --model mistral-small-4-119b --trace
```

`--trace` prints every message a run appended (tool calls, tool results,
final text) to stderr, so tool use is visible rather than inferred.

## 4. Probes

Enter these one at a time in the same REPL session, in this order:

1. `Reply with the single word pong.`
   Baseline: a plain answer, no tool call expected.
2. `Use the echo tool to echo the JSON object {"text":"hello"} and then tell me exactly what it returned.`
   One tool round trip expected: `tool_call … echo({"text":"hello"})` then a `tool_result`.
3. `Call the echo tool three times in one response with inputs {"n":1}, {"n":2} and {"n":3}, then summarize the three results.`
   Observes whether the model batches several calls in one turn or serializes them.
4. `What was the first thing I asked you in this conversation?`
   Observes transcript continuity across inputs.
5. `/quit`

## 5. Report

Send back, unedited:

- the full terminal output of steps 1 to 4 (stdout and stderr, including any `error:` lines);
- `cargo --version`, the operating system, and the output of the `/v1/models` call;
- if known, the server software and version (for example vLLM, llama.cpp, Ollama).

What the report decides: whether the model emits tool calls at all, whether
their `arguments` are well-formed JSON, whether several calls arrive in one
turn, and how long a round trip takes. These observations shape the M2 shell
tool schema and the system-instruction size Keel can rely on.
