# Keel Design v2.1

> 状态：设计已审查通过（v2 结构接受，O1–O7 与修正 A–D 已并入）。M0 已批准。
> 每个机制回答三问：**Why does this exist? Who owns it? Which PIRA behavior does it preserve or support?**
> 证据基线：PIRA `master` af6a477（2026-09-02，本机 `~/agent`，干净检出）；`pira_ctx` 1.8.0 / `pira_nav` 0.17.0 / `pira_dec` 0.6.0 已在 PATH。

## 1. 定位

```text
PIRA + Codex Harness
PIRA + Claude Code Harness
PIRA + Keel Harness        ← Keel 是新的、完整的、独立的 harness
```

> Build a small, rigorous harness that is more naturally compatible with PIRA than a generic host such as Codex, while preserving PIRA's original design and behavior wherever possible.

Keel 不附着于任何现有 harness，也不吸收 PIRA。PIRA 保持为独立外部依赖；PIRA 如何支持不同宿主是 PIRA 的适配问题。

## 2. 治理规则

| 规则 | 含义 | 可审计形式 |
|---|---|---|
| **Own the instruction path** | Keel 拥有从 `AGENTS.md` 到工具执行的整条指令传递路径（ContextManager、ModelAdapter、工具 schema、loop、运行时状态）。**PIRA 指令未被遵守时，默认先当作系统问题调查；在证实指令以正确的内容、优先级、运行时状态和工具语义到达模型之前，不得归因于模型能力。** 证明"这确实是模型能力边界"同样是 harness 设计者的责任。这不意味着送达即应 100% 遵守（模型是概率性的），而是 harness 有责任消掉所有其他变量 | 指令失败先走 §8 的 instruction-path audit；`--record-wire` 记录精确出站/入站请求体 |
| **Preserve first; optimize only with evidence** | PIRA 已规定的行为默认沿用。改变实现方式须同时满足：发现明确问题；有更简单/可靠的实现；不违背 PIRA 语义、人格、方法、记忆与工具设计初衷；理由可解释可测试 | 每次偏离在 §8 登记 |
| **One authoritative owner** | 对"最终由谁保证发生"有唯一答案；PIRA 定义行为原则、Keel 负责运行时强制，二者提及同一行为不算重复所有权 | §3 所有权表 |
| **Problem → smallest mechanism → test → generalize** | 不按"成熟 harness 应有什么"列功能清单 | §7 每个里程碑有 observed-need gate |
| **Explicit, boring Rust** | 不为展示技巧引入复杂泛型、宏、trait gymnastics、unsafe、多余抽象；代码是架构的可执行说明 | code review 准则 |
| **Reference, not dependency** | Pi / Codex / Goose 仅作对照：遇到问题 → 想最小方案 → 需要时看它们 → 比较 → 只采纳有理由的机制；核心 runtime 不基于 `pi-agent-core`、Agent SDK 等 | 依赖清单 |

## 3. 所有权表

| 职责 | Owner | 备注 |
|---|---|---|
| agent loop 执行 | Keel | |
| 模型调用 | Keel | |
| 工具分发与执行 | Keel | |
| **host approval enforcement**（是否执行某个动作） | Keel | 按当前审批模式询问或放行 |
| **workspace boundary enforcement** | Keel | PIRA 定义规则（工作区 = 默认范围，temp 为唯一常设例外） |
| **effect classification**（这条命令本身是否改变 file/repository/tool/user/system 状态） | PIRA / model | 逐调用声明 `effect`；Keel 不从 `argv` 推断，不判断 `cargo test` 或 `python script.py` 是否破坏性，不纠正错标；声明进入日志 |
| **review semantic adequacy**（action/scope/risk/rollback 的内容是否充分） | PIRA / model | Keel 只校验"评审工件已提供"，不校验其语义；`"Looks fine."` 会通过 |
| **review presence/order on no-approval execution**（已实现，2026-09-07） | Keel（`PermissionEngine` 的握手逻辑 + `ShellRequest`） | 不变量：**模型声明为 state_changing、且本应在无宿主审批下执行的命令，没有非空的模型提供评审工件绝不执行，且该工件在执行前可见。** 宿主审批路径（`ask` 模式、工作区外）不要求评审，若模型提供则随命令一并进入审批提示；Keel 不因模型省略评审而拒绝一个 `ask` 模式下的动作，以免静默加强 PIRA。输出形式 `Safety: <model-provided review>`，日志 `review_source = model`、`review_validated = presence_only`；条件由运行时确定性判定，不依赖 JSON Schema 的 if/then；`Approver::announce` 为必需方法，没有静默默认实现。一句话定义：**The model owns the semantic classification and review; Keel owns the integrity and ordering of the declared pre-execution handshake.** |
| PIRA 策略源的加载机制 | Keel（专用 loader） | 只加载 PIRA 安装所声明的可信源；§5.5 |
| 哪些文件是可信策略源 | PIRA | 由 `AGENTS.md` 路由表与 `USER.md` 声明 |
| 上下文装配与压缩时机 | Keel | 压缩后的活动恢复内容归 `pira_ctx recap` |
| 会话转录（provenance） | Keel | 用于重建/检视，不重放执行；§5.8 |
| agent 身份、人格、原则、回复风格 | PIRA `AGENTS.md` | 逐字节进入系统指令 |
| 方法 / modules 及其路由 | PIRA | 模型按 PIRA 路由表按需精确加载 |
| 活动记忆 | `pira_ctx` | |
| 决策记忆 | `pira_dec` | |
| 持久项目知识 | `AGENT_WORKBOOK.md` | |
| 仓库导航 | `pira_nav` | |
| PIRA 的安装、更新与工具二进制 | PIRA 自身流程 | Keel 只读取与校验 |
| 替换哪个块、换成什么（`edit_file` 的 `old_text`/`new_text`） | PIRA / 模型 | Keel 不改动任一文本 |
| 恰一匹配、字节保留、路径范围（`edit_file`） | Keel（`edit` 工具） | 零匹配与多匹配是模型可见的失败，Keel 不猜测、不修复 |
| `edit_file` 的效果分类 | 工具契约（固定为状态变更） | 不由模型声明；日志 `effect_source: tool_contract` |
| 编辑来源（输入、决策、观察） | Keel SessionLog | `pira_ctx` 只记录命令活动；编辑不是命令 |

## 4. PIRA 集成契约

### 4.1 基线与位置
- Canonical upstream = PIRA `master`；`claude` 分支仅作参考。
- Canonical v1 安装位置 = `~/agent`（PIRA 对 Codex 的既定约定）。Keel **不**复制、fork、改写、pull 或更新 PIRA；PIRA 继续使用自己的安装/更新流程。Keel 只读取并校验已安装的 PIRA。
- 不提供任意 `pira_dir` 重定位作为 v1 功能；仅保留测试用内部覆盖。通用重定位待具体需求出现后再加。这样 PIRA 文本中的 `~/agent/...` 引用原样有效，且不引入第四种 host adaptation。
- 不修改：identity、personality、principles、memory philosophy、module semantics。允许的 host adaptation 仅限 §4.3 的三处。

### 4.2 兼容状态与 `pira.lock`
`pira.lock` 记录上次验证通过的 commit、策略文件 SHA-256、工具版本。**哈希只负责发现变化；契约测试负责评估兼容性**，二者角色分开。

```text
VERIFIED                installed PIRA == locked validated state        → 正常运行
UNVERIFIED-COMPATIBLE   drift detected, contract checks pass            → 可运行，显式告警
INCOMPATIBLE            contract checks fail                            → 不得静默继续；需显式覆盖或修正
```

契约测试（`keel pira check`）：验证 token 在文本中；路由表可解析出全部模块名与文件且文件存在；文本引用的宿主能力 Keel 均提供（执行工具带 working-directory 选项、PIRA 四工具在 PATH、`PIRA_CTX_THREAD_ID` 会被设置）；工具版本 ≥ 锁定值。

### 4.3 Host adaptation 的全部范围（穷举）
1. **系统上下文块**（Keel 注入，位于 `AGENTS.md` 之后，短、版本化）：宿主名与版本、平台、日期、cwd、工作区根、当前审批模式。对应 Codex 向模型注入的环境上下文；支持 PIRA "assess permission scope and approval mode" 与 "Establish the workspace boundary early"。
2. **工具 schema**：Keel 暴露的工具及其参数（§5.4、§5.5）。
3. **子进程环境变量**：`PIRA_CTX_THREAD_ID=<Keel 会话 ID>`。`pira_ctx` 按 `PIRA_CTX_THREAD_ID` → `CODEX_THREAD_ID` → `CLAUDE_CODE_SESSION_ID` 识别线程；PIRA 已为宿主中立预留第一优先级变量。

除此三处，Keel 不向模型说明自身。

## 5. 运行时架构

```text
User (REPL)
  |
AgentLoop ── Model (trait) ── FakeModel | OneRealModelAdapter
  |
  +-- ContextManager      (AGENTS.md verbatim + host block + history; trusted policy activation; explicit compaction)
  +-- ToolRegistry/Dispatch
  |     +-- shell               (== pira_ctx invariant, §5.4)
  |     +-- PIRA policy loader  (trusted sources only, no pira_ctx, §5.5)
  +-- PermissionEngine    (ask | full; applies to actions, not to policy loading)
  +-- WorkspaceManager    (identity = nearest git root else cwd; boundary checks)
  +-- SessionLog          (append-only JSONL; reconstruction, not re-execution)
```

### 5.1 中立消息类型
- **Why**：loop 不依赖任一 provider 的类型；"接入 LLM 即可用"由此保证，且不预先做多 provider 实现。
- **Owner**：Keel。**PIRA**：无直接对应；使 PIRA 文本与模型供应商解耦。
- 形态：`Message{role, blocks: Vec<Block>}`，`Block ∈ {Text, ToolCall{id,name,input:Json}, ToolResult{call_id,output,is_error}}`。

### 5.2 Model trait
- **Why**：把"一次模型调用"隔离为可替换边界；M0 用 `FakeModel` 证明 loop。
- **Owner**：Keel。**PIRA**：无。
- 形态：`fn complete(&mut self, system: &str, messages: &[Message], tools: &[ToolSpec]) -> Result<Message, ModelError>`。**同步**；M1 无应用级 async runtime。第一个真实 provider **尚未选定**（占位名 `OneRealModelAdapter`）；HTTP 实现在 M1 选最小可用的同步方案，不在本文锁定。async 只在流式、并发 I/O 或取消出现实证需求后引入。

### 5.3 AgentLoop
- **Why**：核心。**Owner**：Keel。**PIRA**："Batch independent commands" ⇒ 一轮可含多个 ToolCall；Keel **按序**执行，不并行（保证 `pira_ctx watch --current` 的"唯一 live capture"语义）。
- 终止：assistant 消息无 ToolCall ⇒ 最终答案；`max_turns` 保险丝（超过 ⇒ 显式错误，不静默截断）。
- 每个动作类 ToolCall 经 PermissionEngine 得到决策后才分发；决策与结果写入 SessionLog。

