# Design: an explicit, finite model-call budget (T22)

Status: **for review, not approved, not implemented** (2026-09-09). The
alternatives set aside by the review are not revisited here: raising the
default, automatic `Continue`, a progress classifier, compaction, unlimited
runs, and coupling the budget to `full` mode.

## Problem this answers

L2 (`docs/evidence/L2_2026-09-09.md`): the task passed 13/13 under the
protocol, but the default fuse `max_turns = 32` was reached twice and the
run completed only because the operator was allowed to send `Continue`
messages. The fuse worked as designed (it bounded a run to 32 model calls
and stopped with an explicit error, transcript intact) and at the same time
truncated a task that was still progressing (once mid-edit, once with the
suite already green). Both statements are true; the fuse is a resource
bound, not a progress judgment. What is missing is a way for the operator to
authorize a different, still finite, bound before the session starts.

## What `max_turns` is today

- Unit: model calls. One `AgentLoop::run` handles one user message; each
  loop iteration is one model call (a tool turn or the final answer). Tool
  calls are not counted; a turn with nine tool calls costs one.
- Scope: per run. Every user message starts a fresh count.
- Value: the constant `MAX_TURNS = 32` in `main.rs`, not configurable.
- Exhaustion: `LoopError::MaxTurnsExceeded { max_turns }`; the transcript
  keeps everything appended so far; the REPL prints
  `error: agent loop exceeded max_turns = 32`; the log records
  `run_end {"error": …}`; the next user message continues from the kept
  transcript. Nothing is retried or sent automatically.
- Not a loop detector, not related to the approval mode.

## Design

**Keep the default at 32. Let the operator set a finite budget explicitly
before the session starts.**

| Aspect | Decision |
|---|---|
| Unit | model calls per user message, unchanged (the same quantity the fuse counts today; the log's `turns`) |
| Scope | session-wide: one value for every run in the session, fixed at startup; no REPL command changes it mid-session, so the number in the log is the number that applied |
| Legal values | an integer from 1 to 1000. No `0`, no `unlimited`, no negative. The upper bound is part of the design, not an implementation limit: a budget above 1000 calls for a single message is a request for unbounded operation, which this mechanism does not provide. (Reviewer may set a different bound; it must exist.) |
| Interface | `--max-turns N` on the REPL command line; default 32 when absent. Malformed or out-of-range values are a usage error at startup (`--max-turns needs an integer from 1 to 1000`). No environment variable, no config file: one place. |
| Visibility | the REPL prints `[max_turns] N per user message` at startup, unconditionally (not only under `--trace`), so the operator sees the bound that applies; the `session_start` event records `max_turns: N`. |
| Exhaustion | unchanged in kind: `MaxTurnsExceeded`, transcript kept, run ends, no automatic `Continue`, no synthetic message. The REPL line names the fact and the state: `error: model-call budget exhausted (max_turns = N); the run is incomplete; the transcript is kept`. The `run_end` event carries `turns: N` alongside `error`, so the log states how many calls were actually made (today a failed run reports no count and `session_stats.py` cannot include it). |
| Mode | independent of `ask`/`full`. Execution approval and resource authorization are different decisions; `--full` never changes the budget. |
| Ownership | `cli` parses and validates; `main` passes the value to `AgentLoop.max_turns` and to the `session_start` event; `AgentLoop` enforces as today. No new type. |

Not in this design: automatic continuation, a progress or "stuck" classifier,
compaction, an unlimited mode, per-run overrides from inside the session,
changing what a turn counts, any change to tools, PIRA, or approval.

## Why not the alternatives

- Raising the default: one run on one model does not determine a general
  default; 32 stays until more runs say otherwise.
- Automatic `Continue`: would resend on the model's behalf and bypass the
  bound instead of stating it; the transcript would no longer show where the
  operator chose to continue.
- Tying a larger budget to `full` mode: conflates "may act without host
  approval" with "may consume more calls".

## Tests

`cli`: `--max-turns 50` parsed; absent → 32; `0`, `-1`, `abc`, `1001`,
missing value → usage error naming the range. `loop`: existing fuse tests
unchanged (`max_turns` is already a field). `log`: `session_start` carries
`max_turns`; a fused run's `run_end` carries `turns` equal to the budget and
the error. REPL smoke: the startup line is printed with and without
`--trace`. Existing 95 tests green; CI on both platforms.

## After implementation: L2-R1 (pre-registration draft)

Same frozen `queuewatch` seed (`5d668de`), task text, hidden acceptance,
seed-test preservation, Keel runtime plus this change only, PIRA `4e0682d`,
Qwen3.6, vLLM 0.26.0, `full`, `--trace --record-wire`. One variable: an
explicit `--max-turns N`. **No operator `Continue` at all**: a fused run is
the result. The original L2 result stays as recorded.

The budget must have an independent resource basis and must not be set by
reference to the 76 calls observed in L2. Candidates for the reviewer:

| Basis | Derivation | Resulting N |
|---|---|---|
| wall-clock per message | operator accepts at most 10 minutes of model time per message; L2 measured ~5.0 s per call (383 s / 76) | 120 |
| context per message | operator accepts context growth to at most 64k tokens per message; L2 grew ~0.66k tokens per call from a 6.5k start | ~87 |
| declared authorization | operator authorizes a round resource ceiling for autonomous work on a task of this size | 100 |

The basis chosen, its derivation, and N are written into the L2-R1 protocol
before the run. Evidence to compare with L2: acceptance, preservation,
handshake, calls, tokens, wall time, fused or not, and, if fused, what state
the repository was left in. If L2-R1 fuses under a defensible budget, that is
evidence about the task or the model's call efficiency, not a reason to
raise N after the fact.
