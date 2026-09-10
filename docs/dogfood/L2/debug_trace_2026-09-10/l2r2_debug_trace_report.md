# T24 调试轨迹审计 — 实验机回报（2026-09-10，静态、只读）

协议：docs/dogfood/L2/DEBUG_TRACE_AUDIT.md（HEAD 3ad8a3a）。未运行模型，未改任何机制，未重跑。
与协议的出入：仅一处。指令中 Q2 检索命令带 `--regex`，协议不带；按协议形式执行，`'AssertionError|Lists differ'` 直接被接受（`0 hits` 为正常结果），未加 `--regex`。

## 0. 更新仓库、自检、补交

```
$ git pull --ff-only && git rev-parse HEAD
3ad8a3a3454315f5296af4a451df67a28dbe5de9
$ python docs/dogfood/L2/tools/test_debug_trace.py
..
----------------------------------------------------------------------
Ran 2 tests in 0.089s

OK
$ wc -l ~/Desktop/l2r2_logshow.txt
2819
```

补交项 (a) l2r2_diff.patch 自 `queuewatch/events.py` 块起的余部、(b) `pira_ctx history --scope workspace --limit 100` 尾部，见随附文件 `l2r2_step0_supplement.txt`（原样）。

### 脚本问题（原文，未修补脚本）

首次运行 `debug_trace.py` 重定向到文件时中断于第 5 次调用：

```
Traceback (most recent call last):
  File "C:\Users\LM\Desktop\Keel\docs\dogfood\L2\tools\debug_trace.py", line 167, in <module>
    main()
  File "C:\Users\LM\Desktop\Keel\docs\dogfood\L2\tools\debug_trace.py", line 150, in main
    print(f"    | {clip(line, args.width)}")
  File "C:\Users\LM\AppData\Local\Programs\Python\Python312\Lib\encodings\cp1252.py", line 19, in encode
    return codecs.charmap_encode(input,self.errors,encoding_table)[0]
UnicodeEncodeError: 'charmap' codec can't encode character '\u2192' in position 74: character maps to <undefined>
```

原因：Windows 上 stdout 重定向时 Python 默认 cp1252，观察文本含 `→`。绕过：设环境变量 `PYTHONUTF8=1` 后重跑，脚本未改。步骤 1 与步骤 4 的全部输出均在此环境下生成。

## 1. 时间线

`~/Desktop/l2r2_trace.txt`：100 次调用，899 行，55133 字节。全文随附（3 段）。summary 头：

```
summary
  model calls 100  tool calls 123  error results 34  empty responses 0
  repeated identical calls 5  pira_ctx retrievals 0  helper-related calls 24
  pira_ctx result IDs given to the model: 46; retrieved from later: 0
```

## 2. 循环界定

**起点：第 66 次。** `shell [exact] python -m unittest discover -s tests -v`（tool id `chatcmpl-tool-b4c66f718b8fe39e`），新增测试后首次跑全套，`error=true`。这是模型自写的 `--status` 输出测试首次失败的工具结果。观察前几行原文（logshow 第 2195 行起）：

```
tool_result chatcmpl-tool-b4c66f718b8fe39e (error=true): [stderr]
test_check_command_is_unaffected (test_cli.CliTests.test_check_command_is_unaffected) ... ok
test_check_counts_events_and_jobs (test_cli.CliTests.test_check_counts_events_and_jobs) ... ok
test_invalid_transition_is_a_data_error_naming_line_and_job (test_cli.CliTests.test_invalid_transition_is_a_data_error_naming_line_and_job) ... ok
test_malformed_line_is_a_data_error_with_its_number (test_cli.CliTests.test_malformed_line_is_a_data_error_with_its_number) ... ok
test_missing_file_is_a_data_error (test_cli.CliTests.test_missing_file_is_a_data_error) ... ok
```