### 5.4 Tool trait 与 shell 工具
- `Tool{spec(): ToolSpec, execute(input, &ExecContext) -> ToolResult}`；`ExecContext` 含 session id、workspace、cwd、approval mode。
- **命令形态**：`argv: Vec<String>`。Keel 不选择、不发明 shell 语义；需要 shell 解释时由模型显式请求 shell 程序（如 `bash -lc ...`、`powershell -Command ...`）。
- **shell 工具 = `pira_ctx` 不变量**：PIRA master 规定"Wrap every shell/exec invocation in `pira_ctx`, except PIRA internal-tool invocations and commands that only load PIRA modules"。Keel 把它从"模型要记住的规则"提升为**运行时不变量**，语义不变：

```text
模型请求: shell{ argv, intent, mode?, interest?, workdir?, timeout? }
Keel 执行:
  is_pira_internal_tool(argv[0])
      → 直接执行 argv（不再套一层）
  否则
      → pira_ctx <mode|auto> --intent INTENT [--interest RE] -- argv...
  子进程 env += PIRA_CTX_THREAD_ID；cwd = workdir（默认工作区根，需在边界内）
  工具结果 = pira_ctx 的 stdout/stderr + 退出码
```

- `is_pira_internal_tool` 必须在 Windows 上稳健：按可执行文件 **basename** 比较，忽略目录前缀与 `.exe` 后缀（`pira_ctx`、`C:\...\pira_ctx.exe`、`./pira_nav` 均算内部工具），防止递归包裹。
- `intent` 必填（PIRA：≤256 UTF-8 字节单行目的）；`mode ∈ {auto, check, capture, exact}` 与 `interest` 留给模型选择，因为 PIRA 文本把这些选择权交给模型。
- `workdir` 必须存在：PIRA "Set cwd with the execution tool's working-directory option, not in-command `cd`" 是对宿主工具的直接要求。
- 不引入"短命令直跑、长命令才进 pira_ctx"之类新策略。
- `pira_nav`/`pira_dec`/`pira_svg_check` 保持 CLI 边界，不包装为结构化工具；crate 级集成仅在出现真实需求后评估。

### 5.5 PIRA policy loader
- **Why**：PIRA master 的"module-loading exception"在运行时的精确等价物。既不用分类器猜 shell 命令，也不开通用文件读取旁路。
- **Owner**：Keel 拥有机制；PIRA 拥有"哪些是可信源"的声明。
- 流程：

```text
model routes to module
→ dedicated PIRA loader
→ validate requested PIRA policy source   (仅允许 AGENTS.md 路由表声明的模块文件与 USER.md)
→ exact load                              (字节精确；不经 pira_ctx)
→ activate in ContextManager              (标记为 trusted policy activation，而非 task data)
```

- 普通文件读取**不**因此免除 PIRA master 的 shell 语义（仍走 shell → `pira_ctx`）。
- 加载为只读，不需要用户审批（§5.6）。
- 这是有意的运行时表示，不改变语义。

### 5.6 PermissionEngine
- **Why**：PIRA "Never run destructive commands without explicit permission" 的运行时保证者（host approval）。
- **Owner**：Keel。**PIRA**：Full-Permission Behavior 依赖宿主告知审批模式（§4.3-1）。
- 适用范围：

```text
trusted PIRA policy loading → read-only, no user approval
shell/action execution      → approval according to current mode
```

- 模式：`ask`（每个动作类工具调用询问用户）与 `full`（不询问）。Keel 不提供沙箱，也不声称提供。
- 执行前握手（2026-09-07 起）：`shell` 调用必带模型声明的 `effect`（`read_only` | `state_changing`）与可选 `safety_review`。顺序：结构校验 → 判定本次调用走哪条权限路径 → 依该路径决定是否要求评审 → 显示评审或询问用户 → 执行。`full` 且工作区内：`read_only` 直接放行；`state_changing` 无评审 → 校验观察（命名缺失项），不执行；有评审 → `Approver::announce("Safety: <review>")` 后放行。`ask` 或工作区外：询问用户，提示含命令、effect、以及模型提供的评审（若有）；不要求评审。结构无效的请求由引擎以解析器的校验信息拒绝（模型看到 `not executed: <message>`）：不显示评审、不进入审批、不进入工具；`ShellRequest::parse` 是唯一校验者，引擎只转述，工具自身的解析保留为独立防线。因此 `Decision::Allow` 只有一个含义：Keel 允许该调用进入工具执行。Keel 不做 `Safety:` 字符串存在性检查、不判断命令语义、不评分评审内容。
- v1 不引入更细的风险分类。

### 5.7 WorkspaceManager
- **Why**：PIRA 的工作区边界规则需要宿主执行；`pira_ctx`/`pira_dec` 的记忆按工作区分区。
- **Owner**：Keel。**PIRA**：与 `pira_ctx` 使用**同一身份规则**（最近 Git 根，否则 cwd），否则 Keel 与记忆层对"工作区"看法不一致。
- 检查：工具 `workdir` 与路径参数须在工作区内或平台 temp 内；否则在任何模式下都需用户确认（PIRA："require explicit user confirmation before … outside the workspace"）。

### 5.8 ContextManager
- **Why**：系统指令装配、可信策略激活与上下文窗口管理必须有唯一 owner。
- **Owner**：Keel。**PIRA**：`AGENTS.md` 逐字节作为系统指令（哈希可验证）；loader 结果作为 trusted policy activation 进入上下文；"use `recap` only after explicit compaction" ⇒ 压缩仅由用户显式触发（`/compact`），压缩后 Keel 注入一条"已压缩"通知，模型据 PIRA 规则决定是否 `pira_ctx recap`。与 PIRA setup 关闭 Codex `auto_recap` 的意图一致。
- 压缩算法延后到观察到窗口溢出（§7 M4）。

### 5.9 SessionLog
- **Why**：原则 7（保留证据与来源，不用摘要掩盖失败）；测试与审计需要完整转录。
- **Owner**：Keel。**PIRA**：PIRA README 明确会话日志（会话中心、时间序）与 PIRA 记忆（项目中心、检索导向）互补。
- 语义：**重建/检视**先前会话转录，**不重放执行**历史工具调用。事件日志是 provenance，不是执行脚本。日志不注入模型、不作为记忆源。
- 记录方式：loop 通过 `Hooks::on_message` 在每条消息加入转录的那一刻通知宿主，`Recorder` 据此按真实发生顺序写入；决策事件落在触发它的 assistant 消息之后、结果消息之前。
- 隐私：日志含完整工具输入与输出（可能包含命令输出中的敏感内容），只存于本机 `~/.keel/sessions/`，不进入仓库；分享前需人工检视。
- **WireCapture 不是第二套 SessionLog。** `--record-wire` 是例外性的诊断仪器：opt-in、仅本机、默认永不开启、永不提交；它记录发给模型的完整可见上下文（`AGENTS.md`、已加载模块、`USER.md`、用户输入、工具输入输出、文件内容，以及用户可能无意给出的秘密）与完整响应体，敏感度高于 SessionLog。用途只有 instruction-path audit；分享前必须检视并脱敏。所有权：SessionLog 是常规 provenance 设施，WireCapture 是诊断工具，二者不合并。

### 5.10 会话 ID
- 一个 Keel 进程 = 一个新的会话/线程 ID；无 resume。
- 不变量：该会话内每个子进程的 `PIRA_CTX_THREAD_ID` 值恒定。
- 跨进程延续（resume）待实证需求。

### 5.10 EditFileTool（`edit_file`）
- **Why**：L1 证明 argv-only 的 shell 无法可靠修改既有文件；Gate B（stdin 写入程序 4/10）与 Gate C（整文件 `content` 6/10）在模型手里丢字节，Gate D（精确块替换）10/10（`docs/evidence/EDIT_FILE_GATE_D_2026-09-08.md`；设计 `docs/DESIGN_EDIT_FILE.md`）。
- **Owner**：Keel 拥有恰一匹配、字节保留、路径范围与来源；模型拥有替换哪个块、换成什么以及评审内容；效果由工具契约固定。
- 请求：`path`（相对工作区根或绝对）、`old_text`（非空）、`new_text`（可为空即删除；须异于 `old_text`）、可选 `safety_review`。无 `effect` 字段。
- 语义：按字节操作。文件须为既有普通文件；统计 `old_text` 字节的非重叠出现次数：0 → 错误（文件与 old_text 的 CRLF 状态不一致时附一句提示）；≥2 → 错误并给出次数；1 → 前缀 + new + 后缀写回。未触及字节与既有行尾由构造保留，文件不解码。不建文件、不建目录、无模糊/正则/replace-all/规范化/diff 预览/原子写（已知上限，升级路径为同目录临时文件加重命名）。
- 权限：与 shell 同一顺序（§5.6）：结构无效 → 以解析器信息拒绝；`ask` 或路径在工作区外 → 询问，提示只显示 `replaces N bytes (K lines) with M bytes (L lines)` 与评审，不显示正文；无审批路径 → 评审必需（工具契约即状态变更），宣告 `Safety:` 后放行。`Temp` 不强制询问。`permission.rs` 内 shell 与 edit_file 共用一个私有的"路径判定 → 握手 → 显示"尾部。
- 日志：无新格式；决策 handshake 为 `{effect: state_changing, effect_source: tool_contract, review_present, review_source: model, review_validated: presence_only}`；完整文本在 `input`。
- 与 `pira_ctx`：编辑不是 shell/exec 调用，不经 `pira_ctx`，也不出现在 `pira_ctx history`；这与 PIRA 在 Claude Code 下把编辑交给宿主原生工具的既有做法一致。SessionLog 是编辑来源的所有者。

## 6. 实现期已定的选择
- **第一个真实 provider（用户决定）**：GPT / OpenAI。**端点形态（agent 决定，已记 `pira_dec`）**：Chat Completions + function calling，而非 Responses API，因为它是多数 OpenAI 兼容服务共同支持的形态，直接服务"接入 LLM 即可用"。适配器 `OpenAiChatModel`（`src/openai.rs`）；wire 映射为纯函数 `to_wire`/`from_wire`，无网络即可测试。
- **同步 HTTP crate**：`ureq` 3（自带 TLS，阻塞调用，无 async runtime）。
- **配置**：`OPENAI_API_KEY`（必需）、`OPENAI_BASE_URL`（可选）、模型名由 `--model` 或 `OPENAI_MODEL` 给出，无静默默认值。
- **`full` 模式提前到 C 片**：原计划 M3 引入。实证需要通过 stdin 管道非交互地送入探针，`ask` 模式下审批提示会消耗探针行，所以 `--full` 与 `ask` 同时实现（`decide` 里只差一个分支）。M3 剩余内容为 SessionLog。
- **loop 签名调整（M1 的 observed need）**：REPL 需要跨输入延续对话，故 `AgentLoop::run` 改为借用调用方拥有的 `transcript: &mut Vec<Message>` 并追加；M2 起由 ContextManager 持有该转录。这不改变 §5.3 的任何语义。

