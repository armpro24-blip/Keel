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
| **semantic safety review**（破坏性风险判断、`Safety:` 评审） | PIRA | 模型按 PIRA 文本执行；Keel 不解析、不声称强制 |
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
- `full` 模式下语义安全评审（`Safety:`）归 PIRA；Keel **不**做 `Safety:` 字符串存在性检查，因为前缀存在不构成有意义的强制。若日后证据要求更强强制，设计真正的结构化运行时风险检查，而非表面文本校验。
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
| **M4 — 显式压缩** | `/compact` + 压缩通知 + recap 路径；最简压缩算法 | 自动压缩 | 观察到上下文溢出 | 压缩后模型用 `pira_ctx recap` 续接 |
| **未排期** | 文件编辑工具（若 shell 编辑被观察到不可靠）、MCP 客户端、Skills、subagents、项目级 `AGENTS.md` 发现、流式、resume、`pira_dir` 重定位、Cargo workspace、TUI | | 各自需要观察到的问题 | |

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
- **T15 状态：CAUSE ISOLATED = Case A（2026-09-07 审计，`docs/evidence/T15_AUDIT_2026-09-07.md`）。** 送达与渲染均正确；孤立短规则 5/5 遵守，`AGENTS.md`、+host block、Keel 完整请求三层均 0/5；无隐藏通道评审；工具 schema 无干扰。结论：对该模型-serving 配置，规则在完整策略文本中的显著性不足，不是送达缺陷，也不是"模型不能遵守"。关键旁证：同一上下文里模型 4/5 采纳了 shell 工具描述里的写法建议，却 0/5 应用 20 KB 之前的系统规则——位置是操作变量。**待用户决策**：a) Keel 在 shell 工具描述中加一句指向 PIRA 规则的指针（不复述规则内容，仅指明触发条件与位置；属 §4.3-2 允许的工具 schema 适配），先做 Layer 3 变体对照实验（5 次，对照组为不变的 Layer 3）；b) 仅把证据反馈给 PIRA 作者，由策略文本侧解决；c) 两者都做。建议 c，且 a 只在对照实验显示效果后采纳。次要发现：Windows 检出的 `AGENTS.md` 为 CRLF（`core.autocrlf`），Keel 原样发送，跨机器文件哈希不可比，仅 `source_commit` 可比（T17）；chat template 在提示末尾附加 `reasoning_effort: none`，属 serving 配置变量。
- **T15（历史）状态：CAUSE NOT YET ISOLATED（2026-09-07 修订）。** 观察：mistral-small-4-119b 在 full 模式下 7 次相关状态变更尝试、0 次 `Safety:`。按"Own the instruction path"，在完成 instruction-path audit 之前不归因于模型，也不因此建 RiskClassifier。审计只查四件事（协议 `docs/AUDIT_T15.md`）：(1) 用 `--record-wire` 捕获实际发往 vLLM 的 `system + messages + tools`，确认 Full-Permission Behavior 原文在内且 host block 明确写着 full-permission/no-approval；(2) 用 vLLM 的 `/tokenize` 检查 chat template 渲染后 system 角色是否完整保留、未被弱化或截断；(3) 最小隔离探针：仅一条短 system 指令要求 `Safety:`，让模型创建文件；(4) 逐层恢复 PIRA 上下文（最小规则 → 完整 AGENTS.md → +host block → +完整工具 schema），看行为在哪一层变化。结论按差分结果归入：指令显著性/上下文整合问题（PIRA/Keel 呈现方式）、ModelAdapter/serving 缺陷、工具协议干扰、或"模型-serving 配置"的能力边界（不说"模型"，因为权重、chat template、tool parser、解码参数与服务端设置共同决定行为）。每个条件重复 ≥5 次，与 `docs/AUDIT_T15.md` 一致；步骤 3/4 保存完整原始响应，摘要字段不替代原始体。已做的宿主侧修正：host block 的 full 模式措辞改用 PIRA 自己的术语 "full-permission/no-approval mode"，消除词汇不一致这一可能原因。
- **T15（原记录，保留为历史）full 模式下 `Safety:` 评审的实证缺席**：M2 C 冒烟（`docs/evidence/M2C_SMOKE_2026-09-07.md`）中，mistral-small-4-119b 在 full 模式下进行了 6 次写入尝试与 1 次删除，一次都没有打印 PIRA 要求的 `Safety:` 评审，尽管它在 M2 B 里能正确复述这条规则。这证明了"PIRA 语义安全 ≠ 运行时强制的安全"，但没有证明"Keel 应当自己判断命令语义"：一个 RiskClassifier（`cargo test` 改不改 `target/`？`python script.py` 做什么？）很可能成为看似有强制、实则判断不可靑的子系统，比明确承认 `--full` 信任模型更危险。**决策（用户，2026-09-07）**：full 模式下语义安全仍归模型/PIRA；`ask` 是默认，也是模型合规尚未被证明时的推荐模式；不基于单一模型的证据添加命令风险分类器；仅在更广泛的模型证据或出现 `ask` 无法可接受地控制的具体失败后重新审视。`--full` 的帮助文本与 host block 措辞如实描述这一保证边界。证据的适用范围限定为"本次测试的模型与运行"，不推断"本地模型 vs 闭源模型"的一般结论；其他模型跑同一探针后才开始形成模型侧证据。
- **T16 `argv` 中的 shell 操作符**：同一次冒烟里模型三次把 `>` 作为独立 `argv` 元素传给 `echo`/`printf`，命令"成功"（exit 0）却没有任何效果，直到熔断。据此 Keel 在解析时拒绝独立出现的 shell 操作符并给出本平台的 shell 请求形式（不解释、只拒绝，O3 不变）；工具描述改为平台感知。`max_turns` 从 8 提到 32。这两处是"观察到需要"驱动的改动。
- **T17 行尾与哈希可比性**：PIRA 仓库存 LF，但 Windows 上 `core.autocrlf=true` 的检出把 `AGENTS.md` 变成 CRLF；Keel 按"原样字节"发送并哈希，因此同一 PIRA commit 在 Windows 与 Linux 上的 `pira.lock` 文件哈希不同，也多出约 243 个换行 token。单机内一致性不受影响；跨机器只能比 `source_commit`。Keel 不做行尾归一化（保持"发送安装的确切字节"），可向 PIRA 作者建议在仓库加 `.gitattributes` 固定 LF。
- **T12 大小写**：路径分量只在 Windows 上做大小写不敏感比较；macOS 默认文件系统同样不区分大小写但未处理。影响限于边界误判为 Outside（偏保守），非安全问题。

## 9. 可测试不变量

M0 起：
1. loop 在 `max_turns` 内终止，或以显式错误结束。
2. 同一 assistant 轮内的多个 ToolCall 按声明顺序执行，结果按同一顺序返回。

M2 起：
3. 非 PIRA 内部工具的每次 shell 执行都经由 `pira_ctx`（对子进程 argv 断言）；内部工具判定对 basename/路径/`.exe` 形态一致。
4. 每个子进程环境含 `PIRA_CTX_THREAD_ID`，且同一 Keel 会话内值不变。
5. 系统指令中的 `AGENTS.md` 段与 `~/agent/AGENTS.md` 字节相同（哈希）。
6. loader 只加载路由表声明的文件与 `USER.md`；返回内容与文件字节相同；loader 从不启动 `pira_ctx`。
7. 策略加载不产生审批请求；每个动作类工具调用都有权限决策记录。
8. 工作区外（非 temp）路径的工具调用在任何模式下都经用户确认。
9. `pira.lock` 三态判定：哈希不一致且契约通过 ⇒ UNVERIFIED-COMPATIBLE 并告警；契约失败 ⇒ INCOMPATIBLE 且不静默继续。

M4 起：
10. 压缩只由用户命令触发。
