# Tool-protocol audit after L2-R1 (2026-09-09)

Protocol: `docs/dogfood/L2/TOOL_PROTOCOL_AUDIT.md`. Static, read-only
steps only; no model run, no Keel or PIRA change. Steps 1a and 2 were run on
the Keel host and reported by the user; steps 1b and 3 were run by the lab
operator inside the vLLM container on the serving host and delivered as a
pack, stored verbatim in `docs/dogfood/L2/audit_2026-09-09/` (conclusion
document, probe outputs, probe scripts). This report is the Keel author's
reading of all of it; where a claim rests on the lab's source read that the
author could not repeat, that is stated.

## Summary

The four empty responses seen so far (L1 calls 18, 26, 38; L2-R1 call 40)
share one structural signature: `finish_reason: stop`, `tool_calls: 0`,
`content: ''`, and reasoning text that contains tool-call **closing** markup
(`</tool_call>` once, `</parameter>`, in L1 also `<parameter=…>` names) but
**no opening** `<tool_call>` or `<function=…>` and no `</think>`. Across all
77 responses of the two sessions, tool-call markup appears in reasoning in
exactly these 4 plus one successful control (L1 call 36: a stray
`</parameter>` and a fabricated `</thinking>`, followed by a complete call),
and in exactly the 4 no call was extracted.