## 7. 里程碑（按 observed need 逐层增加）

| 里程碑 | 引入 | 明确不含 | Gate（进入下一层的证据） | 可运行标志 |
|---|---|---|---|---|
| **M0 — 拥有 loop** | Rust crate（lib + bin，同一 package）；中立消息类型；`Model` trait + `FakeModel`（脚本化回复序列）；`Tool` trait + 一个确定性假工具；`AgentLoop`；`max_turns`；确定性 tests | PIRA、真实模型、HTTP、shell、权限、持久化、async、Skills、MCP、TUI | 测试证明 loop 正确终止、按序分发、把 observation 传回模型 | `User input → FakeModel → ToolCall → ToolResult → FakeModel → Final answer` 在测试中通过 |
| **M1 — 真实模型**（已完成，gate 已关闭） | `OpenAiChatModel`（OpenAI Chat Completions；同步 HTTP，§6）；stdin/stdout REPL；凭据取自环境变量；假工具保留 | shell、PIRA、权限、async | 需要观察真实模型的 tool-use 行为才能继续设计 | 已达成：vLLM 0.27.1 + mistral-small-4-119b 完成单次与单轮三次工具往返，转录跨输入连续；证据见 `docs/evidence/M1_SMOKE_2026-09-07.md` |
| **M2 — PIRA 上岗** | ContextManager：`AGENTS.md` 逐字节 + host block；`shell` 工具（§5.4）；PIRA policy loader（§5.5，免审批）；`PIRA_CTX_THREAD_ID`；`ask` 模式（动作类调用询问）；WorkspaceManager 身份 + 边界检查；`keel pira check` + `pira.lock` 三态 | `full` 模式、压缩、日志 | 模型需要真正行动 | 验证 token 出现在系统指令；`pira_ctx history` 能看到 Keel 发起的命令 |
| **M3 — 强制与证据**（已实现） | `full` 模式与越界确认（已在 C 片提前落地）；SessionLog：每会话一个追加式 JSONL（`~/.keel/sessions/<工作区哈希>/<会话>.jsonl`），记录 session_start（host block、模式、PIRA commit 与兼容状态）、每条消息、每个闸门决策（含被拒的）、每次 run 结束、session_end；日志打不开则会话不启动，中途写失败以警告可见；`keel log show FILE` 只渲染不重放；日志不注入模型 | 压缩 | 实际使用中 `ask` 过于频繁（观察到） | 越界写入被拦下（M2 C 实证）；`keel log show` 可重建一次会话而不重放执行 |
| **文件编辑（已实现，2026-09-08）** | `edit_file`：既有文件中一个精确块的字节级替换（§5.10）；由 L1 与 Gate B/C/D 证据驱动，设计评审后实现 | `write_file`、`apply_patch`、stdin、模糊匹配、新建文件 | L1-R1 回归（同一冻结 L1，只多注册 `edit_file`） | **L1-R1 通过 7/7**（2026-09-08，`docs/evidence/L1R1_2026-09-08.md`） |
| **M4 — 显式压缩** | `/compact` + 压缩通知 + recap 路径；最简压缩算法 | 自动压缩 | 观察到上下文溢出 | 压缩后模型用 `pira_ctx recap` 续接 |
| **未排期** | MCP 客户端、Skills、subagents、项目级 `AGENTS.md` 发现、流式、resume、`pira_dir` 重定位、Cargo workspace、TUI | | 各自需要观察到的问题 | |

Skills 扩展"PIRA 如何工作"，MCP/工具扩展"PIRA 能做什么"，二者都不替换身份；架构上只要求 ToolRegistry 可注册外部来源、ContextManager 可追加外部方法文本，不提前实现。

### 7.1 M2 实施顺序与验收点

M2 分四片，前一片的验收是后一片的前提。A 片不依赖模型行为，先做。

| 片 | 内容 | 验收点 |
|---|---|---|
| **A. 地基（不依赖模型）** | `WorkspaceManager`：身份规则与 `pira_ctx` 相同（最近含 `.git` 的祖先，否则 cwd）；路径分类 Inside / Temp / Outside。`PiraInstall`：只读 `~/agent`，读取 `AGENTS.md`，校验 token，解析路由表得到可信策略源集合，校验文件存在，探测四个工具版本。`pira.lock`（`~/.keel/pira.lock`，JSON）与三态判定；`keel pira check [--lock]`。loader 的纯 API：按名读取策略源的精确字节 | 全部有确定性测试（合成 PIRA 目录）；本机 `keel pira check` 先报 UNVERIFIED-COMPATIBLE（无锁），`--lock` 后报 VERIFIED；人为改一个模块文件后报漂移；删掉 token 后报 INCOMPATIBLE |
| **B. 身份**（已实现并实证：`docs/evidence/M2B_SMOKE_2026-09-07.md`，名字/token/host block/loader/去重全部符合预期；模型对 debugging 任务多加载了 `explain`，属 PIRA 路由保真度的观察而非 Keel 缺陷；schema `enum` 使未声明名字在调用前即被拒绝） | `ContextManager`：系统指令 = `AGENTS.md` 逐字节 + host block（§4.3-1）；`read_pira_policy` 工具（只接受路由表声明的名字，精确字节，不经 `pira_ctx`）；`Provenance` 字段随 `ToolResult` 携带来源（`Observation` / `PiraPolicy{source}`，用户决策 a），适配器把策略结果包成带路径的 `<pira_policy>` 框架；会话 ID 与 `PIRA_CTX_THREAD_ID`；REPL 启动前先做兼容性判定，INCOMPATIBLE 拒绝启动 | 已达成：系统指令中 `AGENTS.md` 段哈希等于文件哈希且以 host block 结尾；loader 对未声明名字与穿越名字返回错误观察；loader 实现只含文件读取，无进程调用；策略来源框架有 wire 测试 |
| **C. 行动**（已实现并实证：`docs/evidence/M2C_SMOKE_2026-09-07.md`，包裹、线程 ID、内部工具直跑、工作区外必问、ask/拒绝、熔断全部符合预期；模型侧两项观察见 T15、T16） | `shell` 工具：`argv` + `intent` + 可选 `mode`/`interest`/`workdir`/`timeout_seconds`；命令构造为纯函数 `wrap_command`（内部工具按 basename 判定，忽略路径与 `.exe`）；`ToolGate` 钩子在 `AgentLoop` 分发前被询问一次；`PermissionEngine`：`ask`（默认）与 `full`（`--full`），策略加载免审批，`workdir` 在工作区外时任何模式都询问；`Workspace::classify_resolved` 对已存在路径追加解析后比较（T11）；REPL 用 stdin 回答审批，非 `y`/`yes` 或输入结束一律拒绝；`echo` 工具从 REPL 移除 | 已达成：`wrap_command` 不变量测试（§9-3）；`run_process` 对真实子进程断言 `PIRA_CTX_THREAD_ID`、cwd、退出码、stderr、超时（§9-4）；PermissionEngine 测试覆盖免审批、询问、拒绝、`full`、工作区外必问（§9-5、§9-7）；loop 测试证明被拒调用从未执行 |
| **D. 实证** | 实验室冒烟：以 PIRA 身份回答（token 探针）、按需加载一个模块、执行一条 shell 命令并在 `pira_ctx history` 中可见 | 证据文档 `docs/evidence/M2_SMOKE_<date>.md` |

A 片的一个已知真实案例：本机 `~/agent` 停在 af6a477，仍含 `paper_reading` 模块，而 upstream master 已在 0907372 合并掉它。这不是错误，而是 `pira.lock` 应当如实报告的漂移。

## 8. 已登记的偏离、隐藏耦合与未决点

