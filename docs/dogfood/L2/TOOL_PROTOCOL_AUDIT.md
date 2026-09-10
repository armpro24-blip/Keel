# Model/serving tool-protocol audit (after L2-R1)

Status: **complete (2026-09-09, `docs/evidence/TOOL_PROTOCOL_AUDIT_2026-09-09.md`).**
Steps 1a and 2 ran on the Keel host; steps 1b and 3 ran inside the vLLM
container on the serving host, delivered by the lab as a verbatim pack in
`audit_2026-09-09/` (conclusion document, probe outputs, probe scripts).
Final classification: **model output non-compliant with the template**;
configuration mismatch excluded; parser defect excluded (the non-streaming
path scans the whole text in one pass from REASONING, and every
no-transition path preserves the matched text, so the missing opening tags
were never generated). Step 4 not needed. Keel, PIRA, and the frozen workloads are
unchanged. Keel continues to execute only formal `tool_calls`; candidates,
examples, or fragments found in reasoning are never promoted to actions.

## What is established and what is not

Established (L2-R1, `docs/evidence/L2R1_2026-09-09.md`): response 40 had no
executable `tool_calls` and no visible text; Keel ended the run correctly;
the budget was not exhausted and the handshake was not bypassed. The hidden
reasoning of that response ends with `</parameter>`, `</function>`,
`</tool_call>`.

Not established: that a complete, well-formed tool call was generated (a
closing-tag tail is not an opening tag, a tool name, and valid parameters);
whose responsibility the loss is (the model writing into the wrong channel,
a chat-template / reasoning-parser / tool-parser combination mismatch, or a
parser defect); and the recurrence risk, for which one occurrence is no
sample. Working classification: **a tool-protocol failure at the
model/serving boundary; responsibility to be isolated.**

## Step 1: record the serving configuration (read-only)

Correction (2026-09-09): the first version assumed vLLM and Keel share a
machine. They do not: Keel runs on `192.168.3.182`, vLLM on
`192.168.3.103`. Step 1 is therefore two parts.

**1a, on the Keel host (HTTP side, no authorization needed):**

```bash
curl -s http://192.168.3.103:8000/version
curl -s -H "Authorization: Bearer dummy" http://192.168.3.103:8000/v1/models
curl -s http://192.168.3.103:8000/metrics | grep cache_config_info
# rendered template: POST /tokenize with a small messages+tools body, then /detokenize; record its sha256
```

**1b, on the serving host itself (process and source side):** requires
separate authorization for access to that machine; it is not run over SSH
from the Keel session. Run `docs/dogfood/L2/tools/serving_host_probe.sh`
there (read-only: version and path of the installed vLLM, the server's
launch arguments, parser modules present, template and generation-config
hashes when the weights path is visible, and the source excerpts step 3
needs). That script was written before the 0.26.0 module layout was known;
the probes the lab actually ran against 0.26.0 are `audit_2026-09-09/probes/`
(`step1b.sh`, `step1b_fix.sh`, `probe4.sh`–`probe7.sh`) and supersede it for
that version.

Record: vLLM version; model id and revision (the `/v1/models` `root`, and
the local weights directory's `config.json` / `generation_config.json`
hashes if accessible); `--tool-call-parser`; `--reasoning-parser`;
`--enable-auto-tool-choice`; `--chat-template` (or the template shipped
with the weights, its sha256); `--max-model-len`; any `--max-num-seqs`,
default `max_tokens`, and stop settings; whether thinking mode is on by
default for this deployment.

## Step 2: structured check of the L2-R1 wire (local; no content shared)

```bash
python <Keel>/docs/dogfood/L2/tools/audit_tool_protocol.py "C:\Users\LM\.keel\sessions\369f5851b4effc46\07de4bc9b279d6e5.wire.jsonl" --call 40
python <Keel>/docs/dogfood/L2/tools/audit_tool_protocol.py "C:\Users\LM\.keel\sessions\2701934f08fe7649\66a1592081861c8f.wire.jsonl"     # L1: three empty turns
```

The script prints, per response, `finish_reason`, extracted `tool_calls`,
content and reasoning lengths, and counts of `<tool_call>`, `</tool_call>`,
`<function=…>`, `<parameter=…>`, and `<think>` markers inside the reasoning
and inside the content; a session summary of how often tool-call markup
appears in reasoning with and without extracted calls; for each empty
response whether the markup forms a complete block; and for `--call 40`
whether the reasoning contains the denied edit's path and the first 40
characters of its `old_text` and `new_text`. Only structure, counts,
booleans, and markup tags are printed; no reasoning or content text.

Questions it answers: Did call 40 contain a complete `<tool_call>` block
with a function name and parameters? Was it the resend of the denied
`edit_file`? In the other 39 responses (and in L1's), does tool-call markup
ever appear inside reasoning while `tool_calls` were still extracted (which
would point at parser inconsistency), or never (which would point at the
model closing its thinking channel late or not at all on call 40)?

## Step 3: compare with the installed version's official behavior

Runs on the serving host (same authorization as 1b; the probe script prints
the excerpts). For the exact vLLM version and parsers recorded in step 1,
read the installed source (not the latest upstream): the reasoning parser's
delimiter handling (what happens when the closing think marker is absent or
appears after tool markup) and the tool parser's extraction rules (where it
looks for `<tool_call>` blocks, whether it scans text classified as
reasoning, how it treats a block with malformed parameters). State the
expected channel separation for this deployment and classify call 40 as one
of: model output non-compliant with the template (call emitted before the
thinking channel closed); configuration mismatch (template / reasoning
parser / tool parser combination not the one the model card prescribes);
parser implementation defect (compliant output, wrong extraction). Cite
file paths and line ranges of the installed source.

## Step 4: only if steps 1–3 cannot discriminate

Submit for approval, before running anything, a minimal pre-registered
probe: two conditions with fixed configuration and request counts, (a) an
ordinary tool call, (b) a tool call requested immediately after a
`not executed … safety_review` observation; structural scoring as in the
gates; raw responses kept locally. No Keel change, no PIRA change, no
change to the frozen workloads.

## Reporting

`docs/evidence/TOOL_PROTOCOL_AUDIT_<date>.md`: the step-1 record, the
step-2 script output for both sessions, the step-3 reading with citations,
and the classification with its remaining uncertainty. Then stop. L2-R2 is
not approved until the audit is reviewed; the L2-R1 result (10/13,
incomplete) stands permanently regardless of any later run.

Review outcome (user, 2026-09-10): audit accepted; one L2-R2 run approved
under the protocol in `README.md`, section "L2-R2", with no change to Keel,
PIRA, model, or serving stack.
