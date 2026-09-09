# Design: an explicit, finite model-call budget (T22)

Status: **approved 2026-09-09 with three implementation constraints, and
implemented**: (1) the range `1..=1000` stands, but 1000 is this version's
conservative product ceiling, not a mathematical or model-capability
boundary, and moving it needs independent evidence; (2) `run_end` carries
`turns` on exhaustion, and `session_stats.py` distinguishes success from
failure by `error`, sums calls separately, and reports an incomplete count
rather than guessing when a failed run has no `turns`; (3) the value applies
to the whole session while the count starts again with each user message;
it is not a session-wide cumulative limit. The alternatives set aside by the
review are not revisited here: raising the default, automatic `Continue`, a
progress classifier, compaction, unlimited runs, and coupling the budget to
`full` mode.

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
| Scope | the configured value applies to the whole session and is fixed at startup (no REPL command changes it); the count starts again with each user message. It is not a cumulative limit across the session. |
| Legal values | an integer from 1 to 1000. No `0`, no `unlimited`, no negative. 1000 is this version's conservative product ceiling, not a mathematical or model-capability boundary (1001 would still be finite); there is no unlimited setting, and a future change of the ceiling needs its own evidence. |
| Interface | `--max-turns N` on the REPL command line; default 32 when absent. Malformed or out-of-range values are a usage error at startup (`--max-turns needs an integer from 1 to 1000`). No environment variable, no config file: one place. |
| Visibility | the REPL prints `[max_turns] N per user message` at startup, unconditionally (not only under `--trace`), so the operator sees the bound that applies; the `session_start` event records `max_turns: N`. |
| Exhaustion | unchanged in kind: `MaxTurnsExceeded`, transcript kept, run ends, no automatic `Continue`, no synthetic message. The REPL line: `error: model-call budget exhausted (max_turns = N); the run is incomplete; the transcript is kept`. The `run_end` event carries `turns: N` alongside `error` (exactly N calls were made). Other failures (provider error, empty response) record no `turns`: the count is unknown there and is not filled with the budget. `session_stats.py` classifies runs by `error`, sums calls over runs that carry `turns`, and reports the total as incomplete when any run lacks one. |
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

`cli`: absent → 32; `1`, `50`, `1000` parsed (range endpoints included);
`0`, `1001`, `-1`, `abc`, empty, `32.0`, missing value → the usage error
naming the range. `loop`: the existing fuse test also checks the exhaustion
message. `session_stats.py`: a self-check (`test_session_stats.py`) with a
mixed log (a completed run with `turns`, an exhausted run with `error` and
`turns`, an old-style failed run with `error` only) must report 3 runs,
1 completed, 2 failed, and an incomplete call count. `session_start` with
`max_turns` and the `run_end` `turns` on exhaustion are exercised by the
REPL path (`main.rs`), covered by the lab run. Existing tests green; CI on
both platforms.

## After implementation: L2-R1 (pre-registration draft)

Same frozen `queuewatch` seed (`5d668de`), task text, hidden acceptance,
seed-test preservation, Keel runtime plus this change only, PIRA `4e0682d`,
Qwen3.6, vLLM 0.26.0, `full`, `--trace --record-wire`. One variable: an
explicit `--max-turns N`. **No operator `Continue` at all**: a fused run is
the result. The original L2 result stays as recorded.

**Reviewer's choice (2026-09-09): declared authorization, N = 100.** The
basis is fixed as: "This experiment pre-authorizes at most 100 model calls
for the single task. It is an experimental resource allowance, not a
prediction of the calls needed to complete, and it guarantees no time or
context bound." The two derivations were not chosen: 383 s / 76 is an
average wall-clock per call that includes tool execution, so 120 calls
would not be a ten-minute guarantee; context growth is not linear, so 87
calls would not guarantee staying under 64k. Honesty note: 100 is not a
value chosen blind to L2 (76 calls were observed before it was set); it is
not a fitted success threshold, and a later single success is not to be
read as "the budget was optimal". Evidence to compare with L2: acceptance, preservation,
handshake, calls, tokens, wall time, fused or not, and, if fused, what state
the repository was left in. If L2-R1 fuses under a defensible budget, that is
evidence about the task or the model's call efficiency, not a reason to
raise N after the fact.