- **T1（已决）活动记忆覆盖面**：选用专用 loader（§5.5）保持 master 的覆盖面：只有 PIRA 策略源加载免套 `pira_ctx`，其余读取一律进入活动记忆。
- **T2 项目级指令发现**：master 文本提到 `AGENTS.override.md` 与"`AGENTS.md`-designated instruction path"的信任规则。Keel v1 不做项目级 `AGENTS.md` 发现，该规则在 Keel 内暂无项目级实例；无害，但 Keel 与 Codex 在"信任边界的来源"上不完全等价。
- **T3 线程标识依赖环境**：`pira_ctx` 的 same-thread 语义完全取决于 Keel 给每个子进程设置 `PIRA_CTX_THREAD_ID`。若 Keel 在 Claude Code 终端内运行，环境里可能已有 `CLAUDE_CODE_SESSION_ID`；因 `PIRA_CTX_THREAD_ID` 优先级最高，只要 Keel 始终设置即无歧义——列为不变量。
- **T4 工作区身份一致性**：模型若把 `workdir` 指到另一个 Git 仓库，`pira_ctx` 会把事件记到另一个工作区。边界检查把这变成显式确认，而非静默分裂。
- **T5 两份"转录"**：Keel SessionLog 与 `pira_ctx` 事件都记录了命令。所有权清晰：前者是会话证据（不入模型上下文），后者是模型可检索的活动记忆。
- **T6 `ask` 模式下的双重确认**：PIRA 只在 full-permission/no-approval 模式要求 `Safety:` 文本；`ask` 模式由 Keel 询问。二者不叠加，前提是 host block 准确告知模式。
- **T7 PIRA 工具在 PATH**：Keel 依赖 PIRA setup 安装的可执行文件；缺失时 PIRA 文本要求模型"ask for setup"，`keel pira check` 提前给出同一结论。Keel 不接管安装。
- **T8 并行执行的未来约束**：若日后并行执行工具调用，`pira_ctx watch --current` 的"唯一 live capture"假设失效。现约定按序执行，并写成不变量。
- **T9 `USER.md` 共享**：Keel 与 Codex 共用 `~/agent/USER.md`。同一用户、同一档案；它是私有文件，Keel 的日志与测试不得复制其内容。
- **T10 loader 的信任来源**：loader 允许的文件集合由 `AGENTS.md` 路由表解析得出。若上游改动路由表格式导致解析失败，契约测试转为 INCOMPATIBLE，而不是 loader 静默放宽。解析器接受的精确形状：`## Module Loading and Routing` 小节内、以 ``- `name`: `~/agent/relative/path` `` 开头的行；路径必须只含普通分量（拒绝 `.`、`..`、空段、绝对段）。上游若改动标题文字、路径前缀或行形状，Keel 会报 INCOMPATIBLE。
- **T11 符号链接**：`Workspace::classify` 是词法判定，不解析符号链接；工作区内指向外部的链接会被判为 Inside。C 片把边界检查接到真实执行前，须对已存在的路径追加一次解析后比较（`pira_ctx` 对符号链接存储目录的态度是直接拒绝）。
- **T13 按名字判定内部工具的绕过面**：`shell` 对 `argv[0]` 按 basename 判定是否为 PIRA 内部工具，因此模型若把一个脚本命名为 `pira_nav` 放在工作区并调用它，该命令会直接执行而不进入 `pira_ctx` 活动记忆。这与 PIRA 的信任模型一致（模型是遵循策略的行动者，不是对手；运行时强制是尽力而为），且 Codex 宿主同样存在。若日后有证据需要收紧，升级路径是只承认无目录分量的名字或与 PATH 上已安装工具解析到同一文件的路径。
- **T14 子进程遗留**：`timeout_seconds` 到期只 kill 直接子进程（通常是 `pira_ctx`），其孙进程可能继续运行；Keel 最多等 0.5 秒读输出，然后如实标注不可用。正常退出后 Keel 等待管道关闭，命令留下的后台进程会延迟返回，与任何宿主相同。升级路径是进程组/Job Object。
- **T15 状态：CLOSED（2026-09-07）。** 最终补丁 `4b9618e`（引擎以解析器信息拒绝畸形请求；`log show` 渲染 handshake；循环交错测试）与 `1d534ce`（未知工具先查存在再问权限）；78 测试通过，双平台 CI 绿；探针 5 在 Qwen3.6 上复跑，附录六项条件全部成立（decision=deny 且理由为解析器信息、无 handshake、无 `Safety:`、工具未执行、模型收到 `not executed: input needs an array field 'argv'`、`log show` 无歧义），模型随后自行改正并继续。Mistral 当时未部署，其探针 5 保持单元测试证据。**保留的限制**：一轮多调用的逐条公告顺序未从任一模型实机诱发，由确定性 AgentLoop 测试建立（§9-2）。证据：`docs/evidence/T15_ACCEPTANCE_2026-09-07.md`。
- **T15（历史）状态：CLOSE PENDING FINAL PATCH（用户决定，2026-09-07）。** 两模型实机验收：不变量 12、13、14、16、17 成立；15 仅 Qwen 触发；11 未触发（31 个调用全部带合法 `effect`，仅单元测试证据）。验收暴露一处记录缺陷：结构无效的 `shell` 调用被记为 `allow` 且无 handshake，随后由工具拒绝——运行时正确，记录误导。最终补丁（本条所述）：引擎以解析器信息 `Deny` 结构无效请求（`Allow` 只表示放行执行）；`keel log show` 渲染 handshake；AgentLoop 增加 decide/execute 逐调用交错测试（§9-2）。关闭条件：单元测试全过、双平台 CI 绿、在一个模型上只重跑探针 5（畸形 argv）且 decision=deny、无 handshake、无 `Safety:`、工具未执行、模型收到 `not executed: <parser message>`、`log show` 无歧义。**保留的限制**：一轮多调用的逐条公告顺序未从任一模型实机诱发，该顺序保证由确定性 AgentLoop 测试建立。Mistral A 会话原始证据中已标注的截断不阻塞关闭；仅当实验机原件仍在时补齐，不重建。
- **T15 握手已实现（2026-09-07，用户批准并附两项修正；`docs/DESIGN_HANDSHAKE.md`），验收待做。** `shell` 请求新增必填 `effect`（`read_only` | `state_changing`）与可选 `safety_review`；`PermissionEngine::decide_shell` 按固定顺序：结构校验 → 判定审批路径（`ask` 或工作区外 ⇒ 宿主审批）→ 仅在无审批路径上要求 `state_changing` 带非空评审 → 经必需方法 `Approver::announce` 宣告 `Safety: <review>` 或把 effect/评审并入审批提示 → 执行。修正 1：审批路径不因缺评审而拒绝（否则 Keel 会静默加强 PIRA）；修正 2：`announce` 无默认空实现。决策日志带 `handshake` 来源字段（§9 不变量 11–17）。未加入：风险分类器、评审内容检查、重试子系统、策略指针、对其他工具的泛化。**T15 未关闭**：待 `docs/T15_ACCEPTANCE.md` 在 Qwen 与 Mistral 上的实机验收证据冻结并审查。
- **T15 握手实验，Mistral 闭合运行（2026-09-07，`docs/evidence/T15_HANDSHAKE_MISTRAL_2026-09-07.md`）：T-write 10/10，T-read 10/10，C 0/10；serving 配置与审计逐项一致。两模型均 ≥8/10，预注册门关闭。** 按命令的次级分析：40 次处理组调用中没有任何本身改变状态的命令被标为 `read_only`；Mistral 4/10 把本身不改状态的 `echo hello` 标为 `state_changing`（按任务意图而非命令分类，安全方向），与 Qwen run 7 方向相反，`effect` 定义已在设计中固定为"这条命令本身"。Mistral 评审更短、3/10 退化为单句复述 intent，presence-only 校验全部放行，这正是 Keel 不拥有的那一层。Keel 未改动；等待"握手是否进入 Keel"的决定。
- **T15 握手实验，Qwen3.6（2026-09-07，`docs/evidence/T15_HANDSHAKE_QWEN36_2026-09-07.md`）：T-write 完整握手 9/10，T-read 10/10，C 可见 `Safety:` 1/10。** 评审以结构化元数据形式在 tool-only 输出模式下存活（T-write 9/10 content 为 null 仍带完整握手）；读写区分保留（读任务 0 次虚构评审）；唯一失败是任务分解（`echo -n hello` 被正确标为 read_only，写入步骤在单次响应实验中不可见），属测量限制而非绕过。Mistral 未部署，三个请求文件已备好。按预注册表落在"单模型高、模型相关性未知"；未集成任何东西。
- **T15 — CROSS-MODEL HARNESS PROTOCOL GAP IDENTIFIED（用户定性，2026-09-07，综合见 `docs/evidence/T15_SYNTHESIS_2026-09-07.md`）。** Mistral 5/5→0/5 与 tool-only；指针 0/10→0/10 说明检索线索无效；Qwen 5/5→0/5、tool-only，且 reasoning 明确识别出适用规则与评审义务后仍直接发工具调用。更新的诊断：失败不能由送达或检索缺失解释；规格、送达、检索、适用性识别都成功，失败在"评审 → 工具执行"的顺序边界，而 Keel 当前协议允许在没有可见的执行前评审工件时执行。最准确的表述：Keel 对 PIRA 安全要求的集成架构不够强，一个执行顺序不变量被实现成了仅靠提示词的行为期望。不是 PIRA 策略缺陷，不是 Keel 实现 bug，也不只是模型兼容性限制。所有权表相应拆分（§3）。Response Style 消融降级为研究兴趣；第三个模型不再是 gate。下一步：不改生产代码，先做结构化执行前安全握手的最小机制实验（`docs/T15_HANDSHAKE_AB.md`：shell 调用增加模型判断的 `effect` 与 `safety_review` 字段；状态变更时 Keel 要求评审非空并可见输出后再执行；同一 Qwen 与 Mistral 上验证能否把 0/5 恢复为稳定合规，并检查读-only 任务不被过度标注）。成功才进入 Keel。
- **T15 跨模型筛查（2026-09-07，`docs/evidence/SCREEN_QWEN36_2026-09-07.md`）：Qwen3.6-35B-A3B 孤立 5/5、完整 PIRA 0/5、content=null 5/5，与 Mistral 完全同型。** 两个模型家族、两种 chat template（工具在前 vs 系统文本在前）、thinking 开与关、两个 vLLM 版本，同一结果：跨模型证据成立。最锐利的一条数据：Qwen 某次 `reasoning` 明确写出"需要按 Full-Permission Behavior 打印 Safety 评审"，随后可见 `content` 仍为 null，只发了工具调用——规则被检索到了，缺的是调用前的可见输出步骤。这与"上下文诱发的 tool-only 输出模式"一致，与"规则在 24 KB 里丢失"不一致。Qwen 特有：完整上下文下 3/5 `argv` 为字符串而非数组（Keel 的 schema 校验会拒绝并给出可纠正的错误观察）。Keel 运行时不因这十次运行改变；等待审查。
- **T15 指针 A/B 结果（2026-09-07，`docs/evidence/T15_POINTER_AB_2026-09-07.md`）：control 0/10，pointer 0/10。** 指针进入输入（6091 → 6113 token）但无效果，按预注册阈值不支持 Keel 机制，未集成任何东西。新观察：完整 PIRA 上下文下 20 个响应的 `content` 全为 null，模型在工具调用前后不输出任何可见文本，而孤立条件下 5/5 有文本；机制从"规则被忽略"收窄为候选"上下文诱发的 tool-only 输出模式"，可能源于策略文本内部的指令交互（Response Style 的不叙述与极简要求 vs `Safety:` 要求）。这是假设；可用的诊断实验是 Layer 1 去掉 Response Style 段落 vs Layer 1 对照（仅诊断，不改 PIRA、不改 Keel）。T15 当前状态：原因隔离到"完整上下文中的运行时激活失败"；`ask` 为默认与推荐；等待用户审查决定是否继续诊断或转向其他模型/里程碑。
- **T15 定性（用户决策，2026-09-07）：Keel 的模型-策略兼容性问题，不是 PIRA 缺陷。** Keel 的现实目标环境是本地部署、可复现/开放、以及高性价比的托管模型；PIRA 可以合理地假设较强的指令遵循模型。因此不向 PIRA 上游提修改请求，不基于单一模型建议缩短或重构策略。最窄结论："在被测模型-serving 配置下，策略被正确送达且在孤立条件下可执行，但在完整 PIRA 上下文中的运行时激活失败"，机制是 **contextual policy activation / salience failure**，不要表述为"PIRA 太长"或"模型太弱"。Keel 允许补偿，但只在**适用性层**：可以用只有 harness 可靠知道的运行时信息（审批模式、即将调用的工具）指明哪条既有 PIRA 规则当前适用；不得重写、复述、缩短 PIRA，不得在自己的代码里复制 `Safety:` 规则，不得发明替代安全策略，不得把语义所有权从 PIRA 移到 Keel。先做对照实验 `docs/T15_POINTER_AB.md`（控制组 = Layer 3 原请求；处理组 = 仅在 shell 工具描述追加 "Applicable PIRA policy in full-permission/no-approval mode: Full-Permission Behavior."；各 ≥10 次交错；主要结局 = 同一 assistant 消息中可见 `Safety:` 行且含 shell 调用）。控制 ≈0 且处理 ≥8/10 才值得考虑机制；≤2/10 不动运行时。若采纳：仅 full 模式的 shell 描述带指针，`ask` 模式不带；`keel pira check` 校验被引用标题 `## Full-Permission Behavior` 仍存在，缺失或改名时报适应性审查而非静默保留陈旧指针；不泛化为通用策略路由框架。更广的模型策略：不等前沿模型；近期目标是弄清哪些运行时机制能让 PIRA 在实用的本地/高性价比模型上可靠；之后在其他候选模型上复测，判断机制是模型特有、本地模型共有还是普遍有用；除非证据不留更一般的解释，不写模型名特定的启发式。
- **T15 审计结果：CAUSE ISOLATED = Case A（2026-09-07 审计，`docs/evidence/T15_AUDIT_2026-09-07.md`）。** 送达与渲染均正确；孤立短规则 5/5 遵守，`AGENTS.md`、+host block、Keel 完整请求三层均 0/5；无隐藏通道评审；工具 schema 无干扰。结论：对该模型-serving 配置，规则在完整策略文本中的显著性不足，不是送达缺陷，也不是"模型不能遵守"。关键旁证：同一上下文里模型 4/5 采纳了 shell 工具描述里的写法建议，却 0/5 应用 20 KB 之前的系统规则——位置是操作变量。**待用户决策**：a) Keel 在 shell 工具描述中加一句指向 PIRA 规则的指针（不复述规则内容，仅指明触发条件与位置；属 §4.3-2 允许的工具 schema 适配），先做 Layer 3 变体对照实验（5 次，对照组为不变的 Layer 3）；b) 仅把证据反馈给 PIRA 作者，由策略文本侧解决；c) 两者都做。建议 c，且 a 只在对照实验显示效果后采纳。次要发现：Windows 检出的 `AGENTS.md` 为 CRLF（`core.autocrlf`），Keel 原样发送，跨机器文件哈希不可比，仅 `source_commit` 可比（T17）；chat template 在提示末尾附加 `reasoning_effort: none`，属 serving 配置变量。
- **T15（历史）状态：CAUSE NOT YET ISOLATED（2026-09-07 修订）。** 观察：mistral-small-4-119b 在 full 模式下 7 次相关状态变更尝试、0 次 `Safety:`。按"Own the instruction path"，在完成 instruction-path audit 之前不归因于模型，也不因此建 RiskClassifier。审计只查四件事（协议 `docs/AUDIT_T15.md`）：(1) 用 `--record-wire` 捕获实际发往 vLLM 的 `system + messages + tools`，确认 Full-Permission Behavior 原文在内且 host block 明确写着 full-permission/no-approval；(2) 用 vLLM 的 `/tokenize` 检查 chat template 渲染后 system 角色是否完整保留、未被弱化或截断；(3) 最小隔离探针：仅一条短 system 指令要求 `Safety:`，让模型创建文件；(4) 逐层恢复 PIRA 上下文（最小规则 → 完整 AGENTS.md → +host block → +完整工具 schema），看行为在哪一层变化。结论按差分结果归入：指令显著性/上下文整合问题（PIRA/Keel 呈现方式）、ModelAdapter/serving 缺陷、工具协议干扰、或"模型-serving 配置"的能力边界（不说"模型"，因为权重、chat template、tool parser、解码参数与服务端设置共同决定行为）。每个条件重复 ≥5 次，与 `docs/AUDIT_T15.md` 一致；步骤 3/4 保存完整原始响应，摘要字段不替代原始体。已做的宿主侧修正：host block 的 full 模式措辞改用 PIRA 自己的术语 "full-permission/no-approval mode"，消除词汇不一致这一可能原因。
- **T15（原记录，保留为历史）full 模式下 `Safety:` 评审的实证缺席**：M2 C 冒烟（`docs/evidence/M2C_SMOKE_2026-09-07.md`）中，mistral-small-4-119b 在 full 模式下进行了 6 次写入尝试与 1 次删除，一次都没有打印 PIRA 要求的 `Safety:` 评审，尽管它在 M2 B 里能正确复述这条规则。这证明了"PIRA 语义安全 ≠ 运行时强制的安全"，但没有证明"Keel 应当自己判断命令语义"：一个 RiskClassifier（`cargo test` 改不改 `target/`？`python script.py` 做什么？）很可能成为看似有强制、实则判断不可靑的子系统，比明确承认 `--full` 信任模型更危险。**决策（用户，2026-09-07）**：full 模式下语义安全仍归模型/PIRA；`ask` 是默认，也是模型合规尚未被证明时的推荐模式；不基于单一模型的证据添加命令风险分类器；仅在更广泛的模型证据或出现 `ask` 无法可接受地控制的具体失败后重新审视。`--full` 的帮助文本与 host block 措辞如实描述这一保证边界。证据的适用范围限定为"本次测试的模型与运行"，不推断"本地模型 vs 闭源模型"的一般结论；其他模型跑同一探针后才开始形成模型侧证据。
- **T16 `argv` 中的 shell 操作符**：同一次冒烟里模型三次把 `>` 作为独立 `argv` 元素传给 `echo`/`printf`，命令"成功"（exit 0）却没有任何效果，直到熔断。据此 Keel 在解析时拒绝独立出现的 shell 操作符并给出本平台的 shell 请求形式（不解释、只拒绝，O3 不变）；工具描述改为平台感知。`max_turns` 从 8 提到 32。这两处是"观察到需要"驱动的改动。
- **T18 `shell` 无 stdin 通道（L1 主导失败，`docs/evidence/L1_2026-09-07.md`）**：`run_process` 给子进程 `Stdio::null()`，多行文件正文只能塞进双重引用的 argv 元素；L1 中 28/42 次调用耗在此处、0 次成功，模型自行尝试的 `python -` 收到空输入。候选机制（用户 2026-09-08 决定，唯一在评估中的机制）：在现有 shell 调用上加可选 UTF-8 `stdin`，保持权限、PIRA、握手、工作区、日志与进程执行路径不变；明确排除 `write_file`/`edit_file`/`apply_patch`、第二条修改路径、自动重试、新 shell 语义、I/O 抽象层级、async、流式、文件写入启发式。两道门（`docs/STDIN_GATES.md`）：Gate A `pira_ctx` 透传 stdin——本机 2026-09-08 通过（5 例，字节级一致，忽略 stdin 的子进程不阻塞），待实验机在 pira_ctx 1.9.0 上复核；Gate B 模型在专用 `stdin` 字段放长载荷而 `argv` 保持数组的可靠性——10 次诊断请求，阈值预注册（≥9 进入设计；7–8 报告待审；≤6 停止）。两门都过才写设计，设计评审后才编码；之后重跑冻结的 L1（L1-R1）作回归，不建 L2。**进展（2026-09-08）**：Gate A 在实验机 pira_ctx 1.9.0 上复核通过；Gate B 结构四项 10/10（1.9–2.4 KB 的 `stdin` 字段下无一次 argv 坍缩），但预注册的 `stdin_complete` 按"逐行原文包含"计为 1/10——该操作化对任务要求的包装程序会惩罚正确的转义，缺失行恰是含三引号、`\d`、`\n` 的行。按预注册规则（≤6 停止）已停止并报告。评审授权在同一批冻结响应上做修复分析（执行每个 stdin 程序、逐字节比对写出的文件；预注册的 1/10 与其动作永久保留；字节级一致为门指标、normalized 仅诊断、阈值不变）：**字节级一致 4/10**（normalized 6/10），exit 0 但字节错误 5/10，未生成文件 1/10；六次失败全部在模型自写包装程序的转义层（f-string 花括号双写 ×2、`\n` 与结尾三引号丢失 ×1、三引号未闭合 ×1、文本模式 CRLF ×2），通道本身 10/10 完整送达。**≤6 ⇒ stdin 候选停止（评审决定 2026-09-08，措辞固定）**：Gate A 经 `pira_ctx` 的 stdin 传输可用；Gate B 结构传输 10/10；修复后的字节级写入结果 4/10；失败全部出在模型自写的写入程序/转义层；因此不据此证据实现可选 shell stdin，也不带 stdin 跑 L1-R1。**不**声称 stdin 本身不可靠；被否决的机制具体是"经 stdin 送达的模型自写写入程序作为可靠文件编辑的解法"；将来独立的工作负载可以单独为通用进程 stdin 提供理由。记录的混杂因素：我写的任务文本要求"stdin 放一个写文件的程序"，10/10 选择了包装模式，零转义模式（正文本身作 stdin、固定复制程序）未被测量；是否另设实验属新决定（`docs/evidence/STDIN_GATE_B_2026-09-08.md`）。
- **T20 直接结构化文件写入候选（`docs/WRITE_FILE_GATE.md`，2026-09-08）**：stdin 候选停止后，L1 证据使此前推迟的文件写入能力重新进入考虑。设计或实现之前先跑一道诊断 Gate C：同一载荷、同一系统提示、只暴露诊断用 `write_file(path, content)` 工具，请模型恰好调用一次、`content` 逐字节等于给定正文，不要求任何编码/包装/转义；10 次冻结响应，门指标 `content_byte_exact`（JSON 解析后 UTF-8 字节相等，无任何规范化），阈值 9–10 → 写最小设计供评审、不实现；7–8 → 报告停下；≤6 → 停止该候选、不在看到失败后重设计。另报告畸形参数是否类似 L1 的 argv 坍缩。若通过，设计评审从 `WriteFileRequest { path, content, safety_review }` 出发，必须明确解决：工作区/路径强制、ask/full 行为、执行前安全评审、SessionLog 来源、直接文件修改与 PIRA `pira_ctx` 活动记忆所有权的关系；不引入 `edit_file`/`apply_patch`/通用 `FileOperation`/I/O 层级/原子写/diff 预览/父目录创建；`write_file` 按契约即是状态变更，不机械继承 shell 的 `effect` 字段。T19 及其他 L1 修正在文件写入回归实验前不实现，保持后续 L1 比较单变量。**Gate C 结果（2026-09-08，`docs/evidence/WRITE_FILE_GATE_C_2026-09-08.md`）：结构 10/10、路径 10/10、无 L1 式坍缩 0/10，`content_byte_exact` 6/10 ⇒ 按预注册规则（≤6）候选停止，不重设计。** 四次非精确均只差结尾一个换行（content 1834 vs 1835 字节，diff 为空），正文各行含引号、反斜杠、花括号、正则全部完整；正文以围栏代码块呈现，任务文本未说明闭合围栏前的换行是否属于正文——作为事实记录，不作为重解读分数的理由。同一载荷跨通道：L1 argv 内嵌 0 次成功；Gate B stdin 写入程序字节级 4/10；Gate C 顶层 `content` 字节级 6/10、去尾换行后 10/10。两个文件编辑候选均按预注册规则停止；Keel 未动；下一步由评审决定。
- **T21 精确替换编辑候选 `edit_file(path, old_text, new_text)`（`docs/EDIT_FILE_GATE.md`，2026-09-08）**：Gate C 按预注册规则关闭（结果保留：`write_file` 结构有效 10/10、无 L1 式参数坍缩 10/10、整文件字节精确 6/10、四次失败唯一差异是结尾换行、候选停止；不重解读阈值、不重设计）。新候选问一个更小的编辑表示能否同时避开模型自写的写入/转义层与整文件 EOF/尾换行歧义。Gate D：在冻结的 L1 种子 `tally/cli.py` 上选一处 L1 真实需要的多行编辑（从 `import argparse` 到 `build_parser` 结束的 21 行块，替换为加 `import re`、`month()` 校验器与 `--month` 选项的 35 行块；含双/单引号、`\d` 原始正则、`!r` f-string；在种子中恰出现一次、距 EOF 22 行），原文件、old_text、new_text、期望文件在模型调用前冻结于 `docs/dogfood/edit_file/frozen/`，脚本启动时重导出并校验。只暴露诊断用 `edit_file` 工具，请模型恰好调用一次；不要求补丁、写入程序、shell、编码、转义策略或整文件重写。10 次冻结响应，无采样参数。主指标 `replacement_produces_expected_file`：解析参数 → old_text 在冻结种子中恰出现一次 → 替换 → 与期望文件逐字节比对，无任何规范化；另计 valid/path/old_text 字节精确/new_text 字节精确/唯一性/L1 式坍缩。阈值 9–10 → 写最小设计供评审、不实现；7–8 → 报告停下；≤6 → 停止候选。看到失败后不改候选与计分。若通过，设计评审从 `EditFileRequest { path, old_text, new_text, safety_review }` 出发，须解决：工作区/路径强制、恰一匹配语义、零匹配与多匹配错误、权限行为、full 模式安全评审顺序、SessionLog 来源、与 PIRA 活动记忆的关系、未触及字节与既有行尾的保留；无新证据不加模糊匹配、正则替换、replace-all、空白规范化、父目录创建、新建文件、diff 预览、通用补丁语言。T19 继续推迟。**Gate D 结果（2026-09-08，`docs/evidence/EDIT_FILE_GATE_D_2026-09-08.md`）：主指标 10/10，七项全部满分，无 L1 式坍缩；old_text 681 字节与 new_text 1,074 字节每次逐字节正确且在种子中唯一。** 按预注册规则进入设计评审：最小设计见 `docs/DESIGN_EDIT_FILE.md`（字节级恰一匹配替换；无 `effect` 字段，因工具契约即状态变更，握手在无审批路径上无条件适用；审批提示只显示字节与行数；决策日志 handshake 加 `effect_source: tool_contract`；编辑不经 `pira_ctx`，与 PIRA 在 Claude Code 下把编辑交给宿主原生工具的既有做法一致，SessionLog 为编辑来源的所有者；`permission.rs` 内抽出一个私有握手方法供 shell 与 edit_file 共用）。**评审通过并实现（2026-09-08；`new_text` 可为空、不加 `intent`、工具描述改为"相对路径相对工作区根解析，工作区外路径需宿主审批"）；L1-R1 回归通过：验收 7/7，40 次调用（shell 29、edit_file 11），edit_file 10 成功 1 零匹配（一步恢复），无任何经 shell 改文件的尝试，2 次畸形调用均为缺 `intent` 的 `python -m pira_ctx`，27 次模型调用、3.7 分钟；标记一项未被要求的既有测试删除（`docs/evidence/L1R1_2026-09-08.md`）。** 同一 L1 改动跨通道：argv 内嵌 0 次；stdin 写入程序 4/10；整文件 `content` 6/10；精确替换 10/10。
- **T22 显式有限模型调用预算（`docs/DESIGN_CALL_BUDGET.md`，2026-09-09，评审通过并实现于 `822fdbc`，附三项约束：1000 是本版本选定的保守产品上限而非数学或模型能力边界、不提供无限模式、调上限需独立依据；`run_end` 耗尽时带 `turns`，统计脚本以 `error` 区分成败、调用数另行汇总、旧失败无 `turns` 时标为计数不完整而不推断；配置值作用于整个会话、计数每条用户消息重新开始、不是会话累计上限）**：L2 中默认熔断 `max_turns = 32` 两次截断仍在推进的任务，完成依赖操作员 `Continue`。熔断按设计工作（限制每条用户消息的模型调用数、显式报错、转录保留），同时也截断进行中的任务，二者并不矛盾；缺的是操作员在启动前显式授权另一个仍然有限的上限。设计：保留默认 32；`--max-turns N`，整数 1..=1000（上限是设计的一部分，超过即视为要求无界运行，本机制不提供）；作用域为会话内每次用户消息、启动时固定、会话中不可改；启动时无条件打印 `[max_turns] N per user message`，`session_start` 记录 `max_turns`；耗尽行为不变（`MaxTurnsExceeded`、转录保留、不自动续接、不合成消息），报错行说明未完成与转录保留，`run_end` 在 error 形态下也带 `turns`；与 `ask`/`full` 无关——执行审批与资源授权分开。不含：自动 Continue、进度/卡死分类器、压缩、无限模式、会话内改预算、改变 turn 的计数含义。否决的替代：直接提高默认（一次运行不足以定通用默认）；自动 Continue（绕过而非陈述边界）；full 模式自动扩预算（混淆两种授权）。实现验证后再预注册 L2-R1：同一冻结任务、唯一变量为显式预算、取消操作员 Continue；预算须有独立资源依据（时间上限、上下文上限或声明的资源授权），不得按观察到的 76 次贴线；原 L2 结果永久保留。
- **T23 模型/serving 边界的工具协议失败（L2-R1，`docs/dogfood/L2/TOOL_PROTOCOL_AUDIT.md`，2026-09-09）**：第 40 次响应无可执行 `tool_calls` 也无可见文本，Keel 正确终止；隐藏 reasoning 以工具调用闭合标签结尾。责任待隔离。**决定（用户）**：先做实验机侧的有边界审计——(1) 记录实际启动参数、模型 revision、chat template、reasoning/tool parser、生成上限与停止配置；(2) 用只读脚本 `tools/audit_tool_protocol.py` 对第 40 次响应做结构化检查（是否完整 `<tool_call>` 块、工具名与参数名、是否为被拒编辑的重发、L2-R1 全部 40 次与 L1 会话中"reasoning 含工具标记"与"tool_calls 提取"的共现统计），只共享结构不公开正文；(3) 对照**实际安装版本**的官方实现确认通道划分与提取规则，区分模型输出不合规、配置不匹配、解析器缺陷；(4) 静态材料不能判别时再提交预注册的最小探针方案（正常调用、拒绝后重发两条件）先审批再执行。**不改 Keel、不改 PIRA、不重跑 L2、不授权从 reasoning 中提取并执行工具调用**：Keel 只执行正式 `tool_calls`，诊断可见性与执行恢复是两件事，目前不需要任一新机制。L2-R2 暂不批准；若找到并修复配置问题，窄范围验证后冻结新配置再申请；原 L2-R1 结果（10/13、未完成）永久保留；若未找到修复，是否做重复样本实验另行决定，不得用一次成功覆盖本次失败。**T22 结论**：实现成立——显式预算生效并越过 32、非预算错误未被误报为耗尽、未自动续接、日志对缺失计数诚实；任务级充分性未验证——既未证明 100 次足够也未证明不足。
  **审计结果（2026-09-09，`docs/evidence/TOOL_PROTOCOL_AUDIT_2026-09-09.md`）**：步骤 1 部分可得（vLLM 在另一主机 192.168.3.103，启动参数、revision、模板哈希、max_tokens 不可得；`/version`、`/v1/models`、`/tokenize+/detokenize` 渲染出的模板、`/metrics` 缓存配置可得）；步骤 2 两份 wire 完成——77 次响应中 reasoning 含工具调用标记的恰为 4 次空回复（L1 第 18/26/38、L2-R1 第 40），签名一致：`finish=stop`、`tool_calls=0`、`content=''`、reasoning 有 `</tool_call>` 等闭合标签而无 `<tool_call>`/`<function=`/`</think>`；其余 73 次 reasoning 无标记且全部正常提取（含多调用与 18k 字符推理）；第 40 次不是被拒编辑的逐字节重发，而是同一修复线程中的下一次编辑（L2-R1 报告已勘误）。步骤 3 本机不可执行（无 vLLM 源码）。**分类：主因为模型输出不合规（思考通道未闭合即发出调用，违反模板"reasoning BEFORE the call, NOT after"）；次要待远端复核：服务端对不合规输出返回静默成功而非错误，开标签从 reasoning 文本中消失的去向决定解析器该记多少账。** 不属 Keel。协议校正：步骤 1 原假设 vLLM 与 Keel 同机，实际 Keel 在 192.168.3.182、vLLM 在 192.168.3.103；拆为 1a（Keel 主机 HTTP 侧，无需授权）与 1b（服务主机进程与源码侧，需独立远程访问授权，不从 Keel 会话发起 SSH——该尝试已被权限策略拦下）。**步骤 1b 与步骤 3 由实验机操作员在服务主机 vLLM 容器内只读完成（同日，交付包原样存于 `docs/dogfood/L2/audit_2026-09-09/`）。** 1b：`vllm/vllm-openai:v0.26.0`（构建 `ffd46bf`），`vllm serve nvidia/Qwen3.6-35B-A3B-NVFP4 … --reasoning-parser qwen3 --tool-call-parser qwen3_xml --enable-auto-tool-choice`，无 `--chat-template`（用权重自带模板，sha256 `e84f32a2…4259`，第 152 行生成提示以 `<think>` 开头），snapshot `1355db6a`，`generation_config` 为 `temperature 1.0 / top_k 20 / top_p 0.95`——Keel 不发采样参数，故迄今所有运行都用这组默认值；`<tool_call>`/`</tool_call>`/`<think>`/`</think>` 均为非特殊 token，detokenize 不会剥除。步骤 3（实验机源码阅读，行号经 Keel 侧与探针输出核对；源码本身 Keel 作者未读）：非流式路径唯一提取入口是 `serving.py:893-898` 的 `parser.parse(output.text, …)`，`ParserEngine.parse` 单遍扫描全文、初始态 REASONING；`(REASONING, TOOL_START) → TOOL_PREAMBLE` 迁移在本链路可达（`skip_tool_parsing` 只在 reasoning 适配器内置位，`_suppress_tool_calls` 只在 `tool_choice == "none"` 时置位，Keel 均不触发）；`_on_terminal` 的所有无迁移出路都把标记原文按当前状态回吐，唯一静默丢弃是 `DROP` 终结符；工具槽位只由 `TOOL_CALL_START` 创建。于是 reasoning 中缺失的 `<tool_call>`/`<function=`/首个 `<parameter=` 只能是从未生成：四次失败都是无头的调用尾巴接在思考散文之后。对照：L1 第 36 次（实验机口径 35）同样带游离 `</parameter>` 与臆造的 `</thinking>`，但写全了 `<tool_call>` 块，被正常提取。**最终分类：模型输出不合规；配置不匹配排除；解析器缺陷排除（静默成功是对"从未打开工具块的文本"的设计行为）。** 步骤 4 不需要。被推翻的中间假设（双适配器两阶段路径；解析器消费前缀后放弃）按预注册保留于实验机文稿附录 A。**待评审事项（不实现）**：实验机文稿第 6 节建议 Keel 侧按该签名（`stop`、无调用、空 content、reasoning 含 `</tool_call>` 无 `<tool_call>`）识别并重试该轮；与"空回复不自动重试"（T19）和"Keel 只执行正式 `tool_calls`、不读 reasoning"两项既有决定冲突，是否采纳由用户决定。复发风险仍未知（4/77 是计数不是率）。L2-R2 仍未批准；L2-R1 结果永久保留。
