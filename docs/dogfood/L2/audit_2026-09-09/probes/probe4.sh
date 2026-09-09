#!/usr/bin/env bash
# Round 4 (decisive): one Qwen3Parser state machine drives both the reasoning
# and the tool adapter. Read its transitions, especially what happens when the
# closing think tag never arrives. Read-only.
set -u
V="$(python3 -c 'import vllm, os; print(os.path.dirname(vllm.__file__))')"
echo "vllm root: $V"

echo; echo "=== A. vllm/parser/qwen3.py (the state machine) ==="
wc -l "$V/parser/qwen3.py"
sed -n '1,260p' "$V/parser/qwen3.py"

echo; echo "=== A2. rest of qwen3.py if longer than 260 lines ==="
sed -n '261,520p' "$V/parser/qwen3.py"

echo; echo "=== B. how one parser becomes a reasoning adapter and a tool adapter ==="
wc -l "$V/parser/engine/adapters.py"
sed -n '1,200p' "$V/parser/engine/adapters.py"

echo; echo "=== C. event kinds the engine emits ==="
grep -n "class \|REASONING\|CONTENT\|TOOL\|reasoning\|content\|tool" "$V/parser/engine/events.py" | head -60

echo; echo "=== D. abstract parser: the contract ==="
grep -n "class \|def \|think\|reasoning\|tool" "$V/parser/abstract_parser.py" | head -60