Final classification, within the three offered by the protocol:
**model output non-compliant with the template.** Configuration mismatch is
excluded (step 1b: the deployment runs the parsers the model card prescribes
on the weights' own template). Parser implementation defect is excluded
(step 3: the non-streaming path scans the whole generated text in one pass
from the REASONING state, the reasoning→tool transition is reachable, and
every no-transition path preserves the matched text, so a tag absent from
the reasoning text was never generated). The server's "silent success" is
therefore the designed outcome for text that never opened a tool block, not
a lost call. Keel's behavior was correct throughout; no Keel change follows
from this audit.

## Step 1: serving configuration

### 1a, from the Keel host over HTTP

Available over HTTP:

```text
$ curl -s http://192.168.3.103:8000/version
{"version":"0.26.0"}

$ curl -s -H "Authorization: Bearer dummy" http://192.168.3.103:8000/v1/models
{"object":"list","data":[{"id":"nvidia/Qwen3.6-35B-A3B-NVFP4","object":"model","created":1788951982,"owned_by":"vllm",
"root":"nvidia/Qwen3.6-35B-A3B-NVFP4","parent":null,"max_model_len":262144,"permission":[…]}]}
```

Not available over HTTP: vLLM runs on another host (`192.168.3.103`; the
lab machine's addresses are `141.41.42.57`, `192.168.3.182`, `172.31.208.1`),
and `/server_info` and `/config` return 404 on 0.26.0. Everything the HTTP
side could not give is in 1b below.

Facts derived from the interface (each with its basis):

| Item | Value | Basis |
|---|---|---|
| vLLM | 0.26.0 | `/version` |
| model id / root | `nvidia/Qwen3.6-35B-A3B-NVFP4` | `/v1/models` |
| revision / weight hashes | see 1b | remote host |
| `max_model_len` | 262144 | `/v1/models` |
| tool-call format | XML style `<tool_call><function=NAME><parameter=NAME>…</parameter></function></tool_call>` | template rendered via `/tokenize` + `/detokenize` (below) |
| template's tool instructions | include "If you choose to call a function ONLY reply in the following format with NO suffix" and an `<IMPORTANT>` block | same |
| thinking default | on; the generation prompt ends with `<\|im_start\|>assistant\n<think>\n` | same |
| assistant history turns rendered as | `<think>\n\n</think>\n\n<tool_call>…` | same |
| auto tool choice | enabled | behavior: tool calls parsed normally in 10 gate runs and 4 dogfood sessions |
| reasoning parser | enabled | behavior: responses carry a non-empty `reasoning` field |
| default `max_tokens` / stop settings | see 1b | remote host |
| KV / prefix cache | `enable_prefix_caching=True`, `cache_dtype=fp8`, `gpu_memory_utilization=0.4`, `block_size=2144` | `/metrics` `vllm:cache_config_info` |

Rendered template excerpt (sha256
`8d94cb8fabe7e7463e3cf92c42b5c4a009dc906ebe0bb7642c3115b5863eb07c`, from a
three-message probe, not from a session):

```text
<|im_start|>system
# Tools

You have access to the following functions:

<tools>
{"type": "function", "function": {"name": "shell", …}}
</tools>

If you choose to call a function ONLY reply in the following format with NO suffix:

<tool_call>
<function=example_function_name>
<parameter=example_parameter_1>
value_1
</parameter>
…
</function>
</tool_call>

<IMPORTANT>
Reminder:
- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within <tool_call></tool_call> XML tags
- Required parameters MUST be specified
- You may provide optional reasoning for your function call in natural language BEFORE the function call, but NOT after
…
</IMPORTANT><|im_end|>
…
<|im_start|>assistant
<think>
```

### 1b, on the serving host (lab operator, inside the vLLM container)

Source: `docs/dogfood/L2/audit_2026-09-09/step1b_output.txt` (probe
`probes/step1b.sh`) and `step1b_output_fix.txt` (probe `probes/step1b_fix.sh`,
which corrects a defect of the first probe: the model is the positional
argument of `vllm serve`, not `--model`, so the first probe's
`generation_config.json` body and `<think>` check were read from a different
model's snapshot and are superseded; its sha256 table was complete and
correct). Both probes were read-only, streamed into `docker exec -i … bash -s`,
wrote no file, changed no configuration, issued no inference. `HF_TOKEN` was
redacted by the lab before handover.

| Item | Value |
|---|---|
| image / build | `vllm/vllm-openai:v0.26.0` (tag `latest` on the host, digest `sha256:ffb2d59b…abf52`, created 2026-07-25); `VLLM_BUILD_COMMIT=ffd46bfab2128bb84146050e98b51a617c6575ab` |
| installed vLLM | 0.26.0 at `/usr/local/lib/python3.12/dist-packages/vllm`; Python 3.12.13, torch 2.11.0+cu130, transformers 5.14.1 |
| launch (PID 1) | `vllm serve nvidia/Qwen3.6-35B-A3B-NVFP4 --host 0.0.0.0 --port 8000 --tensor-parallel-size 1 --trust-remote-code --quantization modelopt --kv-cache-dtype fp8 --attention-backend flashinfer --gpu-memory-utilization 0.4 --max-model-len 262144 --max-num-seqs 4 --max-num-batched-tokens 8192 --enable-chunked-prefill --async-scheduling --enable-prefix-caching --speculative-config {"method":"mtp","num_speculative_tokens":3,"moe_backend":"triton"} --load-format fastsafetensors --reasoning-parser qwen3 --tool-call-parser qwen3_xml --enable-auto-tool-choice` |
| `--reasoning-parser` / `--tool-call-parser` | `qwen3` / `qwen3_xml`; `--enable-auto-tool-choice` set; no `--chat-template` (template taken from the weights) |
| `qwen3_xml` registry | `<vllm>/tool_parsers/__init__.py:161` maps it to `qwen3_engine_tool_parser`; the state machine is `<vllm>/parser/qwen3.py` |
| model snapshot | `nvidia/Qwen3.6-35B-A3B-NVFP4 @ 1355db6a052410cfd62085d94b58866fd0f2c3c5` (`refs/main`); a second snapshot `491c2f1e…` holds byte-identical copies of the four files below |
| `chat_template.jinja` | sha256 `e84f32a23fdda27689f868aa4a1a5621f41133e51a48d7f3efcbea2839574259`, 7,764 bytes; line 53 carries the tool-format instruction; line 152 ends the generation prompt with `<think>\n` unless `enable_thinking` is false (thinking on by default) |
| `generation_config.json` | sha256 `e70c136c1b78ddc1fb0905bac8e733a4dc448d4f852a5dd75143fffc70be550e`: `do_sample: true`, `temperature: 1.0`, `top_k: 20`, `top_p: 0.95`, eos `[248046, 248044]` |
| `config.json` / `tokenizer_config.json` | sha256 `58aefa1c…0cecc` / `5186f0de…b29b`; architecture `Qwen3_5MoeForConditionalGeneration` |
| protocol tokens | `<tool_call>` 248058, `</tool_call>` 248059, `<think>` 248068, `</think>` 248069, all `special=False`; `<|im_start|>` 248045 and `<|im_end|>` 248046 are `special=True` |
| default `max_tokens` / stop | not set on the command line (vLLM defaults; the model's eos list above) |

Two consequences for the sessions under audit. First, Keel sends no sampling
parameters, so every Keel run so far (gates, L1, L1-R1, L1-R2, L2, L2-R1)
decoded with the model's shipped defaults, temperature 1.0 / top_k 20 /
top_p 0.95. Second, the four protocol tokens are non-special, so
`skip_special_tokens` cannot remove them at detokenization; an opening
`<tool_call>` the model emitted would reach the parser as text.

## Step 2: structured audit of both wires (`audit_tool_protocol.py`)

### L2-R1 (`--call 40`)

```text
requests 40  responses 40
call finish      calls content reasoning r<tc> r</tc> r<function> r<parameter> think(r/c) compl
   1 tool_calls      2      95       111     0      0           0            0 0/0          176
   2 tool_calls      3       0         0     0      0           0            0 0/0          197
   3 tool_calls      4      63        64     0      0           0            0 0/0          294
   4 tool_calls      3       0        75     0      0           0            0 0/0          231
   5 tool_calls      4      41         2     0      0           0            0 0/0          280
   6 tool_calls      3      34        68     0      0           0            0 0/0          227
   7 tool_calls      1      36         0     0      0           0            0 0/0           91
   8 tool_calls      1       0        76     0      0           0            0 0/0          100
   9 tool_calls      1      54       122     0      0           0            0 0/0          130
  10 tool_calls      1       0         2     0      0           0            0 0/0          101
  11 tool_calls      1     111       485     0      0           0            0 0/0          280
  12 tool_calls      1     429      4927     0      0           0            0 0/0         1862
  13 tool_calls      1       0       252     0      0           0            0 0/0          579
  14 tool_calls      1      71       297     0      0           0            0 0/0          285
  15 tool_calls      1       0        58     0      0           0            0 0/0          259
  16 tool_calls      1     120       267     0      0           0            0 0/0          288
  17 tool_calls      1     105       637     0      0           0            0 0/0          504
  18 tool_calls      1      29      1121     0      0           0            0 0/0          549
  19 tool_calls      4      60        67     0      0           0            0 0/0          300
  20 tool_calls      1      67        94     0      0           0            0 0/0          113
  21 tool_calls      1      65       119     0      0           0            0 0/0          126
  22 tool_calls      1     157       686     0      0           0            0 0/0         1062
  23 tool_calls      1      48        89     0      0           0            0 0/0          623
  24 tool_calls      1      36        78     0      0           0            0 0/0         1070
  25 tool_calls      1      25       838     0      0           0            0 0/0          282
  26 tool_calls      1     139      2180     0      0           0            0 0/0          752
  27 tool_calls      1     154       133     0      0           0            0 0/0          191
  28 tool_calls      1       0       150     0      0           0            0 0/0          211
  29 tool_calls      1       0        71     0      0           0            0 0/0          211
  30 tool_calls      1       0        67     0      0           0            0 0/0          197
  31 tool_calls      1     129     18469     0      0           0            0 0/0         8742
  32 tool_calls      1      51       114     0      0           0            0 0/0          815
  33 tool_calls      1      63       105     0      0           0            0 0/0          433
  34 tool_calls      1       0        53     0      0           0            0 0/0          282
  35 tool_calls      1      54       177     0      0           0            0 0/0          149
  36 tool_calls      1       0       115     0      0           0            0 0/0          359
  37 tool_calls      1       0        25     0      0           0            0 0/0           73
  38 tool_calls      1      70       104     0      0           0            0 0/0          132
  39 tool_calls      1       0         0     0      0           0            0 0/0          450
  40 stop            0       0      9967     0      1           0            0 0/0         3298

responses with tool-call markup in reasoning: 1/40
  ...and extracted tool_calls > 0: 0
  ...and extracted tool_calls = 0: 1
empty responses (no tool_calls, no content): [40]
  call 40: markup open/close 0/1, functions [], parameters [], complete-block=False

--- call 40 against the last denied edit_file in request 40 ---
denied call path: C:\Users\LM\Desktop\queuewatch-l2r1\chk.py
reasoning contains that path: False
reasoning contains old_text[:40]: False
reasoning contains new_text[:40]: False
reasoning mentions 'safety_review': False
markup: open 0 close 1 functions [] parameters [] json-names []
reasoning tail markup only: </parameter></function></tool_call>
```

### L1 (session `66a1592081861c8f`, three empty turns)

Numbering note: `audit_tool_protocol.py` numbers every response event in
the wire, including the `response_error` at position 3 (the provider
timeout of run 1). Counted over successful assistant responses only, the
same three empty turns are numbers 17, 25 and 37; the serving-host step-3
document uses that convention. The two conventions name the same
responses (script 18/26/38 = successful-response 17/25/37); L2-R1 had no
error event, so its call 40 is 40 under both. This report keeps the
script's numbering.

```text
requests 38  responses 38
call finish      calls content reasoning r<tc> r</tc> r<function> r<parameter> think(r/c) compl
   1 tool_calls      1       0        81     0      0           0            0 0/0           88
   2 tool_calls      9      74        75     0      0           0            0 0/0          642
   3 ERROR model provider error: HTTP request failed: timeout: global
   4 tool_calls      1     365       624     0      0           0            0 0/0          453
   5 tool_calls      1     138        87     0      0           0            0 0/0          802
   6 tool_calls      1      92       341     0      0           0            0 0/0          828
   7 tool_calls      1       0        60     0      0           0            0 0/0          730
   8 tool_calls      1       0       110     0      0           0            0 0/0           94
   9 tool_calls      1      80       293     0      0           0            0 0/0          823
  10 tool_calls      1      93       148     0      0           0            0 0/0          185
  11 tool_calls      1       0        65     0      0           0            0 0/0          782
  12 tool_calls      1       0      1320     0      0           0            0 0/0          979
  13 tool_calls      1      92      4332     0      0           0            0 0/0         1123
  14 tool_calls      1      52       205     0      0           0            0 0/0          937
  15 tool_calls      1      58       184     0      0           0            0 0/0          122
  16 tool_calls      1     116       166     0      0           0            0 0/0          185
  17 tool_calls      1       0       143     0      0           0            0 0/0          171
  18 stop            0       0       763     0      1           0            3 0/0          172
  19 tool_calls      1       0       143     0      0           0            0 0/0          800
  20 tool_calls      1      93       373     0      0           0            0 0/0          198
  21 tool_calls      1       0       640     0      0           0            0 0/0          720
  22 tool_calls      1      46       229     0      0           0            0 0/0          172
  23 tool_calls      1       0       287     0      0           0            0 0/0          735
  24 tool_calls      1     108       255     0      0           0            0 0/0          214
  25 tool_calls      1      62       433     0      0           0            0 0/0          220
  26 stop            0       0       811     0      1           0            1 0/0          186
  27 tool_calls      1      92      1545     0      0           0            0 0/0          484
  28 tool_calls      1       0       129     0      0           0            0 0/0          151
  29 tool_calls      1      99       328     0      0           0            0 0/0          820
  30 tool_calls      1     114       977     0      0           0            0 0/0         1519
  31 tool_calls      1     126       283     0      0           0            0 0/0         1192
  32 tool_calls      1       0        61     0      0           0            0 0/0          115
  33 tool_calls      1      93       316     0      0           0            0 0/0          308
  34 tool_calls      1      54       979     0      0           0            0 0/0          501
  35 tool_calls      1     132      1554     0      0           0            0 0/0          475
  36 tool_calls      1       0       734     0      0           0            0 0/0          260
  37 tool_calls      1     220      2123     0      0           0            0 0/0          616
  38 stop            0       0       913     0      1           0            1 0/0          191

responses with tool-call markup in reasoning: 3/37
  ...and extracted tool_calls > 0: 0
  ...and extracted tool_calls = 0: 3
empty responses (no tool_calls, no content): [18, 26, 38]
  call 18: markup open/close 0/1, functions [], parameters ['intent', 'effect', 'safety_review'], complete-block=False
  call 26: markup open/close 0/1, functions [], parameters ['safety_review'], complete-block=False
  call 38: markup open/close 0/1, functions [], parameters ['safety_review'], complete-block=False
```

Lab's reading of the four reasoning tails (viewed locally, not published):
L2-R1 call 40 ends `…Let me fix my filter_jobs_by_status test.\n</parameter>\n</function>\n</tool_call>`;
the L1 three have the same shape, with complete parameter-value text before
`</parameter>` followed by `<parameter=intent>…</parameter><parameter=effect>state_changing</parameter><parameter=safety_review>…</parameter></function></tool_call>`.
In none of the four does the reasoning text contain `<think>`, `</think>`,
`<tool_call>`, or `<function=`.

Note on the `--call 40` resend check: the last *denied* `edit_file` in
request 40 targets `chk.py`, and the reasoning of call 40 does not contain
its path or texts; the reasoning is about `filter_jobs_by_status` test
widths. So call 40 was not a byte-level resend of the denied edit; it was the
next edit in the same repair thread. The L2-R1 report's phrase "the resend
of the edit_file just denied" is corrected to "the next edit attempted after
the denial".

## Step 3: installed-source comparison (lab operator, on the serving host)

Source: `docs/dogfood/L2/audit_2026-09-09/tool_protocol_audit_step3.md`
(the lab's conclusion document, Chinese, with two appendices: the
intermediate hypotheses that were refuted, and how the evidence was
collected), obtained with the read-only probes `probes/probe4.sh` …
`probe7.sh` and the line-number section of `probes/step1b.sh`. The Keel
author has not read the vLLM source; what follows is the lab's reading, then
the author's consistency check of it against the outputs that are in the
pack.

### The two preregistered questions

**Does the tool parser receive the full generated text, or only the content
left after the reasoning parser removed the thinking segment?** The full
text. Non-streaming path: `chat_completion_full_generator`
(`<vllm>/entrypoints/openai/chat_completion/serving.py:836`) calls
`parser.parse(output.text, request, enable_auto_tools=…,
model_output_token_ids=token_ids)` at lines 893–898, the only extraction
entry on that path. `ParserEngine.parse` (`parser/engine/parser_engine.py:677`)
calls `_check_skip_tool_parsing` and then `_single_pass_parse` (line 645)
without an `initial_state`, so `_reset()` takes the configured initial state,
which for `qwen3.py:100` is `ParserState.REASONING` when thinking is on. One
state machine scans the whole text once. The two-stage route
(`extract_reasoning` at 490 stripping the think segment, then
`extract_tool_calls_from_content` at 553 starting in `CONTENT`) exists for
`ParserEngineToolAdapter` and is not on this path.

**Where did the opening tags go: consumed by a stage without producing a
call, or never emitted?** Never emitted. `(ParserState.REASONING,
"TOOL_START") → TOOL_PREAMBLE` emitting `(REASONING_END, TOOL_CALL_START)`
is defined at `qwen3.py:137-140` (comment at 136: "Tool call directly from
reasoning (implicit end)") and is live on this path: `skip_tool_parsing` is
set only by the context manager at `adapters.py:52-58`, used at three places
inside the reasoning adapter and nowhere on the `parse()` route;
`_suppress_tool_calls` (`parser_engine.py:125`) is set only when
`tool_choice == "none" and tools` (lines 410–411), which Keel never sends.

### Why a parser defect is excluded (three independent arguments)

1. **Unmatched terminal text is always preserved.** `_on_terminal`
   (`streaming_parser_engine.py:302-352`) has three no-transition exits
   (`transition is None`; the `skip_tool_parsing` branch; `skip_in_token_id_mode
   and _ever_had_token_ids`), all ending in `_emit_for_state(value)`, which
   re-emits the matched text under the current state. The only silent drop is
   `DROP_TERMINAL`, which requires `transition is None` and the terminal name
   `DROP`. The token-id strict dispatch at 285–291 routes to `_on_content`,
   which also preserves text. Hence a tag missing from the reasoning text was
   never in `output.text`; with the non-special token ids of 1b, it was never
   generated.
2. **A tool slot is created only by `TOOL_CALL_START`.** `_events_to_delta`
   (706–785) calls `_ensure_slot` on that event; `_build_extracted_result`
   (1011–1060) skips slots with neither name nor args and sets `tools_called =
   len(tool_calls) > 0`. Four responses with `tool_calls: 0` and empty content
   mean no slot ever got a name or argument, so `TOOL_CALL_START` never fired,
   so `(REASONING, "TOOL_START")` never matched.
3. **Counterfactual.** Had `<function=NAME><parameter=KEY>` been consumed by
   a transition, the following text would have reached `slot.args` as
   `ARG_VALUE_CHUNK` and `slot.name` would be set; `tools_called` would be
   true and the text would sit in `content`, not `reasoning`. Observed: false,
   and the text is in `reasoning`. The machine stayed in `REASONING` from
   start to end.

### What the model actually emitted

The four failing reasonings end with the **suffix** of a tool-call block;
the block's first three markers (`<tool_call>`, `<function=NAME>`, the first
`<parameter=KEY>`) are missing. L1 call 18 (lab numbering 17), 763
characters, has the skeleton
`[458 chars of prose] </parameter> <parameter=intent>…</parameter> <parameter=effect>…</parameter> <parameter=safety_review>…</parameter> </function> </tool_call>`;
the 458 leading characters are the model's deliberation about the shell tool
rejecting long or heavily quoted commands, not a parameter value. The model
closed a parameter it never opened and then wrote three more parameters and
the block's tail.

Control: L1 call 36 (lab 35), same session, model, and parsers,
`finish_reason: tool_calls`, one call extracted. Its reasoning ends
`…approach.\n</parameter>\n</thinking>\n\nLet me try piping Python code into the interpreter:\n\n`,
a stray `</parameter>` plus a fabricated `</thinking>` that is not in the
Qwen3 terminal table at all (`<think>`/`</think>` are). It then wrote a
complete `<tool_call>` block, which the transition consumed and extracted
normally; its opening tag is absent from the reasoning for that reason. The
same tag disorder succeeds when the block is written in full and fails when
its head is dropped. At every tag level in the four failures the opening
count is exactly one below the closing count, and the gap is a contiguous
prefix of the block.

Lab's signature table (lab numbering; see the numbering note in step 2):

| wire | response | reasoning chars | `<tool_call>`/`</tool_call>` | `<function=`/`</function>` | `<parameter=`/`</parameter>` |
|---|---|---|---|---|---|
| L1 | 17 (script 18) | 763 | 0 / 1 | 0 / 1 | 3 / 4 |
| L1 | 25 (script 26) | 811 | 0 / 1 | 0 / 1 | 1 / 2 |
| L1 | 37 (script 38) | 913 | 0 / 1 | 0 / 1 | 1 / 2 |
| L2-R1 | 40 | 9,967 | 0 / 1 | 0 / 1 | 0 / 1 |
| L1 | 35 (script 36), control | 734 | 0 / 0 | 0 / 0 | 0 / 1 |

Of 77 responses, 5 carry any tool markup in reasoning (these five); 72 carry
none.

Trigger context, lab reading: all four came during stretches in which the
model was repeatedly rebuffed (shell rejecting long or heavily quoted
commands in L1; the denied review-less edit in L2-R1) and was cycling
through alternative phrasings. The control call 36 sat in the same L1
stretch. The lab connects this to the Gate D/L1-R1 line: lowering the
structural complexity the model must hold correct in one output lowered the
failure rate. That is an observation about co-occurrence, not a measured
rate.

### Refuted intermediate hypotheses (lab appendix A, kept as preregistered)

- *Two-adapter, two-stage path makes the call unextractable* (reasoning
  adapter consumes the opening tag under `skip_tool_parsing=True`, tool
  adapter then starts in `CONTENT` on stripped text). Refuted: no
  `extract_tool_calls` call exists in `chat_completion/serving.py`; the
  non-streaming path calls `parser.parse()`.
- *Parser consumed the block prefix and then abandoned the call*, inferred
  from "exactly a contiguous prefix is missing". Refuted: all no-transition
  exits of `_on_terminal` preserve text, and no `TOOL_CALL_START` was emitted;
  reading the reasoning body confirmed the 458 characters are prose, not an
  argument value.

### Keel author's consistency check

What could be verified from the pack itself:

- Every `def` line number the lab cites for `parser_engine.py` (217, 401,
  490, 553, 645, 677, 706, 786, 1011), `streaming_parser_engine.py` (302,
  354, 370, 375), `qwen3.py` (100, 101, 114, 120, 136–140) and the
  `parser.parse` call site (`serving.py:893-898`) matches the `grep -n` /
  `sed -n` output in section 4 of `step1b_output.txt`. Line numbers for
  `serving.py:836`, `:153`, `:264-266`, `adapters.py:52-58/74/86/114`,
  `parser_engine.py:125/166-171/408-412/719` and the `_on_terminal` interior
  ranges come from probe outputs the lab did not include in the pack; they
  are taken on the lab's word.
- The lab's per-response figures agree with the Keel-side script in step 2
  under the numbering mapping: reasoning lengths 763/811/913/9,967 and 734
  for the control; `</tool_call>` = 1 and `<tool_call>` = 0 in all four;
  `<parameter=` counts 3/1/1/0 equal the script's parameter-name lists.
- Launch arguments, snapshot, hashes, sampling defaults and token ids are
  read directly from the two probe outputs, not from the conclusion document.
- The three exclusion arguments are consistent with each other and with the
  control: argument 1 predicts that a consumed `<tool_call>` disappears from
  reasoning while an unconsumed one stays; the control shows the former, the
  four failures show neither, which is only possible if the tag was absent.

What could not be verified: the source text itself, and therefore whether
some path the lab did not read could drop text. The lab's reading covered
the whole of `_on_terminal`, the dispatch above it, `_emit_for_state`,
`_on_content`, `_apply_transition`, `parse`, `_single_pass_parse`,
`_events_to_delta`, `_ensure_slot`, `_build_extracted_result`, and the
`qwen3.py` tables; `incremental_lexer.py` was read for structure only. The
residual is a lexer-level drop in a file read for structure, which the
control's preserved stray `</parameter>` and `</thinking>` argue against
but do not close.

## Classification (final)

Within the three categories of the protocol:

**Model output non-compliant with the template.** The template opens
`<think>` for the model; the model must emit `</think>` to leave the
thinking channel and then the call in the prescribed format, with reasoning
"BEFORE the function call, but NOT after". In the four failing responses no
`</think>` appears while tool-call closing markup does, and step 3 shows the
opening markup was never generated: the model wrote the tail of a call
inside the still-open thinking channel and stopped. Not a length or
truncation effect: call 40 used 3,298 completion tokens against 8,742 for
call 31 (a valid call after 18,469 characters of reasoning); totals ~39.3k
against 262,144. Four of 77 responses in these two sessions; that is an
observed count, not a rate estimate. All four followed a rejection or a run
of failed attempts on one sub-problem.

**Configuration mismatch: excluded.** The deployment uses the weights' own
template (sha256 recorded), `--reasoning-parser qwen3`,
`--tool-call-parser qwen3_xml`, `--enable-auto-tool-choice`, thinking on by
default; the same configuration extracted the other 73 responses of these
sessions, multi-call turns included, and Gate D 10/10.

**Parser implementation defect: excluded.** The output was not compliant,
so "compliant output, wrong extraction" does not apply; and the source read
shows the parser could not have consumed an opening tag without producing a
call. The earlier "secondary observation" (silent success instead of an
error) is resolved as designed behavior: text that never opens a tool block
is reasoning, and reasoning ending at eos is a normal `stop`. Whether vLLM
*should* flag closing-only markup is a question for vLLM, not evidence of a
defect in what it does.

**Not Keel's.** Keel denied the review-less edit, received a response with
no tool calls and no text, reported it, kept the transcript, and resent
nothing. Keel continues to execute only formal `tool_calls`; nothing in
reasoning is promoted to an action.

## What remains open

1. **Recurrence risk is still unknown.** Four occurrences over two sessions
   and 77 responses is a count, not a rate; the audit explains the mechanism
   of each occurrence, not how often it will recur under the frozen L2 task.
2. **A Keel-side mitigation was suggested by the lab and is not adopted
   here.** Section 6 of the lab document, marked by the lab as outside the
   audit's scope: detect the signature of step 3 (`stop`, no calls, empty
   content, `</tool_call>` in reasoning without `<tool_call>`) and retry the
   turn, without touching the model or vLLM. This conflicts with two standing
   decisions (no automatic retry after an empty response, T19; Keel executes
   only formal `tool_calls` and reads nothing from reasoning). It is recorded
   in PLAN §8 as a pending review item for the user's decision; nothing is
   implemented.
3. Step 4 (the preregistered minimal probe) is not needed: steps 1–3
   discriminated.

L2-R2 stays unapproved; the L2-R1 result (10/13, incomplete) stands
permanently regardless of any later run.