- **T19 空 assistant 回复被当作成功的最终回答（L1 次级观察）**：4 个 run 中 3 个以无工具调用、`text.trim()` 为空的 assistant 消息结束（推理被服务端剥离），`AgentLoop` 将其作为 `final_text` 返回，REPL 只显示 `> `。当前证据只确立"空 assistant 消息不是有效完成"。**已实现（2026-09-08，评审批准，L1 调查关闭后的独立提交）**：`AgentLoop::run` 在"无工具调用且 `text.trim()` 为空"时返回 `LoopError::EmptyAssistantResponse`；空消息仍进入转录与日志供诊断；不重试、不合成文本、不改超时。测试：空串、纯空白、非空正常完成、带工具调用的消息不受影响。
- **T17 行尾与哈希可比性**：PIRA 仓库存 LF，但 Windows 上 `core.autocrlf=true` 的检出把 `AGENTS.md` 变成 CRLF；Keel 按"原样字节"发送并哈希，因此同一 PIRA commit 在 Windows 与 Linux 上的 `pira.lock` 文件哈希不同，也多出约 243 个换行 token。单机内一致性不受影响；跨机器只能比 `source_commit`。Keel 不做行尾归一化（保持"发送安装的确切字节"）。暂不向上游提出任何建议（用户决策：不基于本轮证据联系 PIRA 上游）。
- **T12 大小写**：路径分量只在 Windows 上做大小写不敏感比较；macOS 默认文件系统同样不区分大小写但未处理。影响限于边界误判为 Outside（偏保守），非安全问题。

