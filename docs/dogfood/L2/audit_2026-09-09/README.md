# 工具协议审计 — 实验机交付包（2026-09-09）

来源：服务主机 192.168.3.103 上的 vLLM 运行容器（只读），
以及实验机本地的 L1 / L2-R1 wire 结构化复核。

## 内容

- `tool_protocol_audit_step3.md` — 步骤 3 结论全文。可直接并入
  `docs/evidence/TOOL_PROTOCOL_AUDIT_2026-09-09.md`。
- `step1b_output.txt` — 步骤 1b 实测输出（`probes/step1b.sh` 在运行容器内的输出，
  前置主机侧 docker ps / inspect 结果；`HF_TOKEN` 值已由实验机操作员抹除）。
- `step1b_output_fix.txt` — 1b.4 对 Qwen3.6 权重目录的补充读取
  （`generation_config.json` 正文、chat_template 的 `<think>` 检查、特殊 token 定义）。
- `probes/` — 本次使用的全部只读探针脚本，供复现与审阅。
  - `probe4.sh` 状态机与适配器
  - `probe5.sh` 引擎的 reasoning / tool 提取入口
  - `probe6.sh` 非流式 `parse()` 链路与抑制开关
  - `probe7.sh` 终结符分派与提取结果构造
  - `step1b.sh` 步骤 1b 记录（版本、启动参数、解析器模块、权重 sha256），
    并打印步骤 3 附录 B 三处待回填的行号区间

## 执行方式

全部脚本经管道送入运行容器，主机与容器内均未落盘文件，未修改任何配置，
未向服务发起任何推理请求：

    ssh <host> "docker exec -i <container> bash -s" < probes/<script>.sh

## 未包含（按协议保留在实验机本地）

wire 文件、全部原始响应正文、会话记录与 log show 输出、
gate_b/c/d 的原始响应、L1/L2 工作区仓库。
步骤 3 文稿所引正文限于判定所必需的最小片段。

## 状态

步骤 1b 与步骤 3 均已完成，附录 B 的版本、路径、行号已按实测回填。
步骤 1a 按协议由 Keel 作者侧完成。

---

Keel-side note (added when the pack was committed): this directory is the
lab's delivery, stored verbatim as evidence. The Keel author's reading of it
is in `docs/evidence/TOOL_PROTOCOL_AUDIT_2026-09-09.md`.
