# Gate D raw results: exact-replace `edit_file` (lab, 2026-09-08)

Keel `a76ee4d` (no Keel code involved; Keel has no `edit_file` tool). vLLM
0.26.0, `nvidia/Qwen3.6-35B-A3B-NVFP4`. Script
`docs/dogfood/edit_file/gate_d.py`, 10 runs, 14:06:24Z to 14:07:55Z. Frozen
edit: `docs/dogfood/edit_file/frozen/` (old_text 681 bytes / 21 lines,
new_text 1,074 bytes / 35 lines, expected file 1,827 bytes), verified by the
script before the run. Raw responses stay on the lab machine
(`~/Desktop/gate_d/`). Reported by the user; `scores.txt` verbatim except
where the lab corrected its own transcription (noted).

## Pre-registered scores (`scores.txt`)

```text
metric                               score
valid_edit_file_call                 10/10
path_exact                           10/10
old_text_byte_exact                  10/10
new_text_byte_exact                  10/10
old_text_unique_in_seed              10/10
replacement_produces_expected_file   10/10  (gate metric)
looks_like_l1_collapse               0/10
```

| run | finish | prompt | completion | reasoning chars | visible content | old_text bytes | new_text bytes | occurrences | expected file |
|---|---|---|---|---|---|---|---|---|---|
| 01 | tool_calls | 5792 | 570 | 387 | (empty) | 681 | 1074 | 1 | yes |
| 02 | tool_calls | 5792 | 1058 | 2301 | (empty) | 681 | 1074 | 1 | yes |
| 03 | tool_calls | 5792 | 2094 | 6461 | (empty) | 681 | 1074 | 1 | yes |
| 04 | tool_calls | 5792 | 1020 | 2156 | (empty) | 681 | 1074 | 1 | yes |
| 05 | tool_calls | 5792 | 1984 | 5923 | (empty) | 681 | 1074 | 1 | yes |
| 06 | tool_calls | 5792 | 561 | 285 | (empty) | 681 | 1074 | 1 | yes |
| 07 | tool_calls | 5792 | 517 | 142 | (empty) | 681 | 1074 | 1 | yes |
| 08 | tool_calls | 5792 | 531 | 215 | (empty) | 681 | 1074 | 1 | yes |
| 09 | tool_calls | 5792 | 650 | 636 | (empty) | 681 | 1074 | 1 | yes |
| 10 | tool_calls | 5792 | 1328 | 3162 | (empty) | 681 | 1074 | 1 | yes |

Row records as reported:

```text
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 6362, "completion_tokens": 570, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 387, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 6850, "completion_tokens": 1058, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 2301, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 7886, "completion_tokens": 2094, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 6461, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 6812, "completion_tokens": 1020, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 2156, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 7776, "completion_tokens": 1984, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 5923, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 6353, "completion_tokens": 561, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 285, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 6309, "completion_tokens": 517, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 142, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 6323, "completion_tokens": 531, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 215, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 6442, "completion_tokens": 650, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 636, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
{"valid_edit_file_call": true, "path_exact": true, "old_text_byte_exact": true, "new_text_byte_exact": true, "old_text_unique_in_seed": true, "replacement_produces_expected_file": true, "looks_like_l1_collapse": false, "n_tool_calls": 1, "note": "", "finish": "tool_calls", "usage": {"prompt_tokens": 5792, "total_tokens": 7120, "completion_tokens": 1328, "prompt_tokens_details": null}, "content_text": "", "reasoning_chars": 3162, "old_text_occurrences": 1, "old_text_bytes": 681, "new_text_bytes": 1074}
```

(Row 10's `usage` was mistyped in the lab's message body and corrected by the
lab to `total_tokens 7120, completion_tokens 1328`, as in the file.)

Lab observations: all ten single-call, `finish = tool_calls`, empty visible
content (no `Safety:` line, unlike 5/10 in Gate C); `old_text` and
`new_text` byte-exact every time, `old_text` unique in the seed every time;
completion 517 to 2,094 tokens and reasoning 142 to 6,461 characters, wide
variation with no effect on correctness.

## Verdict under the pre-registered rule

`replacement_produces_expected_file` = **10/10**. Threshold `9–10/10 →
exact-replace editing is a credible candidate → write a minimal design for
review; do not implement yet`. The design is `docs/DESIGN_EDIT_FILE.md`.
Nothing is implemented.

## Cross-gate comparison on the L1 `cli.py` change

| channel | structural validity | intended file produced (no normalization) |
|---|---|---|
| L1: body inside an argv element (Windows quoting) | 8/42 calls collapsed | 0 |
| Gate B: model-authored writer program on stdin (whole file) | 10/10 | 4/10 |
| Gate C: whole file as a top-level `content` string | 10/10 | 6/10 (4 differ only by the final newline) |
| Gate D: exact replace `old_text` → `new_text` (21 → 35 lines) | 10/10 | 10/10 |

Limitation retained: Gate D shows that the tool-call channel carries an
exact-replace edit of this size faithfully when the model is handed the
exact old and new blocks. It does not show that a model performing the L1
task will reproduce existing text exactly from what it read, choose unique
blocks, or handle zero- and multi-match errors well. Those questions belong
to the regression run after implementation (L1-R1), which stays a
single-variable comparison.
