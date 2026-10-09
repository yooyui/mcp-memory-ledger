# MCP Memory Ledger 项目说明

## 简述

MCP Memory Ledger 是一个 Rust 编写的本机 MCP `stdio` memory demo，用于验证 AI 客户端里的长期记忆 / 自我快照 / 反思修订最小闭环。当前版本以 SQLite 为持久化基础，更适合作为技术 demo、集成验证仓库和研究型原型，而不是完整产品。

兼容说明：当前 Rust crate、二进制、脚本、配置样例和部分历史文档仍使用技术标识 `agent_llm_mm` / `agent-llm-mm`。对外项目名统一使用 MCP Memory Ledger。

## 当前范围

- 本机 `stdio` MCP 服务
- SQLite 持久化
- 配置文件驱动的 provider 加载
- `openai-compatible` / OpenRouter provider
- `ingest_interaction`
- `search_memory` / `get_memory` / `get_reflection_history` / `get_self_model_history` / `get_evidence_relation` / `supersede_memory`
- `build_self_snapshot`
- `decide_with_snapshot`
- `run_reflection`
- `doctor` / `serve` 启动入口

## 当前边界

- M1 已完成十六片到 M1.2.7：四类 scoped search/lookup、union、Claim history、self-model audit、evidence-relation runtime 与 scoped Claim supersede；下一片是 M1.3.0 current-schema structural readback
- `decide_with_snapshot` 已可走 `openai-compatible` 或 OpenRouter provider，但返回契约仍是最小动作字符串
- 没有远程 HTTP transport
- `get_evidence_relation` 已有 scoped runtime 首片，仍不是 richer ranking / weighting
- 还没有 Azure / 本地模型 provider；OpenRouter 通过 OpenAI-compatible transport 使用，显式 live runner 只生成 provider preflight evidence，不是 provider 质量认证、SLA 或 gateway 认证
- release evidence index、provider certification preflight、packaging preflight 和 richer memory semantics projection 均为本机只读 / 预检能力，不生成缺失产品证据、不认证 provider 质量、不创建安装包，也不代表完整多层 memory 体系

## 适合的使用方式

- 本机 AI 客户端接入实验
- self-agent memory 相关概念验证
- Rust + MCP + SQLite 的最小工程骨架参考

## 文档约束

- 每次处理完一个任务后，如果该任务影响了行为、能力边界、接入方式、配置、验证命令或协作规则，必须同步更新对应文档。
- 不应把文档更新留到最后统一处理；代码与文档应尽量在同一轮任务内一起收口。
- [正式化改进计划](formalization-improvement-plan-2026-08-25.md)只负责差距与验收映射，不是第二份执行队列。当前追平阶段统一通过既有 `dev-work -> main` PR 集成；进入 `main` 前必须有 fresh CI 和明确的人工 review。

## 当前验证状态

截至 `2026-08-25`：

- 测试分为 `fast` / `core` / `full` 三级；默认 core 不编译发布工具链
- `release-tools` feature 保留完整发布证据、打包与 provider certification 验证
- MCP `stdio` 当前暴露 10 个工具；`run_reflection` 仍是 identity / commitment / reflection 的唯一 durable write path
- M0 已收口；M1 进行中，M1.2.7 已完成，下一领取顺序是 M1.3.0
- fresh 本地 CI 等价门禁已通过 format、全目标/全特性 Clippy、full tier、status sync 与 diff check；远端 PR checks 仍是独立门禁
- `doctor` 返回 `status = ok`

## 致谢

本仓库在开发、讨论和文档整理过程中明确使用了 OpenAI Codex 作为协作式开发工具。感谢 OpenAI 提供相关工具与研究生态，使这种以讨论驱动、迭代收口的开发方式成为可能。
