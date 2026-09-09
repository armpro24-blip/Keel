# 工具协议审计 — 步骤 3 结论（实验机回报）

日期：2026-09-09
执行者：实验机操作员
范围：仅步骤 3（已安装 vLLM 源码阅读）+ 本机 wire 结构化复核。
步骤 1a 由协议规定另行完成，本文不代为填写。步骤 1b 的实测记录见随附
`step1b_output.txt`，本文附录 B 的版本、路径与行号均据此回填。

---

## 0. 结论先行

四次空回复归类为：**模型输出不合规**。

- 配置不匹配：排除。
- 解析器实现缺陷：排除。

---

## 1. 两个预注册问题的答案

**问题一：`extract_tool_calls` 的输入是完整生成文本，还是已剥离思考段的 content？**

完整生成文本。

非流式链路为 `chat_completion_full_generator` → `parser.parse(output.text, request,
enable_auto_tools=..., model_output_token_ids=token_ids)` →
`ParserEngine.parse` → `_check_skip_tool_parsing(request)` →
`_single_pass_parse(model_output, token_ids)`。`_single_pass_parse` 的
`initial_state` 参数未传，`_reset()` 采用配置初始态；thinking 开启时该初始态
为 `ParserState.REASONING`。整段生成文本单次扫描。

`extract_tool_calls_from_content`（起始态 `CONTENT`、接受剥离后内容）确实存在，
但服务于 `ParserEngineToolAdapter`，不在本链路上。

**问题二：开 `<tool_call>` 标签是否会被 reasoning 解析阶段消费？**

否。

`skip_tool_parsing` 仅由 `adapters.py::_skip_tool_parsing()` 上下文管理器置位，
该上下文只包裹 reasoning 适配器的三个方法；本链路上恒为 False。
另一抑制位 `_suppress_tool_calls` 的唯一置位条件是
`tool_choice == "none" and tools`（parser_engine.py:410），Keel 不发送该值。

因此 `(REASONING, TOOL_START) → TOOL_PREAMBLE`（qwen3.py 中注释为
"Tool call directly from reasoning (implicit end)"）在本链路上可达且生效。

---

## 2. 为什么排除解析器缺陷

三条互相独立的源码论证。

**2.1 未匹配的标记文本必被保留**

`streaming_parser_engine.py::_on_terminal` 的三条 no-transition 出路
（`transition is None`、`skip_tool_parsing` 分支、
`transition.skip_in_token_id_mode and self._ever_had_token_ids` 分支）
全部终结于 `_emit_for_state(value)`，把标记原文按当前状态发回事件流。
唯一无声丢弃的是 `DROP_TERMINAL`，而它要求 `transition is None`
且终结符名恰为 DROP。

推论：某个标记的文本在 reasoning 中不存在，只可能是它从未出现在
`output.text` 中；不可能是"被消费后丢失"。

补一条来自权重目录的旁证（步骤 1b 补充记录）：`<tool_call>`（id 248058）、
`</tool_call>`（248059）、`<think>`（248068）、`</think>`（248069）在
`tokenizer_config.json` 的 `added_tokens_decoder` 中均为 `special=False`，
detokenize 时不会被 `skip_special_tokens` 剥除，必以文本形式进入 `output.text`。
"开标签被 detokenizer 吞掉"因此也可排除。

**2.2 工具槽位只由 TOOL_CALL_START 创建**

`_events_to_delta` 中 `case EventType.TOOL_CALL_START: ... self._ensure_slot(...)`；
`_build_extracted_result` 跳过 `not slot.name and not slot.args` 的槽位。
四次响应返回 `tool_calls: 0` 且 `content` 为空，说明没有任何槽位取得
名字或参数，即 `TOOL_CALL_START` 从未发出，即 `(REASONING, TOOL_START)`
从未匹配。

**2.3 反事实检验**

