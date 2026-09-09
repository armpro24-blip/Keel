# Tool-protocol audit after L2-R1 (2026-09-09)

Protocol: `docs/dogfood/L2/TOOL_PROTOCOL_AUDIT.md`. Static, read-only
steps only; no model run, no Keel or PIRA change. Reported by the user; the
script outputs are verbatim. Where a step could not be completed on the lab
machine, that is stated and the exact remote command that would complete it
is recorded.

## Summary

The four empty responses seen so far (L1 calls 18, 26, 38; L2-R1 call 40)
share one structural signature: `finish_reason: stop`, `tool_calls: 0`,
`content: ''`, and reasoning text that contains tool-call **closing** markup
(`</tool_call>` once, `</parameter>`, in L1 also `<parameter=…>` names) but
**no opening** `<tool_call>` or `<function=…>` and no `</think>`. Across all
77 responses of the two sessions, tool-call markup appears in reasoning in
exactly these 4, and in exactly these 4 no call was extracted; every one of
the other 73 responses had clean reasoning and extracted calls (including
multi-call turns and an 18,469-character reasoning). The common-cause
hypothesis from L2-R1 is therefore supported by structure. Classification
within the three offered: **model output non-compliant with the template
(the call emitted while the thinking channel was still open) as the primary
cause**, with one secondary observation to be confirmed on the serving host:
the server answered such output with a silent "success" (`stop`, empty
content, no calls) rather than any error or degradation. Whether that is a
parser design choice or a defect needs the installed parser source, which is
on another host and was not readable from the lab machine.

## Step 1: serving configuration (partly unavailable)

Available over HTTP:

```text
$ curl -s http://192.168.3.103:8000/version
{"version":"0.26.0"}

$ curl -s -H "Authorization: Bearer dummy" http://192.168.3.103:8000/v1/models
{"object":"list","data":[{"id":"nvidia/Qwen3.6-35B-A3B-NVFP4","object":"model","created":1788951982,"owned_by":"vllm",
"root":"nvidia/Qwen3.6-35B-A3B-NVFP4","parent":null,"max_model_len":262144,"permission":[…]}]}
```

