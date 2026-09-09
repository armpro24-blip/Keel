#!/usr/bin/env bash
# Step 1b follow-up: read ONLY the served model's snapshot (positional arg of
# `vllm serve`), which the first probe missed. Read-only.
set -u
MODEL="$(tr '\0' '\n' < /proc/1/cmdline | sed -n '/^serve$/{n;p;q}')"
echo "served model (positional argv after 'serve'): ${MODEL:-unknown}"
S="/root/.cache/huggingface/hub/models--${MODEL//\//--}"
echo "hub dir: $S"; [ -d "$S" ] || { echo "not found"; exit 0; }

echo; echo "--- refs (which snapshot is current)"
for r in "$S"/refs/*; do printf '%s -> %s\n' "$(basename "$r")" "$(cat "$r")"; done
echo "--- all snapshots present"
ls -1 "$S/snapshots"

SNAP="$S/snapshots/$(cat "$S/refs/main" 2>/dev/null)"
[ -d "$SNAP" ] || SNAP="$(ls -d "$S"/snapshots/* | head -1)"
echo "using: $SNAP"

echo; echo "--- sha256 (served snapshot)"
for f in chat_template.jinja chat_template.json generation_config.json config.json tokenizer_config.json; do
  [ -f "$SNAP/$f" ] && printf '%s  %s\n' "$(sha256sum "$SNAP/$f" | cut -d' ' -f1)" "$f"
done

echo; echo "--- generation_config.json"
cat "$SNAP/generation_config.json" 2>/dev/null || echo "(absent)"

echo; echo "--- chat_template.jinja: size, and every line mentioning think / tool_call / function / parameter (cut to 160 cols)"
wc -c "$SNAP/chat_template.jinja"
grep -n -i -E "think|tool_call|<function|<parameter|enable_thinking" "$SNAP/chat_template.jinja" | cut -c1-160

echo; echo "--- chat_template.jinja: the generation-prompt tail (last 25 lines)"
tail -n 25 "$SNAP/chat_template.jinja" | cut -c1-160

echo; echo "--- tokenizer_config.json: special/added tokens relevant to the parser"
python3 - "$SNAP/tokenizer_config.json" <<'PY'
import json, sys, re
cfg = json.load(open(sys.argv[1]))
print("chat_template embedded in tokenizer_config:", "chat_template" in cfg)
for k in ("eos_token", "pad_token", "bos_token"):
    print(f"{k}: {cfg.get(k)!r}")
atd = cfg.get("added_tokens_decoder", {})
pat = re.compile(r"think|tool_call|tool_response|function|parameter|im_start|im_end", re.I)
rows = [(int(i), v.get("content"), v.get("special")) for i, v in atd.items() if pat.search(str(v.get("content")))]
print(f"added_tokens_decoder: {len(atd)} total; relevant:")
for tid, content, special in sorted(rows):
    print(f"  {tid:>7}  special={special!s:<5}  {content!r}")
PY

echo; echo "--- config.json: architecture / eos"
python3 -c "import json,sys; c=json.load(open(sys.argv[1])); print({k:c.get(k) for k in ('architectures','model_type','eos_token_id','torch_dtype','quantization_config') if k in c})" "$SNAP/config.json" 2>&1 | cut -c1-400