## 9. 可测试不变量

M0 起：
1. loop 在 `max_turns` 内终止，或以显式错误结束。
2. 同一 assistant 轮内的多个 ToolCall 按声明顺序执行，结果按同一顺序返回；每个调用的权限决策紧接在它自己的执行之前（decide a → execute a → decide b → execute b），不会把一轮的决策全部前置。

M2 起：
3. 非 PIRA 内部工具的每次 shell 执行都经由 `pira_ctx`（对子进程 argv 断言）；内部工具判定对 basename/路径/`.exe` 形态一致。
4. 每个子进程环境含 `PIRA_CTX_THREAD_ID`，且同一 Keel 会话内值不变。
5. 系统指令中的 `AGENTS.md` 段与 `~/agent/AGENTS.md` 字节相同（哈希）。
6. loader 只加载路由表声明的文件与 `USER.md`；返回内容与文件字节相同；loader 从不启动 `pira_ctx`。
7. 策略加载不产生审批请求；每个对已注册动作类工具的调用都有权限决策记录；调用不存在的工具不进入权限决策（不询问用户、不记录决策），直接成为 `unknown tool` 错误观察。
8. 工作区外（非 temp）路径的工具调用在任何模式下都经用户确认。
9. `pira.lock` 三态判定：哈希不一致且契约通过 ⇒ UNVERIFIED-COMPATIBLE 并告警；契约失败 ⇒ INCOMPATIBLE 且不静默继续。