若 `<function=NAME><parameter=KEY>` 曾被迁移消费，则其后的文本会以
`ARG_VALUE_CHUNK` 进入 `slot.args`，`slot.name` 亦会被填充，
`tools_called` 应为 True。实测为 False，且该段文本出现在 `reasoning`
而非 `content`，证明状态机自始至终停留在 `REASONING`。

---

## 3. 模型实际发出了什么

四次失败的 reasoning 尾部保留了一个工具调用块的**后缀**，块首连续三个标记
（`<tool_call>`、`<function=NAME>`、首个 `<parameter=KEY>`）缺失。

以 L1 第 17 次为例，reasoning 全长 763 字符，标记骨架为：

```
[458 chars] </parameter> [1] <parameter=intent> [66] </parameter> [1]
<parameter=effect> [16] </parameter> [1] <parameter=safety_review> [86]
</parameter> [1] </function> [1] </tool_call>
```

开头 458 字符是模型的思考散文（内容为"shell 工具似乎拒绝较长命令或含复杂引号
的命令，换个办法"一类的推敲），不是任何参数的值。模型在想清楚之后直接接上了
一个无头的调用尾巴：先闭掉一个从未打开的 `</parameter>`，再补三个参数，
最后 `</function></tool_call>`。

**对照组：L1 第 35 次（成功）。** 同会话、同模型、同解析器，
`finish_reason: tool_calls`，`tool_calls: 1`。其 reasoning 尾部为
`…approach.\n</parameter>\n</thinking>\n\nLet me try piping Python code
into the interpreter:\n\n`。同样带有游离闭标签，且 `</thinking>` 这个标签
在 Qwen3 终结符表中根本不存在（表中为 `<think>`/`</think>`），系模型臆造。
但它随后写出了完整的 `<tool_call>` 块，因而被正常消费、正常提取，
其开标签也因被迁移消费而不出现在 reasoning 中。

同一种标签错乱，写全了就成功，写漏了块首就失败。这直接证明失败方在模型侧。

---

## 4. 失败签名与分布

签名：`finish_reason: "stop"` + `tool_calls: 0` + `content` 为空
+ reasoning 内含 `</tool_call>` 而不含 `<tool_call>`。

在两条 wire 的全部 77 条 assistant 响应中，该签名精确命中且仅命中 4 条。

| wire | 响应序号 | reasoning 长度 | `<tool_call>` / `</tool_call>` | `<function=` / `</function>` | `<parameter=` / `</parameter>` |
|---|---|---|---|---|---|
| L1 | 17 | 763 | 0 / 1 | 0 / 1 | 3 / 4 |
| L1 | 25 | 811 | 0 / 1 | 0 / 1 | 1 / 2 |
| L1 | 37 | 913 | 0 / 1 | 0 / 1 | 1 / 2 |
| L2-R1 | 40 | 9967 | 0 / 1 | 0 / 1 | 0 / 1 |
| L1 | 35（成功对照） | 734 | 0 / 0 | 0 / 0 | 0 / 1 |

四次的每一层开标签都恰好比闭标签少一个，缺口构成块首的连续前缀。

全部 77 条响应中，reasoning 带有任一工具标记残留的共 5 条，即上表五条；
其余 72 条无残留。

**序号口径提示。** 上表序号为按 wire 中出现顺序对 assistant 响应的 1 起计数
（L1 共 37 条，L2-R1 共 40 条）。此前回报使用的编号为 L1 第 18/26/38 次、
L2-R1 第 40 次；L2-R1 一致，L1 三项相差 1。请按你方的调用计数口径核对后统一，
两套编号指向的是同一批响应。

---

## 5. 触发情境

四次失败均发生在模型反复受挫的时段：shell 工具拒绝较长命令或含复杂引号的命令，
模型在连续更换写法。L1 第 17 次明写 "The shell tool seems to reject longer
commands or commands with complex quoting"；作为对照的第 35 次处于同一困境，
并写出了臆造的 `</thinking>`。

标签错乱集中出现在长时挣扎区段。这与此前 `edit_file` 把 L1 的 0/7 变为 7/7、
并把 L2 带到 13/13 属于同一条线索：降低模型在单次输出中需要正确维持的结构复杂度，
失败率随之下降。