同一观察随后（logshow 2202–2227 行）列出 5 个 FAIL：
`test_report_no_match_status_filter`、`test_report_status_filter_limits_rows`、`test_report_unknown_status_is_rejected`（test_cli）、
`test_status_filter_limits_rows_but_summary_counts_all`、`test_status_filter_none_is_unchanged`（test_report），
再给出 5 段完整回溯（2249–2355 行），末行 `Ran 51 tests in 0.023s` / `FAILED (failures=5)` / `[exit 1]`。整个结果 162 行，全部进入 SessionLog 的 tool_result。

其中目标测试的回溯原文（logshow 2270–2290 行）：

```
FAIL: test_report_status_filter_limits_rows (test_cli.CliTests.test_report_status_filter_limits_rows)
Traceback (most recent call last):
  File "C:\Users\LM\Desktop\queuewatch-l2r2\tests\test_cli.py", line 105, in test_report_status_filter_limits_rows
    self.assertEqual(
AssertionError: Lists differ: ['job[39 chars]led  1        0:01:04', 'total 1  queued 0  ru[31 chars], ''] != ['job[39 chars]led    1        0:01:04', 'total 3  queued 1  [33 chars], '']

First differing element 1:
'a    failed  1        0:01:04'
'a    failed    1        0:01:04'

  ['job  status  attempt  duration',
-  'a    failed  1        0:01:04',
+  'a    failed    1        0:01:04',
?                ++

-  'total 1  queued 0  running 0  completed 0  failed 1',
?         ^         ^                       ^

+  'total 3  queued 1  running 0  completed 1  failed 1',
?         ^         ^                       ^
```

第 66 次时该测试有两处不一致：列宽（`failed` 后两个空格 vs 四个）与汇总行计数。第 67 次修 cli.py 后汇总行一致，列宽不一致保留到结束。

**终点：第 100 次**（预算耗尽）。

**结束时该测试状态**：test_cli.py 中的期望 `'a    failed    1        0:01:04'` 自第 63 次写入后未再改动；实际输出 `'a    failed  1        0:01:04'`。第 66 次之后模型未再运行过 unittest（全套或单测均无）。

## 3. 三个问题

### Q1 正确诊断所需信息是否出现在模型可见的工具输出里？

循环内失败测试观察逐条：

| 调用 | 判定 | 观察形式 | 说明 |
|---|---|---|---|
| 66 | **yes** | 直接输出（`mode: exact`，`[stderr]` 头；非 pira_ctx 摘要，无结果 ID） | 5 个 FAIL 各含 `AssertionError: Lists differ` / `Tuples differ` 行、`First differing element`、期望行与实际行、`?  ++` / `^` 差异标记。目标测试的期望 `'a    failed    1        0:01:04'` 与实际 `'a    failed  1        0:01:04'` 均在观察内。 |

循环内没有第二次失败测试观察。作为补充（非测试失败，但同属反馈信息）：第 70、72 次 `[exact] python -c` 直接打印了 `render_report` 在各测试场景下的实际字符串（如 `'job  status  attempt  duration\na    queued  1        -\n…'`、`'d    failed  1        0:00:04'`），同为直接输出、无摘要。

决定性行不存在"只在保留捕获里而不在摘要里"的情形：第 66 次根本没有经过 pira_ctx 捕获。

### Q2 若不可见，是否保留在 pira_ctx 且返回可检索 ID？模型是否尝试检索？

前件不成立（Q1 为 yes、直接输出）。就事实记录：

- 全程给出结果 ID 46 个，事后检索 0 次；时间线无任何 `[retrieve]` 标记。
- 循环内（66–100）给出的 ID 共 12 个：第 68 次 1 个（135905-514ca5dae94d）、76 次 2 个、85 次 4 个、86 次 2 个、87 次 1 个、89 次 1 个、91 次 1 个。均未检索。
- 这 12 个捕获对应的是 `python -c` 一行程序报错、CLI 对数据文件的运行、辅助脚本的字节转储，没有一个是 unittest 运行；断言行不在任何保留捕获里。

