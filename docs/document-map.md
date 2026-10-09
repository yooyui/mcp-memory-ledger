# 文档总览

本页只保留当前仍有执行或使用价值的入口。历史 specs、旧计划、阶段快照和发布记录不再与当前主线并列展示，统一见[历史归档](archive.md)。

## 1. 五分钟理解项目

主线说明只由以下五个入口组成，按顺序阅读即可理解项目：

1. [README](../README.md)：项目是什么、能做什么、如何启动以及当前边界。
2. [项目起点与主线原则](origin-and-principles.md)：为什么做、哪些原则不能丢、什么功能不应进入主线。
3. [当前实现状态](project-status.md)：implemented / partial / unimplemented 的事实。
4. [Now / Next / Later 路线图](roadmap.md)：阶段顺序。
5. [唯一 active plan](plans/2026-07-10-product-replan.md)：当前任务、证据门与停止条件。

README 负责公共入口，起点与原则负责方向约束，状态负责当前事实，路线图负责阶段顺序，active plan 负责任务执行。需要判断完成证据时，再查阅 [reality gates](product/follow-up-reality-gates.md)。不要从历史计划中的 checkbox 领取任务。

## 2. 项目身份与公共入口

- [positioning.md](positioning.md)：公共名称、目标用户和对外口径。
- [FAQ](faq.md)：常见问题与保守回答。
- [English README](../README.md)：GitHub 默认首页。
- [中文 README](README.zh-CN.md)：简体中文入口。
- [日本語 README](README.ja.md)：日文入口。
- [英文项目说明](project-overview.en.md)
- [中文项目说明](project-overview.zh-CN.md)
- [日文项目说明](project-overview.ja.md)

公共名称使用 **MCP Memory Ledger**；`agent_llm_mm` / `agent-llm-mm` 只作为当前 crate、binary、脚本与配置兼容标识。

## 3. 使用与开发

- [macOS 开发说明](development-macos.md)
- [Windows 开发说明](development-windows.md)
- [本机 MCP 接入](local-mcp-integration-2026-03-26.md)
- [测试指南](testing-guide-2026-03-24.md)：`fast` / `core` / `full` 分层命令和按改动类型选择验证范围。
- [Rust Toolchain Policy](toolchain-policy.md)：Rust `1.95.0` 固定版本、支持下限和升级门禁。
- [Provider contract](provider-contract.md)
- [Self-revision demo guide](self-revision-demo-guide-2026-04-24.md)

这些文件是按需使用的操作手册，不负责定义项目定位或当前任务。默认工作流仍是本地 MCP `stdio`。开发、测试和正式数据应使用不同的显式 `database_url`。

## 4. 当前数据与能力边界

### 核心产品边界

- [Local Alpha PRD](product/prd-local-alpha.md)
- [Data lifecycle](product/data-lifecycle.md)
- [Data safety](product/data-safety-local-alpha.md)
- [Structured decision protocol](product/structured-decision-protocol.md)
- [Reality gates](product/follow-up-reality-gates.md)
- [`rmcp 0.5` → `2.2.0` compatibility spike](spikes/rmcp-compatibility-2026-07-14.md)：独立迁移破坏面、no-go 结论与测试矩阵。

### 本地运维与可观测性

- [Support bundle boundary](product/support-bundle-local-alpha.md)
- [Correlation ID contract](product/correlation-id-contract.md)
- [Daemon observe-only gate](product/daemon-observe-only-gate.md)

### 暂不进入主线的方向

- [Memory layering roadmap](product/memory-layering-roadmap.md)
- [Remote / team mode boundary](product/remote-team-mode-boundary.md)
- [Threat model](security/threat-model-local-and-remote.md)

这些文件描述边界或未来条件，不代表能力已经实现，也不构成当前任务队列。

## 5. 发布与验证

- [正式化改进与主线同步计划](formalization-improvement-plan-2026-08-25.md)：从 technical MVP 到可交付 Local Product Alpha 的差距矩阵、验收清单和 GitHub 主线同步路径；不是第二份任务队列。
- [MVP release gate](release-gate.md)
- [Local Alpha release gate](product/release-gate-local-alpha.md)
- [Release engineering](product/release-engineering.md)
- [Release readiness](release-readiness.md)

发布、provider 和 packaging preflight 是辅助门禁，不是项目北极星，也不能替代 scoped recall、provenance 或 correction 闭环。

## 6. 文档职责与证据权威

| 文档 | 回答的问题 |
| --- | --- |
| [origin-and-principles.md](origin-and-principles.md) | 这件事为什么值得做，什么不能偏离？ |
| [project-status.md](project-status.md) | 当前代码实际上有什么？ |
| [roadmap.md](roadmap.md) | 先做什么、后做什么？ |
| [plans/2026-07-10-product-replan.md](plans/2026-07-10-product-replan.md) | 当前允许执行哪个切片？ |
| [product/follow-up-reality-gates.md](product/follow-up-reality-gates.md) | 哪些说法有证据，哪些仍被阻断？ |
| [formalization-improvement-plan-2026-08-25.md](formalization-improvement-plan-2026-08-25.md) | 正式化还缺哪些产品、工程、安全、发布和主线治理条件？ |

冲突时先以代码、测试和数据库 readback 为准，再修正文档；不得用旧计划覆盖当前事实。

## 7. 历史归档

整理前完整基线保存在本地分支：

```text
codex/archive/pre-mainline-reset-2026-07-10
```

[archive.md](archive.md) 记录了归档范围、基线 commit、查阅和精确恢复方法。归档内容包括：

- 原始逐轮对话日志；
- `docs/superpowers/` 历史 specs 和 plans；
- 阶段工作快照与实现对照；
- 旧 productization / P1-P2-P3 记录；
- 历史 release note、demo report、改名与发布准备材料。

这些内容仍可追溯，但不再占据当前文档导航，也不再拥有执行权。
