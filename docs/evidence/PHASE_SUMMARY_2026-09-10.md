# 阶段总结（2026-09-10，实验机）

范围：2026-09-07 至 09-10 的运行与审计。引用以 `docs/evidence/` 与 `PLAN.md` §8/§10（HEAD 5032a06）为准；每段末尾括号内为证据文件。不含下一步建议，不含对模型或 PIRA 的一般性评价。

## 1. 已验证的能力

**预算执行。** 机制：`--max-turns N` 限制每条用户消息的模型调用数（T22，实现于 `822fdbc`）；耗尽时 Keel 打印 `error: model-call budget exhausted (max_turns = N); the run is incomplete; the transcript is kept`，转录保留，不自动续接。验证：L2-R2 第 100 次调用后按此终止，运行以 11/13 记为失败；T25 阶段 2 的 D1-A-9 在第 12 次调用后按此终止，CSV 记为 `budget`，其余 79 次为 `completion`。两处的 `[max_turns] N per user message` 横幅均在发送任务前出现。（L2R2_2026-09-10.md；SHELL_CONTRACT_AB_2026-09-10.md；PLAN.md §8 T22）

**权限执行。** 机制：full 模式下声明为 `state_changing` 的调用须带非空 `safety_review`，Keel 先打印 `Safety:` 公告再执行，缺评审拒绝并可在下一次调用恢复，决策事件先于结果事件。验证：L2 18 次提议状态变更中 16 次带评审先公告后执行、1 次缺评审被拒并一步恢复、1 次结构校验拒绝，顺序违规 0；L2-R1 15 次带评审先公告后执行，3 次缺评审被拒、2 次恢复；L2-R2 27 次带评审先公告后执行、`Safety:` 公告 27、缺评审被拒 2 并恢复 2、顺序违规 0；T25 80 次运行的 `full_mode_check` 均为 `approve? 0`、`ordering violations 0`。握手机制的来源是 T15 实况验收（Mistral 9/9、Qwen 5/5 状态变更调用各带一次公告）。（L2_2026-09-09.md；L2R1_2026-09-09.md；L2R2_2026-09-10.md；SHELL_CONTRACT_AB_2026-09-10.md；T15_ACCEPTANCE_2026-09-07.md）

**错误可见性。** 三类失败都以明确错误行出现，不被当作成功完成。空回复：T19 实现后，无工具调用且文本为空的 assistant 消息返回 `LoopError::EmptyAssistantResponse`，L2-R1 第 40 次即以 `error: the model returned an empty response (no tool calls and no text)` 终止。预算耗尽：见上。结构性拒绝：`argv` 被作为字符串而非数组发送时，观察为 `input needs an array field 'argv'`，不执行；T25 两臂 18 个已确认参数错误事件全部属此类，每次都在下一次调用被纠正。（L2R1_2026-09-09.md；SHELL_CONTRACT_AB_2026-09-10.md；PLAN.md §8 T19、T22）

**转录保留。** 每次运行留下 SessionLog 与 wire 两份记录；事后审计与评分全部只读地建立在它们之上：T23 对两份 wire 共 77 次响应的结构化审计与四次空回复的签名比对；T24 对 L2-R2 SessionLog 的 100 次调用逐次时间线与循环 35 次的分类；T25 的 80 次运行由 `score_shell_ab.py` 从 SessionLog 与运行仓库机械评分，80 次脚本均无报错。（TOOL_PROTOCOL_AUDIT_2026-09-09.md；DEBUG_TRACE_2026-09-10.md；SHELL_CONTRACT_AB_2026-09-10.md）

**限定工作负载下的成功案例。** Gate D：冻结的多行 `edit_file` 编辑，`replacement_produces_expected_file` 10/10。L1-R1：隐藏验收 7/7（唯一变量为注册 `edit_file`，L1 为 0/7）。L1-R2：隐藏验收 7/7，full 模式，11 次状态变更编辑全部带评审。L2：隐藏验收 13/13，种子测试 42/42 保全，使用了 2 次操作员 `Continue`（协议允许 3 次）。T25 四个小任务：A 正确 34/40，B 正确 39/40。（EDIT_FILE_GATE_D_2026-09-08.md；L1R1_2026-09-08.md；L1R2_2026-09-08.md；L2_2026-09-09.md；SHELL_CONTRACT_AB_2026-09-10.md）

## 2. 仍未达到的目标

在当前模型配置下（`nvidia/Qwen3.6-35B-A3B-NVFP4` @ snapshot `1355db6a`，vLLM 0.26.0，Keel 不发送采样参数、由权重的 `generation_config.json` 决定），100 次预算、零 `Continue` 的 L2 自主完成尚未成功。两次尝试：L2-R1 在第 40 次调用以空回复终止，隐藏验收 10/13；L2-R2 在第 100 次调用以预算耗尽终止，隐藏验收 11/13。这是两次尝试的序列，不是率；L2 的 13/13 使用了 2 次 `Continue`，不属于零 `Continue` 条件。（L2R1_2026-09-09.md；L2R2_2026-09-10.md；L2_2026-09-09.md）