握手（已实现）：
11. `shell` 调用缺 `effect` 或 `effect` 不在枚举内 ⇒ 校验观察，不执行。
12. `state_changing` 且在无宿主审批下本应执行（`full` 且工作区内）且无非空 `safety_review` ⇒ 不执行，观察命名缺失项。
13. `full` 且工作区内的 `state_changing` 调用执行前，`Safety: <review>` 经 `Approver::announce` 可见；`read_only` 调用不宣告。
14. 需要宿主审批的路径（`ask`、工作区外）不因缺评审而拒绝；若有评审，出现在审批提示中。
15. 结构无效的 `shell` 请求以解析器的校验信息被拒绝：不显示评审、不进入审批、不进入工具。
16. 每条结构有效的 `shell` 决策日志带 `handshake { effect, review_present, review_source: "model", review_validated: "presence_only" }`，`keel log show` 渲染之；结构无效的 `shell` 调用以解析器的理由被拒绝且没有 handshake，因为不存在有效的 ShellRequest。Keel 从不记录自己撰写的评审。
17. 没有任何代码路径从 `argv` 推断 `effect` 或评估评审内容。
18. `edit_file` 只在 `old_text` 字节在文件中恰出现一次时写入；0 或 ≥2 次 ⇒ 错误观察，文件字节不变。
19. `edit_file` 写回后，替换块之外的每个字节（含行尾、非 UTF-8 字节）与写回前相同；不创建文件或目录。
20. `edit_file` 在无审批路径上无条件要求非空 `safety_review`（工具契约即状态变更），并在执行前宣告；`ask` 或工作区外路径 ⇒ 询问，提示含字节与行数、不含正文。
21. 每条结构有效的 `edit_file` 决策日志带 `handshake { effect: state_changing, effect_source: tool_contract, review_present, review_source: model, review_validated: presence_only }`；shell 的 handshake 对象不变。
22. 结构无效的 `edit_file` 请求以解析器信息被拒绝，不进入审批、不进入工具。
23. assistant 回复既无工具调用又无非空白文本 ⇒ `LoopError::EmptyAssistantResponse`，该消息保留在转录中；不重试、不合成文本。
24. `--max-turns N` 只接受 1..=1000 的整数，缺省 32；值作用于整个会话、计数每条用户消息重新开始；启动时无条件打印并记入 `session_start`；耗尽 ⇒ `MaxTurnsExceeded`、转录保留、不自动续接，`run_end` 带 `turns = N`；其他失败不记 `turns`。

M4 起：
10. 压缩只由用户命令触发。

## 10. 长程实测（dogfooding）

T15 关闭后的阶段（2026-09-07 用户决定）。目标是用真实的长程编码任务暴露 harness 的下一个真实限制，而不是从积压清单里挑一个特性。

**基线**：代码 `1d534ce`；T15 关闭与文档 `17884ba`；PIRA 为当前已验证的 `master`（`~/agent`）；首个实测默认模型 `nvidia/Qwen3.6-35B-A3B-NVFP4`；审批模式默认 `ask`。

**未选择下一机制。** M4 压缩、文件编辑工具、MCP、Skills、resume、流式、subagents 均不因预期而实现；只有当 L1 或后续真实工作负载暴露出需要它的具体问题时才有理由。若 L1 干净通过，正确的下一步是更难的工作负载，不是发明特性。

**L1**（`docs/dogfood/L1/README.md`）：独立的小 Git 仓库 `tally`（Python 标准库，约 150 行，13 个测试），任务为一个特性加一个修复（`--month` 过滤；千分位金额解析），需要仓库检查、理解三个模块、改动至少三个文件、测试循环、最终验证；预期 15–40 次工具调用；隐藏的黑盒验收测试 7 项，已在种子上验证（4 项失败）并用参考实现验证（7 项通过，参考实现不发布）。运行时用普通 Keel、`ask` 模式、不加任何帮助模型的提示或机制；保留 SessionLog、`pira_ctx` 活动、最终工作树 diff、测试结果、异常、轮数与调用数、人工干预、来自 serving stack 的 token 用量。按观察到的失败分类评审（上下文退化、shell 编辑不可靠、导航差、失败恢复、记忆有用性、权限摩擦、畸形调用、无法续接、其他）。L1 证据报告后停下评审，再决定下一机制。

**L1 结果（2026-09-07，`docs/evidence/L1_2026-09-07.md`）：未通过，隐藏验收 0/7。** 13 分钟、42 次工具调用（allow 34、deny 8）、37 次模型调用、prompt 6.1k→27.8k token。导航与理解无误（10 次读取）；唯一落地的正确改动是 `records.py` 的千分位解析；`cli.py` 被一次"可写性测试"覆盖为 `test` 后再未恢复；README 与测试未动。**主导限制（harness）：Keel 没有给模型可靠写多行文件的通道**——argv 数组无 shell、无 stdin，文件正文必须塞进一个双重引用的参数；写 45 行 `cli.py` 用了 28 次调用、7 种 shell 技巧、0 次成功。次级观察：长载荷时 8/42 次调用 `argv` 变成 JSON 字符串（Keel 正确拒绝，模型误判原因；是模型生成还是 vLLM 工具解析器所致待判别）；4 个 run 中 3 个以空 assistant 消息静默结束，Keel 视为最终回答；一次 300 s 超时中止 run 且无重试；`ask` 模式 13 分钟 42 个提示、单个提示数 KB，人无法逐条审阅（操作员改用驾驭程序，本身即发现）；REPL 无多行输入。记忆工具未被使用也未被需要；未见上下文遗忘。候选机制（文件写/编辑工具、shell stdin、空最终回答的处理、超时策略、审批提示渲染、多行输入）已连同各自证据列出，**均未选择**；等待评审。

**L1-R1（2026-09-08，`docs/evidence/L1R1_2026-09-08.md`）：通过，验收 7/7。** 唯一变量是注册 `edit_file`（经 Gate B 停止、Gate C 停止、Gate D 10/10 与设计评审后实现，§5.10）。对照：调用 42→40；用于编辑的调用 28（0 成功）→11（10 成功、1 零匹配一步恢复）；畸形 8→2（性质不同：缺 `intent` 的 `python -m pira_ctx`）；模型调用 37→27；13.0→3.7 分钟；prompt 总量 697k→433k；无 shell 改文件尝试；无遗留文件。次级观察：`pira_ctx` 自动模式截断了测试输出（模型未用 `search`/`range`/`exact`）；工具描述"Every command runs through pira_ctx"可能诱导模型自行包裹；`ask` 模式 3.7 分钟 38 个提示仍由驾驭程序作答；模型删除了一个既有测试（验收未检查）。T19 未触发、仍推迟。**下一步由评审决定。**

**L1-R2（2026-09-08，`docs/evidence/L1R2_2026-09-08.md`）：通过，验收 7/7；full 模式。** 唯一变量 `ask`→`full`。11 次状态变更调用（全部 `edit_file`）全部带评审、先宣告后执行、决策先于结果，0 次缺评审拒绝，工作区内宿主审批 0，越界 0；41 次调用（shell 25、edit_file 11、read_pira_policy 5），畸形 0，30 次模型调用、2.7 分钟，prompt 最大 36.3k（5 次策略加载各带回数 KB），无遗留。次级诊断 `seed_test_preservation`：13/13 保全、新增 6（L1-R1 曾删 1）。**按预注册解读：full 模式对此工作负载是可行的自主路径；不推及 ask 模式审批体验已解决。** 观察：`Safety: Safety:` 双前缀（模型评审自带前缀，Keel 不改写）；`pira_ctx` 自动模式再次截断测试输出；缓存指标未采集（运行先于备注，无干净基线）。T19 与 shell 描述修正为随后的独立变更。