---

## 6. 不属于本次审计范围的一条观察

若 Keel 侧希望缓解，成本最低的做法是按第 4 节的签名识别并重试该轮，
无需改动模型或 vLLM。是否采纳属于 Keel 作者的决定，此处仅作记录。

---

## 附录 A：被推翻的中间假设

按预注册要求记录证伪过程。

**假设一（已推翻）：双适配器两阶段路径导致不可提取。**
即 `ParserEngineReasoningAdapter.extract_reasoning` 在
`skip_tool_parsing=True` 下消费开标签，随后
`ParserEngineToolAdapter.extract_tool_calls` 在 `CONTENT` 起始态上
只看到剥离后的内容。
证伪：`chat_completion/serving.py` 中不存在 `extract_tool_calls` 调用；
非流式路径调用的是 `parser.parse()`。该两阶段路径服务于其他调用者。

**假设二（已推翻）：解析器消费了块首前缀后中途放弃。**
由标记骨架"缺失的恰是连续前缀"推出。
证伪：`_on_terminal` 的全部 no-transition 出路均保留标记原文；
且 `TOOL_CALL_START` 未发出意味着从未进入工具状态。
读取 reasoning 正文后确认，被当作"参数值"的 458 字符实为模型的思考散文。

---

## 附录 B：证据采集方式

vLLM 源码阅读在服务主机的运行容器内以只读方式进行，脚本经管道送入
`docker exec -i <容器> bash -s`，主机与容器内均未落盘任何文件，
未修改任何配置，未向服务发起任何推理请求。

wire 文件与全部原始响应正文保留在实验机本地，未外传。本文所引正文
限于判定所必需的最小片段。

已读文件与行号区间。版本与根路径取自步骤 1b 实测：vLLM 0.26.0
（镜像 `vllm/vllm-openai:v0.26.0`，构建提交 `ffd46bfab2128bb84146050e98b51a617c6575ab`），
根路径 `/usr/local/lib/python3.12/dist-packages/vllm`，下表用 `<VLLM>` 代指。

行号取自本次只读探针的 `grep -n` / `sed -n` 直接输出，未经换算。

判定链主干：

| 文件 | 行号区间 | 内容 |
|---|---|---|
| `<VLLM>/entrypoints/openai/chat_completion/serving.py` | 836 | `def chat_completion_full_generator` |
| 同上 | 893-898 | `reasoning, content, tool_calls = parser.parse(output.text, request, ...)` — 非流式唯一提取入口 |
| 同上 | 153 | `self.parser_cls = ParserManager.get_parser(...)` |
| 同上 | 264-266 | `parser = self.parser_cls(...)` 实例化 |
| `<VLLM>/parser/engine/parser_engine.py` | 677-701 | `def parse` — 调 `_check_skip_tool_parsing` 后调 `_single_pass_parse`，不传 `initial_state` |
| 同上 | 645-673 | `def _single_pass_parse` — `_reset(initial_state=None)` → 配置初始态 |
| 同上 | 217-223 | `def _feed` |

抑制开关：

| 文件 | 行号 | 内容 |
|---|---|---|
| `<VLLM>/parser/engine/parser_engine.py` | 401-412 | `def _check_skip_tool_parsing` |
| 同上 | 408 | `if not self.skip_tool_parsing and not self._suppress_tool_calls:` |
| 同上 | 410-411 | `if tool_choice == "none" and tools: self._suppress_tool_calls = True` — 唯一置位条件 |
| 同上 | 125 | `self._suppress_tool_calls: bool = False` |
| 同上 | 719 | `suppress = self._suppress_tool_calls`（`_events_to_delta` 内） |
| 同上 | 166-171 | `skip_tool_parsing` property / setter，转发至 `self._engine` |
| `<VLLM>/parser/engine/adapters.py` | 52-58 | `def _skip_tool_parsing` 上下文管理器，唯一置位处 |
| 同上 | 74, 86, 114 | 三处 `with self._skip_tool_parsing():`，均在 reasoning 适配器内 |

