#!/usr/bin/env bash
# Round 5 (final): confirm two things.
#  1. what skip_tool_parsing does inside the engine (does it disable the
#     REASONING -> TOOL_PREAMBLE implicit-end transition, and is the matched
#     <tool_call> terminal dropped from the reasoning text?)
#  2. the non-streaming order in the chat serving layer: is extract_reasoning
#     called first and its stripped content handed to extract_tool_calls?
# Read-only.
set -u
V="$(python3 -c 'import vllm, os; print(os.path.dirname(vllm.__file__))')"
PE="$V/parser/engine/parser_engine.py"
SV="$V/entrypoints/openai/chat_completion/serving.py"
echo "vllm root: $V"; wc -l "$PE" "$SV"

echo; echo "=== A. skip_tool_parsing: every occurrence in the engine ==="
grep -rn "skip_tool_parsing" "$V/parser" --include=*.py

echo; echo "=== B. parser_engine.py: extract_reasoning ==="
LN="$(grep -n "def extract_reasoning" "$PE" | head -1 | cut -d: -f1)"
echo "line: ${LN:-none}"; [ -n "$LN" ] && sed -n "${LN},$((LN+55))p" "$PE"

echo; echo "=== C. parser_engine.py: extract_tool_calls_from_content ==="
LN="$(grep -n "def extract_tool_calls_from_content" "$PE" | head -1 | cut -d: -f1)"
echo "line: ${LN:-none}"; [ -n "$LN" ] && sed -n "${LN},$((LN+55))p" "$PE"

echo; echo "=== D. parser_engine.py: where a transition is looked up / skipped ==="
grep -n "transitions\|skip_tool_parsing\|ParserState.TOOL\|def _feed\|def feed\|def _run\|drop\|discard" "$PE" | head -50

echo; echo "=== E. serving.py: the non-streaming response builder ==="
grep -n "extract_reasoning\|extract_tool_calls\|def chat_completion_full_generator\|def _create_chat_completion\|reasoning\b" "$SV" | head -50

echo; echo "=== F. serving.py: 60 lines around the first extract_tool_calls ==="
LN="$(grep -n "extract_tool_calls" "$SV" | head -1 | cut -d: -f1)"
echo "line: ${LN:-none}"; [ -n "$LN" ] && sed -n "$((LN>40?LN-40:1)),$((LN+20))p" "$SV"