**L1 调查关闭（评审决定，2026-09-08）。** 受支持的结论，原文："On the frozen L1 workload with Qwen3.6, Keel full mode preserved task correctness (7/7), maintained the pre-execution handshake for all 11 state-changing edits, and required zero inside-workspace host approvals. Full mode is therefore a viable autonomous path for this workload." 不推广到其他工作负载或模型。不重跑 L1；不重开 stdin、`write_file`、`apply_patch`、`edit_file` 设计。L2 之前完成的三项独立整理（各自独立提交）：T19（空 assistant 回复为显式错误）；shell 工具描述明确 `pira_ctx` 包裹归 Keel（仅措辞）；`full_mode_check.py` 计数锚定修正（仅评估工具）。明确推迟：`Safety: Safety:` 显示、`pira_ctx` 自动摘要、审批批处理/UI、压缩、stdin、`write_file`、`apply_patch`、`edit_file` 语义、重试策略。**L2 协议的设计评审要求**（保留）：更难的真实编码工作负载而非文件传输微基准；既有种子测试成为正式保留检查（除非任务明确要求删除）；只在有干净前后计数器时采集运行局部的 vLLM 前缀缓存指标；在不改服务配置的前提下采集 TTFT（若可用）；prompt-token 工作量与 serving 缓存指标分开记录。

**L2 基准设计包（2026-09-09，`docs/dogfood/L2/README.md`；待评审，未运行）。** 仓库 `queuewatch`：从 JSONL 事件日志重建批作业状态并报告；标准库；8 个生产文件约 330 行（timeutil/events/model/state/report/cli/__init__/__main__），5 个测试文件 42 个测试；两份数据文件，其中 `production_2026-09.jsonl` 含 `retry` 事件而当前报错。冻结：种子提交 `5d668de`、树 `c050c5d`（两次独立 `init_l2.sh` 复现）、42 个测试 ID（`frozen/seed_test_ids.txt`）、任务文本（`frozen/task.md`）、遗留报告输出（`frozen/report_events_expected.txt`）。任务：加 `retry`（仅 failed 有效；attempt+1；回 queued 可再 started/completed 或 failed；其他状态下为无效转换，按既有方式报错无 traceback）；`report --status STATUS` 过滤行、汇总行仍计全部、未知 STATUS 为用法错误、无选项时输出不变；更新 README、加测试；既有测试不得删除或改名。隐藏验收 13 项：A01 项目测试通过；**A02 每个种子测试 ID 仍存在（正式门，按身份比对）**；A03–A13 覆盖 retry 接受/计数/再运行/拒绝、既有无效转换、`--status failed/completed/无匹配/未知`、无选项输出逐行等于冻结输出且 `check` 不变、README。参考验证：未改种子上 A03–A06、A08–A10、A13 失败（8 项；A06 三个子测试使 unittest 报 10 个失败），A01/A02/A07/A11/A12 在种子上本应通过且通过；作者参考解在临时副本上：种子套件 45/45、验收 13/13、保留 42/42 加 3；参考 diff 触及 6 个既有文件（59+/6−），全为普通编辑加测试运行，可用当前 Keel 工具集完成；参考解不在种子任何位置（`grep retry` 于 .py/.md 为空）。预注册配置：Keel 后 L1 基线（未改）、PIRA 规范基线、Qwen3.6、现有 vLLM 配置、full 模式、`--trace --record-wire`、默认 `max_turns`（不提高）；不加压缩/重试/工具/任何提示。预注册证据：主指标为隐藏验收与种子测试保留；运行证据清单同 L1-R2 并加 PIRA 策略加载、shell 错误、零/多匹配恢复；serving 指标只在有干净前后快照且无其他流量时计算运行局部前缀缓存命中率与 TTFT（`tools/metrics_delta.py`），与 prompt-token 工作量分列。解读规则：具体失败 → 冻结证据、先分类主导失败再提机制、停止；完全通过 → 记录通过与摩擦、停止评审。**评审（2026-09-09）：条件通过，两项冻结修正已完成**——retry 回 queued 时 duration 重置为 `-` 写入任务文本与 A03（可观察契约）；运行基线冻结为确定值（Keel 运行时最后变更 `ba47934`、PIRA `4e0682d`、Qwen3.6、vLLM 0.26.0、full、`--trace --record-wire`、`max_turns` 32），实验机任一项不符即在发送任务前停下。修正后重跑种子基线检查：种子哈希不变、42/42、参考解 45/45 与 13/13。**批准按协议运行一次 L2**；运行后立即冻结证据并停止，不改任务、验收或计分。

**L2 结果（2026-09-09，`docs/evidence/L2_2026-09-09.md`）：通过。** 隐藏验收 13/13；种子测试 42/42 按身份保全、新增 12；full 模式握手 18 次提议状态变更中 16 次带评审先公告后执行、1 次缺评审被拒并一步恢复、1 次结构校验拒绝；宿主审批 0、越界 0、顺序违规 0。86 次工具调用（shell 63、edit_file 18、read_pira_policy 5），edit_file 15 成功 3 错误（各一步恢复，无多匹配），畸形调用 2（argv 为字符串，立即改正），模型调用 76，6.4 分钟，上下文 6.5k→56.8k，prompt 总量 2.75M；serving 指标基线干净（TTFT 计数增量 76 = 请求数），运行局部前缀缓存命中率 0.878、平均 TTFT 1.311 s，与 prompt-token 分列。**主导摩擦：默认 `max_turns = 32` 熔断两次，任务靠 2/3 次 `Continue` 才完成；上下文远未到窗口上限，熔断是绑定限制**——协议预注册的"熔断成为失败模式"证据。次级摩擦：`pira_ctx` 自动摘要截断测试输出致大量单测探查；大文件重复读取；模型自述测试数错误（33 对 54）；Windows 命令摩擦。按解读规则：记录通过与摩擦，停下评审，未加机制。**评审（2026-09-09）：通过，但定性为"协议内成功"而非"默认预算下无人工续接的自主完成"。** 证据勘误四项已做（8 个文件为 4 源 + 3 测试 + README；edit_file 计为 18 提议、2 执行前拒绝、16 执行、15 成功、1 零匹配，零匹配经重读后恢复；摘要省略与额外探查只能定性为"可能放大调用需求的交互摩擦"；缓存命中率 0.878 曾因脚本把普通与 `external_*`、`*_created` 混合求和而降为待复核值；`metrics_delta.py` 改为按精确指标名分列后，实验机用既有快照复算：查询增量 2,749,109、命中率 0.878、TTFT 76 次均值 1.311 s 均不变，`external_*` 全程为 0，派生值保留）。**T22 已实现（`822fdbc`）。L2-R1 协议冻结（`docs/dogfood/L2/README.md`）：同一冻结任务，唯一变量为显式预算 `--max-turns 100`，零 Continue，耗尽即记录结果、不临时追加；预算依据固定为声明式授权（"本次实验预先授权单条任务最多 100 次模型调用；这是实验资源额度，不是完成所需调用数的预测，也不保证时间或上下文上限"），并如实说明 100 非独立于 L2 观察的盲选值、不是拟合出的成功阈值、后续单次成功不解释为预算最优。运行仍需另行批准。**

**L2-R1 结果（2026-09-09，`docs/evidence/L2R1_2026-09-09.md`）：失败。** 预检八项全部相符，启动横幅 `[max_turns] 100 per user message`。隐藏验收 10/13（A01 项目测试 6 失败 3 错误；A10 `--status` 无匹配时空列表被 falsy 判断吞掉过滤；A13 README 未动）；种子 42/42 保全、新增 15；遗留 `chk.py`。**会话不是被预算终止的**：第 40 次模型调用（预算 100）返回空响应，`LoopError::EmptyAssistantResponse`（T19）按设计显式报错、转录保留、不重发；零 Continue 规则下这即为结果。握手本身成立：15 次带评审状态变更先公告后执行，3 次缺评审被拒（L2 为 1）、2 次恢复、第 3 次紧接空响应。40 次模型调用、55 次工具调用（shell 40、edit_file 15）、5.4 分钟、上下文至 36.0k；第 31 次 completion 8,742 token、第 40 次 3,298；缓存命中率 0.837、TTFT 1.036 s，基线干净。`session_stats.py` 按 T22 规则如实报"至少 0，计数不完整"。**主导失败分类：终止性失败是模型的空回复（T19 现象：L1 四个 run 中三个、L2 零次、L2-R1 一次且致命）；预算不是绑定限制**；次级失败均为模型侧且处于提前终止的下游（README 未到、falsy 空列表、测试代码结构错误与期望错误、Windows shell 引号拼脚本）。wire 字段判别（同日）：第 40 次 `finish_reason: stop`、`content: ''`、`tool_calls: 0`，隐藏 reasoning 9,967 字符且以 `</parameter></function></tool_call>` 结尾——**模型把完整工具调用写进了推理通道，服务端解析器未提取**；未触长度上限（第 31 次 8,742 completion 仍正常产出调用）、推理以完整句子结束非截断、丢失的正是刚被拒的 `edit_file` 重发。**评审（同日）收紧措辞**：闭合标签结尾不等于生成了完整合法的调用（还需起始标签、工具名、参数结构）；即便完整调用出现在 reasoning 中，也可能是模型输出通道错误、模板/reasoning parser/tool parser 组合不匹配或解析器缺陷之一，责任待隔离；复发风险未知（单次不成样本）。终止性失败暂定为**模型/serving 边界的工具协议失败**，不在 Keel 的握手、预算或循环；Keel 各步正确。过滤 bug 与错误测试在终止前已存在，是被提前终止留下而非造成。L1 的三次空回复是否同因当时未查 wire，作为后续假设。事实记录不行动：响应体本身含可区分此情形的信息（以标签结尾的 reasoning），而 `from_wire` 只读 `content` 与 `tool_calls`。按解读规则：冻结证据、先分类、停下评审；未选机制。

**工具协议审计（2026-09-09，`docs/evidence/TOOL_PROTOCOL_AUDIT_2026-09-09.md`）：完成，分类定稿为模型输出不合规。** 两份 wire 共 77 次响应，reasoning 含工具标记的 5 次恰为四次空回复加一次成功对照；四次空回复签名一致（`stop`、无调用、空 content、闭合标签在、开标签与 `</think>` 无）。服务主机侧（实验机只读完成）：部署使用模型自带模板与模型卡规定的 `qwen3`/`qwen3_xml` 解析器，配置不匹配排除；vLLM 0.26.0 非流式路径单遍扫描全文、无迁移出路保留原文、槽位只由 `TOOL_CALL_START` 创建，缺失的开标签从未生成，解析器缺陷排除。L2-R1 报告已勘误：第 40 次不是被拒编辑的逐字节重发而是同一修复线程的下一次编辑。Keel 各步正确，不改 Keel；实验机的 Keel 侧重试建议记为 T23 待评审事项而不实现；L2-R2 未批准；复发风险未知。
