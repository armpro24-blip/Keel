# Cross-model L2 control (T26): preparation only

Status: **preparation approved by the user on 2026-09-10; no run and no
compute purchase approved.** The phase turns from "explain why the current
model fails" to "test whether Keel's design holds across models".

## Question

How much of the observed completion limit (no autonomous L2 completion
under 100 calls and zero Continue on Qwen3.6-35B-A3B-NVFP4) belongs to the
tested model configuration, and how much to Keel's general tool design? One
control model cannot separate every cause, but it discriminates better than
another repetition on the same model.

## Frozen boundaries (do not change for the control)

- Keel runtime `822fdbc`; PIRA `4e0682dd`; L2 task, seed `5d668de`,
  hidden acceptance (13 checks), seed-test preservation (42 IDs);
  `--full --trace --record-wire --max-turns 100`; zero Continue; a question
  from the model ends the run as incomplete.
- No model-specific prompt, tool description, recovery mechanism, or budget
  change. The model sees exactly what Qwen saw.
- The control model is from a **different model family** and must work
  through the existing OpenAI-style tool interface (formal `tool_calls`
  with JSON arguments; `argv` as an array).
- Serving differences (parsers, template, sampling defaults, quantization,
  context length) are recorded as they are, not equalized.

## Preparation steps

1. **Inventory (lab, read-only, no inference):** models already available
   on the serving host or otherwise usable now; for each: family, id and
   snapshot, quantization and size, whether the installed vLLM 0.26.0 has a
   reasoning parser and a tool-call parser for it (`vllm/parser/`,
   `vllm/tool_parsers/__init__.py`), context length, whether it can be
   served alongside or only instead of the current model (GPU memory), and
   the cost of using it: local switch time and downtime for the current
   model, or per-token price and any purchase for a hosted option.
   Existing Keel evidence with that model (T15 handshake acceptance,
   screening) is cited where it exists.
2. **Choice (user):** one model. No ranking.
3. **Compatibility check (lab, after the choice, small and preregistered):**
   the five T15 acceptance probes (`docs/evidence/T15_ACCEPTANCE_2026-09-07.md`)
   on the chosen deployment: tool call parsed, `argv` array shape,
   `effect`/`safety_review` handshake in full mode, malformed-argv denial
   observed, no empty responses in the probes. Record the deployment with
   the step-1b probe (`docs/dogfood/L2/audit_2026-09-09/probes/step1b_fix.sh`
   and the argv/environ read) and the Keel-host 1a checks.
4. **Freeze the run protocol:** the L2-R2 section of `README.md` with the
   model line replaced and the serving record attached; then submit for the
   user's approval to run once.

## Interpretation, fixed now

Whatever the outcome of a single control run, it does not show that one
model is "stronger" or that Keel "has no problem". It adds one point on a
second family under identical harness conditions; the comparison is
reported alongside L2-R1 and L2-R2 as a sequence of attempts.

## Reporting

Inventory: `docs/evidence/CROSS_MODEL_INVENTORY_<date>.md`. Compatibility
check and run, if approved later: separate evidence files.
