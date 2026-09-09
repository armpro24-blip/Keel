#!/usr/bin/env bash
# Read-only probe to run ON the vLLM serving host itself (not over SSH from
# the Keel session). Prints the installed vLLM version and path, the running
# server's launch arguments, which reasoning/tool parsers exist and which are
# enabled, and the parts of the installed source that decide how reasoning
# and tool calls are separated. Nothing is modified; no server call is made.
#
#     bash serving_host_probe.sh [PYTHON]
#
# Output is meant to be pasted into docs/evidence/TOOL_PROTOCOL_AUDIT_<date>.md
# (step 1b and step 3). Long source excerpts are bounded; widen the ranges by
# hand if a decisive function is cut.
#
# Written before the installed layout was known. vLLM 0.26.0 does not have
# entrypoints/openai/tool_parsers/, reasoning/qwen3_reasoning_parser.py or
# serving_chat.py; its Qwen3 machinery is parser/qwen3.py, parser/engine/*,
# and entrypoints/openai/chat_completion/serving.py. For 0.26.0 use the
# probes the lab ran instead: ../audit_2026-09-09/probes/ (step1b.sh,
# step1b_fix.sh, probe4.sh .. probe7.sh). The step-1b section below still
# works on any version; the step-3 greps print nothing on 0.26.0.
set -u
PY="${1:-python}"

echo "=== 1b. installed vLLM ==="
"$PY" -c "import vllm, os; print(vllm.__version__, os.path.dirname(vllm.__file__))" 2>&1
V="$("$PY" -c "import vllm, os; print(os.path.dirname(vllm.__file__))" 2>/dev/null)"

echo; echo "=== 1b. server process arguments ==="
ps -eo pid,args | grep -v grep | grep -i "vllm" 2>/dev/null || echo "(no vllm process visible to this user)"

echo; echo "=== 1b. parser modules present ==="
[ -n "$V" ] && { ls "$V"/entrypoints/openai/tool_parsers/ 2>/dev/null | grep -i -E "qwen|hermes|xml" ; ls "$V"/reasoning/ 2>/dev/null | grep -i -E "qwen|deepseek|think" ; }

echo; echo "=== 1b. weights: chat template and generation config (if the model path appears in the process args) ==="
MODEL_DIR="$(ps -eo args | grep -v grep | grep -i vllm | grep -o -E '(--model[= ]|serve )[^ ]+' | head -1 | sed -E 's/^(--model[= ]|serve )//')"
echo "model argument: ${MODEL_DIR:-unknown}"
for cand in "$MODEL_DIR" "$HOME/.cache/huggingface/hub"/models--*/snapshots/*; do
  [ -d "$cand" ] || continue
  for f in chat_template.json chat_template.jinja generation_config.json config.json tokenizer_config.json; do
    [ -f "$cand/$f" ] && printf '%s  %s\n' "$(sha256sum "$cand/$f" | cut -c1-16)" "$cand/$f"
  done
done 2>/dev/null

if [ -z "$V" ]; then echo; echo "vllm not importable with $PY; stop here and report."; exit 0; fi

echo; echo "=== 3. serving_chat.py: where reasoning and tool calls are separated ==="
grep -n -E "reasoning_parser|extract_reasoning|extract_tool_calls|tool_parser" "$V"/entrypoints/openai/serving_chat.py 2>/dev/null | head -60

echo; echo "=== 3. reasoning parser(s): delimiter handling ==="
for f in "$V"/reasoning/qwen3_reasoning_parser.py "$V"/reasoning/deepseek_r1_reasoning_parser.py "$V"/reasoning/abs_reasoning_parsers.py; do
  [ -f "$f" ] && { echo "--- $f"; grep -n -E "think|def extract_reasoning|def is_reasoning_end|start_token|end_token" "$f" | head -40; }
done

echo; echo "=== 3. tool parser(s): what text extract_tool_calls receives and scans ==="
for f in "$V"/entrypoints/openai/tool_parsers/qwen3xml_tool_parser.py "$V"/entrypoints/openai/tool_parsers/qwen3coder_tool_parser.py "$V"/entrypoints/openai/tool_parsers/hermes_tool_parser.py; do
  [ -f "$f" ] && { echo "--- $f"; grep -n -E "def extract_tool_calls|tool_call_start_token|tool_call_end_token|<tool_call>|</tool_call>|<function=|model_output|content" "$f" | head -60; }
done

echo; echo "=== 3. call order in serving_chat.py (non-streaming path), 40 lines around the first extract_tool_calls ==="
LN="$(grep -n "extract_tool_calls" "$V"/entrypoints/openai/serving_chat.py 2>/dev/null | head -1 | cut -d: -f1)"
[ -n "$LN" ] && sed -n "$((LN>30?LN-30:1)),$((LN+10))p" "$V"/entrypoints/openai/serving_chat.py
