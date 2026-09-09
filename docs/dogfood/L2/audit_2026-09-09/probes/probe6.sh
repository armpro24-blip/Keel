#!/usr/bin/env bash
# Round 6 (final): serving.py's non-streaming path calls parser.parse(), not
# the two-adapter route. Read parse() itself, the request-dependent tool
# suppression flag, and the tool-terminal special case in the lexer loop.
# Read-only.
set -u
V="$(python3 -c 'import vllm, os; print(os.path.dirname(vllm.__file__))')"
PE="$V/parser/engine/parser_engine.py"
SP="$V/parser/engine/streaming_parser_engine.py"
SV="$V/entrypoints/openai/chat_completion/serving.py"
echo "vllm root: $V"

echo; echo "=== A. parser_engine.py: def parse (the path serving.py uses) ==="
for LN in $(grep -n "    def parse" "$PE" | cut -d: -f1); do
  echo "--- line $LN"; sed -n "${LN},$((LN+60))p" "$PE"
done

echo; echo "=== B. _check_skip_tool_parsing and _suppress_tool_calls ==="
sed -n '395,445p' "$PE"
echo "--- every _suppress_tool_calls in vllm/parser ---"
grep -rn "_suppress_tool_calls" "$V/parser" --include=*.py

echo; echo "=== C. _single_pass_parse ==="
LN="$(grep -n "def _single_pass_parse" "$PE" | head -1 | cut -d: -f1)"
echo "line: ${LN:-none}"; [ -n "$LN" ] && sed -n "${LN},$((LN+70))p" "$PE"

echo; echo "=== D. _feed ==="
sed -n '217,275p' "$PE"

echo; echo "=== E. serving.py: chat_completion_full_generator around parser.parse ==="
sed -n '860,935p' "$SV"

echo; echo "=== F. streaming_parser_engine.py: the tool-terminal special case ==="
sed -n '285,330p' "$SP"

echo; echo "=== G. how the parser object reaches serving.py (which class is it) ==="
grep -n "parser\s*=\|parser:\|get_parser\|ParserManager\|parser_manager" "$SV" | head -30
