#!/usr/bin/env bash
# Round 7: the parser consumed <tool_call><function=NAME><parameter=KEY> and
# then emitted the remainder as reasoning, producing no tool call. Read the
# terminal/content emitters and the lexer's token-id gating to find where the
# tool parse is abandoned. Read-only.
set -u
V="$(python3 -c 'import vllm, os; print(os.path.dirname(vllm.__file__))')"
SP="$V/parser/engine/streaming_parser_engine.py"
PE="$V/parser/engine/parser_engine.py"
IL="$V/parser/engine/incremental_lexer.py"
TS="$V/parser/engine/token_id_scanner.py"
echo "vllm root: $V"; wc -l "$SP" "$IL" "$TS"

echo; echo "=== A. _on_terminal in full (the branch cut off last round) ==="
LN="$(grep -n "def _on_terminal" "$SP" | head -1 | cut -d: -f1)"
sed -n "${LN},$((LN+95))p" "$SP"

echo; echo "=== B. _on_content and _emit_for_state ==="
for f in _on_content _emit_for_state; do
  LN="$(grep -n "def $f" "$SP" | head -1 | cut -d: -f1)"
  echo "--- $f (line ${LN:-none})"; [ -n "$LN" ] && sed -n "${LN},$((LN+45))p" "$SP"
done

echo; echo "=== C. token-id gating: _token_id_terminal_names, _ever_had_token_ids, _tool_terminals ==="
grep -n "_token_id_terminal_names\|_ever_had_token_ids\|_tool_terminals\|_has_drops\|DROP_TERMINAL\|CONTENT_TERMINAL" "$SP" | head -40

echo; echo "=== D. _build_extracted_result (why tools_called ended up False) ==="
LN="$(grep -n "def _build_extracted_result" "$PE" | head -1 | cut -d: -f1)"
echo "line: ${LN:-none}"; [ -n "$LN" ] && sed -n "${LN},$((LN+60))p" "$PE"

echo; echo "=== E. the rest of _events_to_delta (tool arg accumulation) ==="
LN="$(grep -n "def _events_to_delta" "$PE" | head -1 | cut -d: -f1)"
sed -n "$((LN+55)),$((LN+130))p" "$PE"

echo; echo "=== F. incremental_lexer: how a terminal match is produced ==="
grep -n "def \|class \|partial\|pending\|buffer\|longest\|prefix" "$IL" | head -50

echo; echo "=== G. does qwen3 gate <tool_call> on token ids ==="
grep -n "token_id_terminals\|terminals\s*=\|drop_terminals\|DROP" "$V/parser/qwen3.py"
