# T25 阶段 2：shell 描述 A/B，80 次运行 — 实验机回报（2026-09-10）

协议：docs/dogfood/shell_contract/SHELL_CONTRACT_AB.md（HEAD fe9229e）。一次运行，80 次，全部完成。不做门槛判定、不做解读。

## 与指令 / 协议的出入（先列）

1. **步骤 0 `git diff main --stat`**：指令写"仅 src/shell.rs"，实测 4 个文件（`SHELL_CONTRACT_AB.md`、`score_shell_ab.py`、`test_score_shell_ab.py`、`src/shell.rs`）。原因：分支 `exp/shell-contract-b` 从 `cbe1923` 分出、自身只改 `src/shell.rs`；`main` 之后多了 `fe9229e`（评分脚本修正，仅 docs/tools）。进入二进制的差异只有 `src/shell.rs`（+16/−7，与协议冻结的 B 文本一致）。评分使用 `main` 上 `fe9229e` 的脚本。已回报，用户未叫停，按此继续。
2. **任务文本**：指令中的文本有粘贴粘连（`summary.txtwhose`、`idis 3`、`itsname`、D1 尾部多余的反引号）。以协议表格为准，四行从表格程序化抽取（含反引号），见 §2。
3. **评分脚本观察（未修补）**：`helper_paths` 把三次运行里种子已跟踪的文件列为 helper：R1-B-7 的 `tests/test_*.py` 五个、R2-A-3 与 R2-B-6 的 `queuewatch/report.py`。列出的路径缺盘符（`/Users/LM/Desktop/t25/...`，无 `C:`），像是路径归一化后与仓库跟踪表未匹配上。这三次贡献了 B 的 helper_calls 8 中的 7、A 的 2 中的 1。原样回报供作者判断，CSV 未改。
4. 运行器（实验机侧的 REPL 驾驭程序）在首个提示符处检查横幅 `[max_turns] 12 per user message` 后才发送任务，任务发送后的下一个提示符即 `/quit`，`approve?` 答 `n`。80 次均检测到横幅；无 `approve?` 提示；无静默超时。

## 0. 构建两份二进制

```
$ git pull --ff-only && git rev-parse HEAD
fe9229e88030b075a3f66f9c0510886b0ae977a2
$ git log -1 --format=%h -- src Cargo.toml Cargo.lock
822fdbc
$ cargo build -q && cp target/debug/keel.exe ~/Desktop/keel-A.exe
(ok; 5790208 bytes)
$ git fetch origin exp/shell-contract-b && git checkout exp/shell-contract-b && git rev-parse HEAD
4f71fe7ef6373833d6129019a4e5852c743c75e7
$ git diff main --stat
 docs/dogfood/shell_contract/SHELL_CONTRACT_AB.md   |  4 +-
 .../dogfood/shell_contract/tools/score_shell_ab.py | 63 +++++-----------------
 .../shell_contract/tools/test_score_shell_ab.py    | 25 +++------
 src/shell.rs                                       | 23 +++++---
 4 files changed, 36 insertions(+), 79 deletions(-)
$ git diff main exp/shell-contract-b --stat -- src Cargo.toml Cargo.lock
 src/shell.rs | 23 ++++++++++++++++-------
 1 file changed, 16 insertions(+), 7 deletions(-)
$ git merge-base main exp/shell-contract-b
cbe1923
$ git log --oneline exp/shell-contract-b..main
fe9229e T25 scoring: classification rule fixed and aligned with the protocol; self-check green (11 tests)
$ cargo build -q && cargo test 2>&1 | grep -c "test result: ok"
16
$ cp target/debug/keel.exe ~/Desktop/keel-B.exe && git checkout main
back on: main fe9229e
$ grep -c "argv\[0\] is the program" ~/Desktop/keel-A.exe ~/Desktop/keel-B.exe
/c/Users/LM/Desktop/keel-A.exe:0
/c/Users/LM/Desktop/keel-B.exe:1
$ python docs/dogfood/shell_contract/tools/test_score_shell_ab.py
Ran 11 tests in 0.224s
OK
```

## 1. 服务与 PIRA 预检

```
$ git -C ~/agent rev-parse HEAD
4e0682dd745f1dbafa772d9c11b369132db4c1a8      (AGENTS.md sha256 e6c7d63046d42f14…)
$ curl -s http://192.168.3.103:8000/version
{"version":"0.26.0"}
$ curl -s -H "Authorization: Bearer dummy" http://192.168.3.103:8000/v1/models
id nvidia/Qwen3.6-35B-A3B-NVFP4  root nvidia/Qwen3.6-35B-A3B-NVFP4  max_model_len 262144
```

服务主机（`step1b_fix.sh` + argv 读取，输出 `t25_serving.txt` 138 行随附）：snapshot `main -> 1355db6a052410cfd62085d94b58866fd0f2c3c5`；`chat_template.jinja e84f32a2…4259`；`generation_config.json e70c136c…550e`；argv 含 `--reasoning-parser qwen3 --tool-call-parser qwen3_xml --enable-auto-tool-choice`，无 `--chat-template`；`VLLM_IMAGE_TAG=vllm/vllm-openai:v0.26.0`，`VLLM_BUILD_COMMIT=ffd46bfab2128bb84146050e98b51a617c6575ab`。无 token 出现。启动前 `vllm:num_requests_running 0 / waiting 0`。

## 2. 任务文本（从协议表格抽取，单行发送；sha256 为 line+LF 前 16 位）