Unavailable, and why: vLLM runs on another host (`192.168.3.103`; the lab
machine's addresses are `141.41.42.57`, `192.168.3.182`, `172.31.208.1`).
The process listing on the lab machine matched only the PowerShell command
itself. Therefore the literal `--tool-call-parser`, `--reasoning-parser`,
`--enable-auto-tool-choice`, `--chat-template` values, the model revision,
the weights' `config.json` / `generation_config.json` / template hashes, the
default `max_tokens`, and stop settings could not be recorded. `/server_info`
and `/config` return 404 on 0.26.0.

Equivalent facts derived from the interface (each with its basis):

| Item | Value | Basis |
|---|---|---|
| vLLM | 0.26.0 | `/version` |
| model id / root | `nvidia/Qwen3.6-35B-A3B-NVFP4` | `/v1/models` |
| revision / weight hashes | unavailable | remote host |
| `max_model_len` | 262144 | `/v1/models` |
| tool-call format | XML style `<tool_call><function=NAME><parameter=NAME>…</parameter></function></tool_call>` | template rendered via `/tokenize` + `/detokenize` (below) |
| template's tool instructions | include "If you choose to call a function ONLY reply in the following format with NO suffix" and an `<IMPORTANT>` block | same |
| thinking default | on; the generation prompt ends with `<\|im_start\|>assistant\n<think>\n` | same |
| assistant history turns rendered as | `<think>\n\n</think>\n\n<tool_call>…` | same |
| auto tool choice | enabled | behavior: tool calls parsed normally in 10 gate runs and 4 dogfood sessions |
| reasoning parser | enabled | behavior: responses carry a non-empty `reasoning` field |
| default `max_tokens` / stop settings | unavailable | remote host |
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

## Step 3: installed-source comparison (not executable from the lab machine)

`import vllm` fails and no vLLM source tree exists on the lab machine
(`192.168.3.182`); the installed 0.26.0 lives on `192.168.3.103`. An SSH
attempt from the Keel session toward that host was stopped by the permission
policy as cross-host remote execution needing separate authorization (port
22 open, key in `known_hosts`, `~/.ssh/config` user `infolabor`: the
channel itself works). File paths and line ranges therefore could not be
cited. The step is completed by running
`docs/dogfood/L2/tools/serving_host_probe.sh` **on the serving host itself**
(if the host named `spark-a4b3` is that machine, run it there directly);
equivalent manual commands:

```bash
python -c "import vllm, os; print(vllm.__version__, os.path.dirname(vllm.__file__))"
ps -ef | grep vllm            # --tool-call-parser / --reasoning-parser / --chat-template / --enable-auto-tool-choice
sed -n '1,200p' <vllm>/entrypoints/openai/tool_parsers/qwen3xml_tool_parser.py      # or the parser actually enabled
sed -n '1,200p' <vllm>/reasoning/qwen3_reasoning_parser.py                           # or the parser actually enabled
grep -rn "tool_call" <vllm>/entrypoints/openai/serving_chat.py | head -40
```

The decisive question for that read: does the enabled tool parser's
`extract_tool_calls` receive the full generated text or only the text left
after the reasoning parser has removed the thinking segment? And where did
the opening `<tool_call>` / `<function=…>` tags go: never emitted by the
model, or consumed by a parser stage without producing a call?

## Classification

Within the three categories of the protocol:

**Primary: model output non-compliant with the template.** The template
opens `<think>` for the model; the model must emit `</think>` to leave the
thinking channel and must then emit the call in the prescribed format, with
reasoning "BEFORE the function call, but NOT after". In the four failing
responses no `</think>` appears anywhere while tool-call closing markup does,
so the call was begun inside the still-open thinking channel. Not a length or
truncation effect: call 40 used 3,298 completion tokens against 8,742 for
call 31 (which produced a valid call after 18,469 characters of reasoning);
totals ~39.3k against 262,144. Not a configuration mismatch in the ordinary
sense: the same template and parsers handled the other 73 responses of these
two sessions, multi-call turns included, and Gate D 10/10 under the same
configuration. Context of all four: each came immediately after a rejection
or repair setback (L2-R1: an `edit_file` denied for a missing review; L1:
repeated failed attempts to write a patch script), during protracted work on
one sub-problem. Four of 77 responses in these two sessions; that is an
observed count, not a rate estimate.

**Secondary, to be confirmed on the serving host:** the server returned the
non-compliant output as a normal completion (`finish_reason: stop`, empty
content, no calls) rather than an error or a degraded signal. Whether the
tool parser only scans content after reasoning removal (a design choice that
makes this outcome expected) or should have recovered the block (a defect)
decides how much of the failure is booked to the parser. The absence of the
opening tags from the reasoning text is the specific open question.

**Not Keel's.** Keel denied the review-less edit, received a response with no
tool calls and no text, reported it, kept the transcript, and resent nothing.
Keel continues to execute only formal `tool_calls`; nothing in reasoning is
promoted to an action. Diagnostic visibility (the response does carry the
distinguishing markup) and execution recovery remain two different questions,
and neither mechanism is requested by this audit.

## What remains open, and the next static step

1. The read-only source read on the serving host (step 3, via
   `serving_host_probe.sh`). It settles the secondary observation and the
   fate of the opening tags. It requires no model run and no configuration
   change, but it does require authorized access to `192.168.3.103`, which
   is the reviewer's decision, not something the Keel session initiates. The
   decisive reading: if `serving_chat.py` first strips the thinking segment
   and hands only the remaining content to `extract_tool_calls`, then a call
   written inside an unclosed think block is structurally unextractable and
   the four empty responses are the necessary outcome of the model's output,
   not a parser accident.
2. Only if that read cannot discriminate, the pre-registered minimal probe
   of the protocol's step 4, submitted for approval first.

L2-R2 stays unapproved; the L2-R1 result (10/13, incomplete) stands.