直接核实（只读，在 `~/Desktop/queuewatch-l2r2` 内）：

```
$ pira_ctx search 20260910-140047-82c1322cb65f 'AssertionError|Lists differ' --context 3
0 hits
$ pira_ctx search 20260910-135905-514ca5dae94d 'AssertionError|Lists differ' --context 3
0 hits
```

对照（证明检索本身可用、捕获内容完整可取）：

```
$ pira_ctx search 20260910-140047-82c1322cb65f 'invalid choice' --context 3
1 hits
L3 stderr: queuewatch report: error: argument --status: invalid choice: 'bogus' (choose from queued, running, completed, failed)
L1 stderr: usage: queuewatch report [-h] [--status {queued,running,completed,failed}]
L2 stderr:                          path
$ pira_ctx search 20260910-135905-514ca5dae94d 'Traceback|SyntaxError' --context 3
2 lexical hits
L6 stderr: SyntaxError: invalid syntax
L1 stderr: Traceback (most recent call last):
L3 stderr:   File "<string>", line 1
L4 stderr:     try: main(list(["report", "tests/../../data/sample.jsonl", "--status", "bogus"])) except SystemExit: pass
L5 stderr:                                                                                       ^^^^^^
L2 stderr:   File "<string>", line 1, in <module>
```

### Q3 循环内每次调用用于什么？

分类规则的一处操作化（协议未明说，在此写明以便复核）：同一信息的**首次**获取尝试按其目的归 F（即便因引号问题失败）；此后仅改变 shell 引号 / 一行程序写法 / 辅助脚本机制的**重试**归 H。

计数（第 66–100 次，共 35 次）：

| 码 | 计数 |
|---|---|
| R | 3 |
| F | 8 |
| I | 1 |
| H | 22 |
| P | 0 |
| O | 1 |

代码串（66→100）：

```
FRFRFRFOHFFHHHHHHHHFFHHHIHHHHHHHHHH
```

逐次：

| 调用 | 码 | 内容 |
|---|---|---|
| 66 | F | 全套 unittest，5 失败（首次） |
| 67 | R | edit cli.py：把全部 jobs 传给 render_report，修汇总计数 |
| 68 | F | `python -c` 复现 CLI 输出，SyntaxError（引号） |
| 69 | R | edit cli.py：`--status` 加 `choices=` |
| 70 | F | `[exact] python -c` 打印 3 个 report 场景的实际字符串 |
| 71 | R | edit test_report.py 三个测试的期望 |
| 72 | F | `[exact] python -c` 打印 CLI 场景实际字符串（failed / running 无匹配 / 全部） |
| 73 | O | `type cli.py` + `type report.py`，重读自己在 67/69 刚改过的文件（有介入改动，故不计 I） |
| 74 | H | 重试 68 的一行程序，仍 SyntaxError（引号） |
| 75 | F | 对 `data/sample.jsonl --status failed` 跑 CLI，文件不存在 |
| 76 | F | `dir data\` 找数据文件 + 对 `data/events.jsonl --status failed` 跑 CLI（成功） |
| 77 | H | 再次重试一行程序，SyntaxError（引号） |
| 78 | H | `exec(open('tests/test_report.py').read().split(...))` 借测试文件的辅助函数，无输出 |
| 79 | H | 同上 exec 技巧 + 打印，ValueError `second must be in 0..59`（辅助数据 `finished=64`） |
| 80 | H | 再次一行程序，SyntaxError（引号） |
| 81 | H | 创建 `tests/_check.py` |
| 82 | H | 以绝对路径运行 `_check.py`，ModuleNotFoundError |
| 83 | H | 以相对路径运行 `_check.py`，ModuleNotFoundError |
| 84 | H | edit `_check.py` 被拒（缺 safety_review） |
| 85 | F | 对生产数据跑 CLI `--status completed/queued/running/bogus` 四次，输出符合预期 |
| 86 | F | 对 `tests/../../data/sample.jsonl` 跑 CLI 两次，文件不存在 |
| 87 | H | `python -c` 读写 `_check2.py`，`io.UnsupportedOperation` |
| 88 | H | edit `_check.py` 失败：CRLF 与 old_text 不匹配 |
| 89 | H | `python -c` 转储 `_check.py` 原始字节（查 CRLF） |
| 90 | I | 原样重跑 83 的 `python tests\_check.py`，其间 `_check.py` 未变（88 失败、89 只读），同样 ModuleNotFoundError |
| 91 | H | `python -m queuewatch report tests\_check.py`，把辅助脚本当日志文件跑，`not valid JSON` |
| 92 | H | `cmd /C python -c "..."` 一行程序，SyntaxError（引号） |
| 93 | H | edit `_check.py` 失败：CRLF |
| 94 | H | `python -c` 把 `_check.py` 清空 |
| 95 | H | edit `_check.py` 被拒：old_text 为空 |
| 96 | H | `python -c` 写入 `# check\n` 作占位 |
| 97 | H | edit `_check.py`：把占位替换为 1319B 脚本 |
| 98 | H | 运行 `_check.py`，ValueError `second must be in 0..59`（`finished=64`） |
| 99 | H | edit `_check.py`（1319B→1319B，shebang 行） |
| 100 | H | edit `_check.py`：把 64/60 秒改成 59/58 |