```
R1  chars=136  c3e3070dc6ef2b4f
Count the test methods (functions whose name starts with `test_`) defined under `tests/` in this repository. Reply with the number only.
R2  chars=123  852c211abc6479f1
Compute the SHA-256 of the file `queuewatch/report.py` exactly as stored on disk. Reply with the lowercase hex digest only.
D1  chars=182  da5e1c05b77d80b6
Create the file `notes/summary.txt` whose entire content is the following line followed by one LF newline, byte for byte, UTF-8 without BOM: `She said "it's done" -- path C:\tmp\x y`
D2  chars=124  f9b479df7384f639
In `data/quotes.jsonl`, find the record whose `id` is 3 and reply with the exact value of its `name` field and nothing else.
```

## 3. 运行

- 顺序：R1、R2、D1、D2；每任务 10 对，奇数对 A→B，偶数对 B→A。逐次顺序表见 `run_order.txt`（随附，含每次的开始/发送/结束 UTC、wire 行数、结束列、Keel 错误行）。
- 每次：`init_l2.sh ~/Desktop/t25/<task>-<arm>-<n>`（80 次均 `commit 5d668de…`，42 tests OK）；D2 复制 `quotes.jsonl`；`keel-<arm>.exe --model "nvidia/Qwen3.6-35B-A3B-NVFP4" --trace --record-wire --full --max-turns 12`。
- 时间：首次 init 2026-09-10 20:21:29Z，末次结束 20:49:33Z，**共 28 分 04 秒**。
- **总模型调用 382**（A 180，B 202）；`vllm:time_to_first_token_seconds_count` 增量 = 382，与 CSV 一致。总工具调用 317。
- 人工干预：每次仅任务行与 `/quit`。**无模型提问**（80 个会话末段均无问句；driver 日志无 approve 提示）。Continue 0。
- Keel 错误：仅 D1-A-9 `error: model-call budget exhausted (max_turns = 12); the run is incomplete; the transcript is kept`。其余 79 次 `end=completion`。
- 评分脚本 80 次 rc=0；full_mode_check 80 次：`approve? prompts 0`、`ordering violations 0`（`handshake.txt` 随附）。

## 4. 结果（results.csv 原样随附；以下为按列的机械汇总，不含判定）

| | A | B |
|---|---|---|
| 正确运行 / 40 | 34 | 39 |
| 含 confirmed arg_error 的运行数（事件数） | 5 (6) | 9 (12) |
| prog_errors 事件 | 2 | 3 |
| undetermined | 0 | 0 |
| helper_calls（含 §出入 3 的可疑计入） | 2 | 8 |
| end ≠ completion | 1 (D1-A-9 budget) | 0 |
| 模型调用 | 180 | 202 |

按任务：

| 任务 | A 正确 | B 正确 | A arg_err 运行 | B arg_err 运行 | A helper | B helper | A 调用 | B 调用 |
|---|---|---|---|---|---|---|---|---|
| R1 | 10/10 | 9/10 | 1 | 2 | 1 | 4 | 57 | 67 |
| R2 | 9/10 | 10/10 | 0 | 0 | 1 | 3 | 22 | 23 |
| D1 | 7/10 | 10/10 | 1 | 3 | 0 | 1 | 64 | 77 |
| D2 | 8/10 | 10/10 | 3 | 4 | 0 | 0 | 37 | 35 |

D1 附加列：A `file_correct` 7、`line_ending` 1（D1-A-5，44 字节，CRLF/BOM 类）；B `file_correct` 10、`line_ending` 0。

非正确运行（7 次：A 6、B 1）与最终文本（截 150 字符）：

```
R1-B-1  completion | 8
R2-A-2  completion | The SHA-256 digest is:\n\n```\n25ba9483d9cf0c91b81cd8d6576d2df3f04eff7340157269172891b2d51c1a44\n```
D1-A-3  completion | file 39 bytes (no trailing LF) | Created `notes/summary.txt` (39 bytes). The content is: …
D1-A-5  completion | line_ending=True (44 bytes) | Created `notes/summary.txt`. Hex dump confirms exact content — 44 bytes, UTF-8 without BOM, ending with a single LF …
D1-A-9  budget     | (no final text)
D2-A-1  completion | The name field for id 3 is: `She said "it's done" -- path C:\tmp\x y`
D2-A-7  completion | `She said "it's done" -- path C:\tmp\x y`
```


`notes.txt` 中评分脚本给出的 arg_error 明细：18 条，**全部为结构性拒绝 `input needs an array field 'argv'`**；无 output-evidence 类（`"import` / `can't open file`）。分布：A — R1-A-9 ×2、D1-A-9、D2-A-1、D2-A-5、D2-A-9；B — R1-B-5、R1-B-8、D1-B-2、D1-B-5、D1-B-8 ×2、D2-B-3 ×2、D2-B-4、D2-B-6 ×2、D2-B-7。

`helper_paths` 列出的路径（供人工复核）：R1-A-1 `f.py`；R1-B-7 `tests/test_cli.py … test_timeutil.py`（种子已跟踪，见出入 3）；R2-A-3 `queuewatch/report.py`（已跟踪）；R2-B-6 `queuewatch/report.py; queuewatch//report.py`（已跟踪）；D1-B-8 `notes/create.py`。

## 5. 随附文件（原样）

`results.csv`（81 行）、`notes.txt`（98 行）、`handshake.txt`（560 行）、`run_order.txt`（81 行）、`t25_serving.txt`（138 行）。80 个仓库 `~/Desktop/t25/<task>-<arm>-<n>` 保持模型留下的状态；80 份 wire 与 SessionLog 留本机（路径在 `run_order.txt` 的 `log` 列）。
