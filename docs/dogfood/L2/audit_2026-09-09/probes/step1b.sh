#!/usr/bin/env bash
# Step 1b — serving-host record, adapted to the vLLM 0.26.0 layout.
# Read-only: no file is written, no configuration changed, no inference issued.
# Run inside the serving container:
#   ssh <host> "docker exec -i <container> bash -s" < step1b.sh
set -u

echo "########## 1b.1  installed vLLM: version and path ##########"
python3 -c "import vllm, os, sys; print('vllm.__version__ =', vllm.__version__); print('vllm.__file__    =', vllm.__file__); print('package dir      =', os.path.dirname(vllm.__file__)); print('python           =', sys.version.replace(chr(10),' '))"
V="$(python3 -c 'import vllm, os; print(os.path.dirname(vllm.__file__))')"
echo "distribution metadata:"
python3 -c "import importlib.metadata as m; d=m.distribution('vllm'); print('  name/version:', d.metadata['Name'], d.version)" 2>&1
echo "torch / transformers:"
python3 -c "import torch, transformers; print('  torch', torch.__version__, '| transformers', transformers.__version__)" 2>&1

echo
echo "########## 1b.2  server launch arguments ##########"
echo "--- PID 1 cmdline (argv, NUL-separated -> one per line)"
tr '\0' '\n' < /proc/1/cmdline
echo "--- any other vllm process"
for p in /proc/[0-9]*; do
  c="$(tr '\0' ' ' < "$p/cmdline" 2>/dev/null)"
  case "$c" in *vllm*) echo "  ${p#/proc/}: $c";; esac
done | head -20
echo "--- VLLM_* environment of PID 1"
tr '\0' '\n' < /proc/1/environ 2>/dev/null | grep -E '^(VLLM|HF|CUDA|TORCH)' | grep -v -E 'TOKEN|KEY|SECRET|PASS' | sort

echo
echo "########## 1b.3  parser modules present and selected ##########"
echo "--- vllm/parser/"
ls -1 "$V/parser" 2>/dev/null
echo "--- vllm/parser/engine/"
ls -1 "$V/parser/engine" 2>/dev/null
echo "--- vllm/tool_parsers/  (registry entries mentioning qwen)"
grep -n "qwen" "$V/tool_parsers/__init__.py" 2>/dev/null
echo "--- vllm/reasoning/ (if present)"
ls -1 "$V/reasoning" 2>/dev/null | head -20
echo "--- parser selected by the launch arguments above"
tr '\0' '\n' < /proc/1/cmdline | grep -A1 -E 'parser|reasoning' || echo "  (no explicit --tool-call-parser / --reasoning-parser in argv; parser chosen by model default)"

echo
echo "########## 1b.4  weights: chat template and generation config ##########"
MODEL="$(tr '\0' '\n' < /proc/1/cmdline | grep -A1 -x -- '--model' | tail -1)"
[ -n "${MODEL:-}" ] || MODEL="$(tr '\0' '\n' < /proc/1/cmdline | grep -m1 -- '--model=' | cut -d= -f2-)"
echo "model argument: ${MODEL:-unknown}"
CANDS=""
[ -n "${MODEL:-}" ] && [ -d "$MODEL" ] && CANDS="$MODEL"
for d in /root/.cache/huggingface/hub/models--*/snapshots/* \
         "${HF_HOME:-/nonexistent}"/hub/models--*/snapshots/* \
         /models/* /weights/*; do
  [ -d "$d" ] && CANDS="$CANDS $d"
done
for d in $CANDS; do
  hit=0
  for f in chat_template.json chat_template.jinja chat_template.jinja2 \
           generation_config.json config.json tokenizer_config.json; do
    if [ -f "$d/$f" ]; then
      hit=1
      printf '%s  %s\n' "$(sha256sum "$d/$f" | cut -d' ' -f1)" "$d/$f"
    fi
  done
  [ "$hit" = 1 ] && echo "  ---"
done
echo "--- generation_config.json contents (small, safe to quote in evidence)"
for d in $CANDS; do
  [ -f "$d/generation_config.json" ] && { echo "  from $d"; cat "$d/generation_config.json"; break; }
done
echo "--- does the chat template open <think> for the assistant turn?"
for d in $CANDS; do
  for f in chat_template.jinja chat_template.json tokenizer_config.json; do
    [ -f "$d/$f" ] || continue
    n="$(grep -c "<think>" "$d/$f" 2>/dev/null || echo 0)"
    echo "  $d/$f : '<think>' occurrences = $n"
  done
  break
done

echo
echo "########## 4.  exact line ranges for step-3 appendix B ##########"
PE="$V/parser/engine/parser_engine.py"
SP="$V/parser/engine/streaming_parser_engine.py"
Q="$V/parser/qwen3.py"
SV="$V/entrypoints/openai/chat_completion/serving.py"
echo "--- file lengths"
wc -l "$PE" "$SP" "$Q" "$SV"
echo "--- def boundaries in parser_engine.py (name -> first line)"
grep -n "^    def \|^    @" "$PE" | sed -n '1,400p' | grep -E "def (parse|_single_pass_parse|_feed|_check_skip_tool_parsing|_events_to_delta|_ensure_slot|_build_extracted_result|extract_reasoning|extract_tool_calls_from_content)\b"
echo "--- def boundaries in streaming_parser_engine.py"
grep -n "^    def " "$SP" | grep -E "def (_on_terminal|_emit_for_state|_on_content|_apply_transition|feed|finish)\b"
echo "--- the REASONING -> TOOL_PREAMBLE transition in qwen3.py"
grep -n -B2 -A4 'ParserState.REASONING, "TOOL_START"' "$Q"
echo "--- terminals / token_id_terminals / transitions block starts in qwen3.py"
grep -n "terminals=\|token_id_terminals=\|transitions=\|initial_state=" "$Q"
echo "--- parser.parse call site in serving.py"
grep -n -B3 -A6 "parser.parse(" "$SV"
echo
echo "########## end of step 1b ##########"