事实补记（不作解读）：循环内 3 次 R 中，67、69 改业务代码，71 改 test_report.py 期望；test_cli.py 的期望在循环内没有任何一次改动。P 为 0：`read_pira_policy` 的 6 次全部在第 66 次之前（3、5、27、36、62）。

时间线 summary 的 `helper-related calls 24` 是全程按工具调用计的（含 23–29 的 `tests_output.txt` 与 56–57 的 `_write_cli.py`），与本节循环内按模型调用计的 H=22 口径不同。

## 4. L2 与 L2-R1 存在性检查（仅脚本）

```
$ python docs/dogfood/L2/tools/debug_trace.py "C:\Users\LM\.keel\sessions\5fadfcb1fe58a369\a016c8b78df4b6dd.jsonl" | tail -20
（尾 20 行为结果 ID 列表 …given at call 50 … 75 retrieved at never；summary 头如下）
summary
  model calls 76  tool calls 86  error results 21  empty responses 0
  repeated identical calls 2  pira_ctx retrievals 0  helper-related calls 0
  pira_ctx result IDs given to the model: 50; retrieved from later: 0

$ python docs/dogfood/L2/tools/debug_trace.py "C:\Users\LM\.keel\sessions\369f5851b4effc46\07de4bc9b279d6e5.jsonl" | tail -20
（尾 20 行为结果 ID 列表 …given at call 1 … 37 retrieved at never；summary 头如下）
summary
  model calls 40  tool calls 55  error results 8  empty responses 1
  repeated identical calls 0  pira_ctx retrievals 0  helper-related calls 0
  pira_ctx result IDs given to the model: 20; retrieved from later: 0
```

同一 `--status` 表格格式测试的修复区间（快速阅读时间线，仅起止）：

- **L2：42–63。** 42 首次全套失败；46 单跑 `test_report_status_filter_queued`；52、56、60 三次改 test_cli.py 期望；63 单测通过，64 全套通过。
- **L2-R1：25–40。** 25 全套失败（含列宽断言）；27–30 试图打印实际列宽（引号失败 → 写 chk.py）；31、32 改期望；33–39 继续为 chk.py 折腾引号；40 空回复，运行结束。该测试在 L2-R1 结束时未被再次运行，修复未确认。

## 5. 保留在本机的材料

wire 文件、完整正文、`~/.keel/sessions/19b4f12b539d766d/*`、`queuewatch-l2r2` 工作区与其 pira_ctx 存储。本文引用的观察原文限于回答所需。