## 3. 已否决或未证实的方向

**工具描述 B（T25）。** 阶段 2 的 80 次运行后，B 未达预注册的候选推进门槛，三条中无一条完全成立：(1) 总正确 39/40 ≥ 34/40，但 R1 上 B 9 < A 10；(2) A 含已确认参数错误的运行 5，B 为 9，超过 ⌊5/2⌋=2；(3) 修正后 B 与 A 的辅助文件调用各 1，但 D1 上 B 1 > A 0。候选停止，描述不合入，运行时保持 `822fdbc`。B 文本针对的 shell 二次解析类错误在 80 次中出现 0 次；两臂全部 18 个已确认参数错误事件都是 `argv` 被当作字符串的结构性拒绝。（SHELL_CONTRACT_AB_2026-09-10.md）

**自动恢复。** 空回复后重试、从 reasoning 提取调用，均未批准。T23 文档"What remains open"第 2 条记录：实验机在其文稿第 6 节提出按签名重试该轮，作者以其与两项既定决定冲突（T19 不自动重试；Keel 只执行正式 `tool_calls`、不从 reasoning 读取）为由不采纳，登记为待用户评审事项，未实现。（TOOL_PROTOCOL_AUDIT_2026-09-09.md；PLAN.md §8 T19、T23）

**三个现象不得混为一个根因。**

- 模型协议失败（T23）。现象：四次空回复，`finish_reason=stop`、无 `tool_calls`、`content` 为空，reasoning 以工具调用闭合标签结尾而无 `<tool_call>` 开标签与 `</think>`。证据：两份 wire 77 次响应中恰有这 4 次（L1 第 18/26/38、L2-R1 第 40）；服务主机源码阅读表明非流式路径单次扫描完整文本、`(REASONING, TOOL_START)` 迁移可达，其余 73 次同配置正常提取，分类为模型输出不合规、vLLM 解析器无缺陷。区别：失败发生在模型生成与服务端解析之间，Keel 未收到任何可执行的调用，与工具参数或调试路线无关。（TOOL_PROTOCOL_AUDIT_2026-09-09.md）
- 调试绕路（T24）。现象：模型在第 66 次已看到完整断言差异（期望 `'a    failed    1        0:01:04'`、实际 `'a    failed  1        0:01:04'`），此后不再运行测试，转而重算自己期望的字符串直至预算耗尽。证据：Q1 为 yes（162 行直接输出进入 tool_result）；46 个 pira_ctx 结果 ID 0 次检索；循环 35 次中 H（辅助脚本与 shell/文件机制）22、R 3、F 8、I 1、O 1。区别：信息已送达且解析无误，消耗发生在模型对已有证据的使用方式上，不是协议失败也不是参数被改动。（DEBUG_TRACE_2026-09-10.md）
- 工具参数错误（T25）。现象：`argv` 以字符串而非数组发送，被 Keel 结构性拒绝。证据：阶段 1 五项传输测试在两台机器全过，L2-R2 第 68/74/77/80 次的 `SyntaxError` 全为非法一行 Python（复合语句在 `;` 之后），shell 二次解析仅见于显式 `cmd /C` 的第 39/54/92 次；阶段 2 的 18 个已确认参数错误事件全部是 `argv` 结构拒绝，二次解析 0 次。区别：参数原样到达，错误在模型构造请求的形状；先前归入"引号问题"的多数实为模型写出的非法 Python，与传输层和描述文本无关。（SHELL_TRANSPORT_2026-09-10.md；SHELL_CONTRACT_AB_2026-09-10.md）

---

## Keel 作者附注（2026-09-10，不改上文结论）

上文为实验机交付原文（Markdown 从交付页面原样取回，HTML 实体已还原）。逐项核对结果：

- 所有数字均与所引证据文件一致（抽查原句：L2 的 18/16/1/1、T15 的 Mistral 9 与 Qwen 5、L1-R2 的 11 次带评审编辑、T23 的 77/4/73、T24 的 35 次分类、T25 的门槛三条与 18 个事件）。
- 一处措辞超出可核实范围，源自作者自己的 T25 证据而非实验机：错误可见性一段中"每次都在下一次调用被纠正"。交付的 CSV 只能证明含 argv 坍缩的 14 次运行中 13 次随后正常结束（其中 D2-A-1 结束但答案格式不正确）、D1-A-9 以预算耗尽结束；"下一次调用即纠正"未在 SessionLog 上逐次核对。作者已同步把 `SHELL_CONTRACT_AB_2026-09-10.md` 中的对应句子改为可核实的表述；本文原文保留，以此附注为准。
- 无其他出入。本总结不含机制建议，符合收口决定；PLAN §10 已登记。