论证 2.1（未匹配标记必被保留）：

| 文件 | 行号区间 | 内容 |
|---|---|---|
| `<VLLM>/parser/engine/streaming_parser_engine.py` | 302-352 | `def _on_terminal` 全文 |
| 同上 | 306-316 | `transition is None` 分支，末行 `return self._emit_for_state(value)` |
| 同上 | 308-315 | `DROP_TERMINAL` 是唯一 `return []` 的出路 |
| 同上 | 318-347 | `skip_tool_parsing` 分支（本链路不触发） |
| 同上 | 349-350 | `if transition.skip_in_token_id_mode and self._ever_had_token_ids: return self._emit_for_state(value)` |
| 同上 | 354-368 | `def _emit_for_state` |
| 同上 | 370-373 | `def _on_content` |
| 同上 | 375-401 | `def _apply_transition` — 迁移生效时消费标记文本，不回吐 |
| 同上 | 285-291 | token-id 严格模式下的分派（`strict` 集合内终结符转 `_on_content`） |

论证 2.2 / 2.3（槽位与提取结果）：

| 文件 | 行号区间 | 内容 |
|---|---|---|
| `<VLLM>/parser/engine/parser_engine.py` | 1011-1060 | `def _build_extracted_result` — 跳过 `not slot.name and not slot.args`；`tools_called = len(tool_calls) > 0` |
| 同上 | 706-785 | `def _events_to_delta`，`case EventType.TOOL_CALL_START: ... self._ensure_slot(...)`；719 行 `suppress = self._suppress_tool_calls` |
| 同上 | 786 | `def _ensure_slot` |

状态机定义：

| 文件 | 行号 | 内容 |
|---|---|---|
| `<VLLM>/parser/qwen3.py` | 100 | `initial_state=ParserState.REASONING if thinking else ParserState.CONTENT` |
| 同上 | 101 | `terminals={` |
| 同上 | 114 | `token_id_terminals={` |
| 同上 | 120 | `transitions={` |
| 同上 | 136-140 | `(ParserState.REASONING, "TOOL_START"): Transition(ParserState.TOOL_PREAMBLE, (REASONING_END, TOOL_CALL_START))`，136 行注释 "Tool call directly from reasoning (implicit end)" |

仅作交叉核对、未进入判定链：

- `<VLLM>/parser/abstract_parser.py`
- `<VLLM>/parser/engine/events.py`
- `<VLLM>/parser/engine/incremental_lexer.py`（1-223 全文结构）
- `<VLLM>/parser/engine/adapters.py` 1-200
- `<VLLM>/tool_parsers/__init__.py:161`（`"qwen3_xml"` 注册项）
- `<VLLM>/parser/engine/parser_engine.py` 490-515（`extract_reasoning`）、
  553-576（`extract_tool_calls_from_content`）—— 两者均不在非流式链路上，
  用于排除附录 A 假设一

以上行号已按 `step1b.sh` 第 4 节在服务主机上的实测输出回填。

权重侧记录（`step1b_output_fix.txt`）：服务中的 snapshot 为
`nvidia/Qwen3.6-35B-A3B-NVFP4 @ 1355db6a052410cfd62085d94b58866fd0f2c3c5`
（`refs/main`；另一 snapshot `491c2f1e…` 四个文件 sha256 完全相同）。
`chat_template.jinja` sha256 `e84f32a2…4259`，第 152 行生成提示以 `<think>
`
开头（thinking 默认开启，对应解析器初始态 REASONING）；第 53 行为模板自带的
工具调用格式指令。`generation_config.json` sha256 `e70c136c…550e`，
`temperature 1.0 / top_k 20 / top_p 0.95 / do_sample true`。
启动参数中 `--reasoning-parser qwen3 --tool-call-parser qwen3_xml
--enable-auto-tool-choice`，并启用 MTP 投机解码（`num_speculative_tokens 3`）；
后者与本判定无关，仅作环境记录。
