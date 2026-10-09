# 本机 MCP 接入说明（2026-03-26，按 2026-08-09 scoped Episode read 更新）

## 1. 目标

把 `agent_llm_mm` 以本机 `stdio` MCP 服务的方式接入 Codex 等 AI 客户端，并保证：

- 启动路径稳定
- SQLite 落盘路径可控
- 有独立的预检命令
- 能清楚区分“可正式嵌入的能力”和“仍属 mock/实验的能力”
- 对 automatic self-revision MVP 的入口范围和失败语义有准确预期

## 2. 推荐接入形态

macOS 下优先入口是 `scripts/agent-llm-mm.sh`。它提供 `init` / `migrate` / `doctor` / `serve` 封装，并贴合当前 zsh / bash 环境。

如果你想完全绕过脚本，也可直接运行 `cargo run --quiet --bin agent_llm_mm -- <serve|init|migrate|doctor>`。

原因：

- 可以从任意当前目录启动
- 可以固化项目根目录
- 可以统一显式数据库生命周期、只读诊断与 `serve` 模式
- 后续切换为预编译二进制时，客户端配置无需大改

入口脚本：

- [scripts/agent-llm-mm.sh](../scripts/agent-llm-mm.sh)

## 3. 本机预检

正式接入前，先执行：

```zsh
cd ~/code/agent-llm-mm
cp examples/agent-llm-mm.example.toml agent-llm-mm.local.toml
./scripts/agent-llm-mm.sh init
./scripts/agent-llm-mm.sh doctor --read-only
```

预期输出为 JSON，至少包含：

- `transport`
- `database_url`
- `database_lifecycle`
- `auto_reflection_runtime_hooks`
- `self_revision_write_path`
- `status`

当前 `status = "ok"` 代表：

- 配置已解析
- SQLite schema / ledger / foreign keys / runtime defaults 已通过只读检查
- provider 已按配置完成校验
- 当前数据库可由 `serve` 只读打开校验
- `doctor` 还能保守显示当前 MCP runtime hook coverage 和 durable write path

当前 `doctor` 输出里的 self-revision 相关字段，应按下面口径理解：

- `auto_reflection_runtime_hooks`
  - 只表示当前 MCP-wired automatic path 的文档化 runtime coverage
  - 当前准确值应为：
    - `ingest_interaction:failure`
    - `ingest_interaction:conflict`
    - `decide_with_snapshot:conflict`
    - `build_self_snapshot:periodic`
- `self_revision_write_path`
  - 当前准确值应为 `run_reflection`
  - 这表示 automatic self-revision 最终仍收口到既有 durable write path，而不是新增 MCP tool 或旁路持久化接口

这些字段不表示：

- 存在新的 auto-reflection MCP tool
- 存在后台 daemon / 定时自治进程
- 所有 MCP 请求都会自动反思

截至 `2026-07-10`，验证入口已减重为：

- `./scripts/test-tier.sh fast` 用于主线短反馈；`./scripts/test-tier.sh core` 覆盖默认运行时
- `./scripts/test-tier.sh full` 启用 `release-tools`，保留完整发布证据、打包与 provider certification 验证
- `./scripts/status-sync-check.sh` 只检查 active plan / reality gate 和仓库 fixture，不再编译整套测试清单
- self-revision demo package wrapper 可生成本地 artifact report
- `release-soak-local.sh` 已提供本地 release evidence runner；数据库写入强制落到 candidate-isolated runtime path，`release-boundaries.json` 明确记录不接受正式库路径。它不生成 Windows runner、真实 fresh-machine、remote/team、上传、tag、安装包或发布认证证据
- `product-readiness-check.sh` 已提供本地候选 readiness gate，能够把 release decision、release engineering、真实 fresh-machine、Windows parity、remote/team、安全/auth 和产品措辞缺口保持为 blocked
- `release-evidence-index.sh`、`provider-certification-check.sh` 和 `packaging-preflight-check.sh` 已提供候选 evidence 索引、provider live-certification 缺口预检和 packaging 缺口预检；这些脚本不新增 MCP tool、不调用 provider endpoint、不生成 installer、不上传文件，也不改变 `stdio` 接入契约；provider evidence 占位文件和零字节/部分 packaging archive 不会被当作完整证据

## 4. 启动服务

```zsh
cd ~/code/agent-llm-mm
./scripts/agent-llm-mm.sh serve
```

注意：

- 该命令会启动 MCP `stdio` 服务
- 终端看起来像“挂住”，这是正确行为
- 不应在服务运行期间往标准输入随意写普通文本

## 5. Codex 配置示例

你当前机器上的 Codex 配置格式已经在使用：

- `[mcp_servers.<name>]`
- `command`
- `args`
- `env`

可直接参考：

- [examples/codex-mcp-config.toml](../examples/codex-mcp-config.toml)

推荐做法：

- `command` 指向 `scripts/agent-llm-mm.sh`
- `args` 只传 `serve`
- `env` 显式传入 `AGENT_LLM_MM_CONFIG`

```toml
command = "/absolute/path/agent-llm-mm/scripts/agent-llm-mm.sh"
args = ["serve"]
```

如你不想经过脚本，也可直接用 `cargo run`：

```toml
command = "cargo"
args = ["run", "--quiet", "--bin", "agent_llm_mm", "--", "serve"]
```

## 6. 接入排障（与平台文档共用）

同类问题请先按对应平台文档执行 `doctor` 排查，核心症状与处理一致：

| Symptom | Likely Cause | Verification | Fix |
| --- | --- | --- | --- |
| `init` / `migrate` cannot write SQLite | database path not writable or sandbox restriction | 先运行 `doctor --read-only` 查看 `database_lifecycle`，再核对 config 与环境覆盖 | 仅为显式 lifecycle command 选择可写路径；不要为只读 doctor 放宽正式库权限 |
| `serve` reports database not ready | missing / stale schema / incomplete defaults | 查看 `doctor --read-only` 的 lifecycle status | missing 用 `init`，旧库用 `migrate`；只有明确允许写入时才用 `doctor --allow-bootstrap` |
| MCP client starts the wrong binary | auxiliary `src/bin` target ambiguity | 检查客户端是否显式传递 `--bin agent_llm_mm` | 优先使用脚本入口（`agent-llm-mm.sh` / `agent-llm-mm.ps1`），或固定 `--bin agent_llm_mm` |
| dashboard not visible | `[dashboard].enabled = false` 或端口占用 | 查看配置和 `doctor` 输出 | 将 `enabled` 设为 `true`，并选择可用 localhost 端口 |
| model calls fail | provider 配置不完整 | `doctor` 中确认 `provider`、`base_url`、`model` | 补齐本地 TOML 的 provider 配置（仅本地文件，勿提交 API key） |

平台细节请参考：

- [docs/development-macos.md](./development-macos.md)（平台版排障矩阵）
- [docs/development-windows.md](./development-windows.md)（平台版排障矩阵）

## 7. 当前能力状态

### 已实现

- `ingest_interaction`
- `search_memory`（M1.1.1 Event、M1.1.2 Claim、M1.1.3 Episode、M1.1.4 Reflection provenance recall 与 M1.1.6 跨类型 union 首片；显式 `namespace` 必填，provider 离线可用；省略 `record_type` / `record_types` 时仍为 Event）
- `get_memory`（M1.2.1 Event + M1.2.2 Claim + M1.2.4 Episode + M1.2.5 Reflection lookup 首片；显式 `namespace` + stable `id` 必填；省略 `record_type` 保持 Event，其余类型要求显式 `record_type`；跨 scope / record-only Reflection 返回 `record: null`）
- `get_reflection_history`（M1.2.3 exact scoped Claim revision chain 首片；显式 `namespace` + `claim_reference` 必填，limit 默认 20、范围 `1..=100`）
- `get_self_model_history`（M1.2.6 scoped identity/commitment revision audit 首片；显式 `namespace` + `history_type` 必填，limit 默认 20、范围 `1..=100`）
- `get_evidence_relation`（M1.1.5 scoped evidence-relation runtime 首片；显式 `namespace` + `trigger_window_event_ids` 必填，`selected_evidence_event_ids` / `selection_basis` 可选）
- `supersede_memory`（M1.2.7 scoped Claim audited supersede 首片；显式 `namespace` + `claim_reference` + `replacement_claim` + `replacement_evidence_event_ids` + `summary` 必填；复用 `run_reflection`，默认不 hard delete）
- `build_self_snapshot`
- `run_reflection`
- `doctor` / `serve`
- SQLite 持久化
- `namespace` 最小闭环
- `openai-compatible` provider
- OpenRouter provider（通过 OpenAI-compatible `/chat/completions` transport；配置示例和本地 stub 不是 live evidence，显式 `--live` runner 才能生成 bounded live preflight evidence）
- 配置文件驱动的 provider 选择
- `search_memory` 的 Event 路径支持 exact event reference、event kind、inclusive RFC3339 time window 和 `1..=100` limit；返回 canonical event ID、scope、时间、摘要和 claim/episode provenance。
- additive `record_type = Claim` 路径支持 canonical/raw `claim_reference`、`claim_status`、`mode` 和 `1..=100` limit；省略 `claim_status` 时默认 `Active`。结果包含 canonical `claim:<id>`、subject/predicate/object、mode/status、canonical evidence event references、episode references 和直接 source/superseded reflection links。claims 没有 stored `recorded_at`，因此该路径拒绝 event reference、event kind 与时间过滤。
- additive `record_type = Episode` 路径支持 exact opaque `episode_reference` 与 `1..=100` limit。SQLite 先通过同 scope Event membership 收窄，再按最新 scoped Event 元组稳定排序；结果原样返回持久化 Episode reference，并包含由同 scope 数据证明的 canonical Event/Claim provenance。该首片不规范化 `episode:` 前缀，也不返回未持久化的 objective/outcome/lesson。
- additive `record_type = Reflection` 路径支持 exact persisted `reflection_reference` 与 `1..=100` limit。一条 Reflection 只在 superseded Claim 属于请求 scope、且 replacement 为空或同 scope 时可见；record-only 行没有 Claim anchor，不能推断 namespace。mixed-scope edge 整条隐藏。`get_memory` 已接受 scoped Reflection lookup。
- 四种 `search_memory` 路径都先在 SQLite 按 server-derived owner + namespace 或等价 scope attribution 收窄，再应用类型专属 filter/limit；它们只读、provider-free，跨 scope exact reference 返回空结果。additive `record_types` 可在同一 scope 内合并这些已有 tagged record，并按 recorded_at / type / id 稳定收口；union 拒绝类型专属 filter。当前仍不是完整 versioned ledger / correction 合同。
- `get_memory` 复用相同 scoped read service 返回单条 Event、Claim、Episode 或 scoped Reflection。省略 `record_type` 时保持原 Event 语义，包括 raw Event ID；Claim 要求显式 `record_type = Claim`，并接受 canonical/raw Claim ID，从而避免 `claim:*` raw Event ID 的判型歧义。精确 Claim lookup 不套用 search 的默认 Active 过滤，因此 Active、Disputed、Superseded 都可按 ID 返回。Episode / Reflection 要求显式类型，并把 `id` 当作 opaque exact persisted reference。Reflection lookup 复用 M1.1.4 归属：record-only / missing / cross-scope 都返回 `record: null`。它不提供 unscoped existence probe，也不代表 versioned identity/commitment ledger 或 record-only history 已完成。
- `get_reflection_history` 接受 canonical/raw Claim ID，以 exact scoped Claim 为锚点递归读取 superseded/replacement 双向链，并按 newest-first 返回 reflection ID、时间、摘要、canonical superseded/replacement Claim references 与同 scope canonical evidence references。missing/cross-scope anchor 返回空，mixed-scope edge 整条隐藏；读取只读且不依赖 provider，operation metadata 仅含 `history_type`、`result_count`、`has_more`。
- `get_self_model_history` 按 `history_type = Identity` 或 `Commitment` 读取现有 reflection 审计列，只返回能通过同 scope Claim 归属的 newest-first 修订。record-only 更新与 mixed-scope edge 保持不可见。limit 默认 20、范围 `1..=100`，并用 `has_more` 表示截断。该首片不是 versioned identity/commitment ledger，也不提供 rollback。
- `get_evidence_relation` 接受裸 ID 或 `event:<id>` 组成的 trigger window，先保序去重再与请求 owner+namespace 做 intersect-only 收窄。missing / cross-scope trigger ID 从窗口省略；selected 必须是 scoped window 的子集，否则 fail closed。结果返回 canonical `event:<id>`、window_rank、selected / available-not-selected、binary weight 与 rejection reason。路径只读、provider-free，operation metadata 仅含 `report_type`、`trigger_window_size`、`selected_count`、`result_count`。它不引入 ranking 或 widening。
- `supersede_memory` 接受 canonical/raw Claim ID 与至少一条 evidence，先确认 target 与 evidence 都在请求 namespace，再调用既有 `run_reflection` 事务。旧 Claim 变为 `Superseded`，历史接口仍可回看；missing / cross-scope 输入 fail closed。operation metadata 仅含 `correction_type` 与 `durable_write_path = run_reflection`。它不是第二条 durable write path，也不覆盖 identity/commitment 或 Event/Episode/Reflection 纠错。
- Episode / Reflection lookup、Claim history、self-model audit 与 Claim supersede 首片都不覆盖 versioned identity/commitment ledger 或 record-only reflection history。Episode reference 仍是 opaque persisted string；这些切片都没有 schema migration/index，保留 technical-MVP 表扫描性能边界。
- trigger-ledger-backed automatic self-revision MVP
  - 当前 MCP-wired automatic path 只有 4 条：
    - `ingest_interaction -> failure`
    - `ingest_interaction -> conflict`
    - `decide_with_snapshot -> conflict`
    - `build_self_snapshot -> periodic`
  - `ingest_interaction` 仍可通过 ingest DTO 提供 `trigger_hints`
  - `ingest_interaction -> conflict` 仍要求显式 `trigger_hints` 包含 `conflict` 或 `identity`；无 hints 或非兼容 hints（例如只有 `commitment`）不会因为文本看起来冲突就启动该 hook
- `decide_with_snapshot` / `build_self_snapshot` 当前仍要求显式传 `auto_reflect_namespace`
- `decide_with_snapshot` 还要求显式传 conflict-compatible `trigger_hints`，否则不会因为“库里已有 evidence”而自动进入 conflict self-revision
  - proposal 首阶段已可携带 `proposed_evidence_event_ids`、`proposed_evidence_query`、`confidence`；其中 query 在 explicit ids 为空时可对当前 trigger window 做 bounded narrowing，并在有交集时只按当前窗口内候选应用 `limit`，若没有交集则拒绝处理而不是绕过 query；在 explicit ids 非空时也会约束这些 ids 必须满足当前窗口内的 query 过滤条件，但这仍不是 richer widening / ranking engine
  - best-effort auto-reflection 现在会返回 structured diagnostics，包含 `trigger_type`、`namespace`、`trigger_key`、outcome、rejection / suppression reason、`cooldown_state`、cooldown boundary、evidence window size 与 selected evidence ids，供日志与测试复用
  - proposal 会经过服务端治理，再转译到既有 `run_reflection`
  - 没有新增单独 MCP tool；identity / commitments 的 durable write path 仍是 `run_reflection`
  - direct `run_reflection` 不会递归进入 auto-reflection

### 部分实现

- `decide_with_snapshot`
- automatic self-revision 的 runtime coverage

原因：

- commitment gate 是真实能力
- 下游模型已可走 `openai-compatible` 或 OpenRouter；配置示例和本地 stub 不是 live evidence，显式 `--live` runner 只生成 bounded live preflight evidence，不是 provider 质量、SLA 或 gateway 认证
- 返回契约仍是最小动作字符串
- 更适合作为流程验证能力，而不是最终生产决策能力
- 当前领域层与协调器能表达 `failure / conflict / periodic` trigger type
- 当前 MCP runtime coverage 已接到这 4 条 hook，但仍是受限、显式、best-effort 的 demo 形态
- 这不代表“所有 MCP entry point 自动反思”，也不代表出现后台自治调度

### 未实现

- versioned identity/commitment ledger 和 record-only Reflection history
- Event/Episode/Reflection 纠错，以及真实客户端 M1 退出故事
- richer 自动 evidence lookup
- richer evidence weighting / relation / ranking
- richer reflection 语义（当前已有最小 `identity_core` / `commitments` 深层修订，但仍不是 richer schema / versioned policy）
- 更多 provider 类型（Azure、本地模型；provider 质量、SLA 和 gateway 认证仍未完成）
- 持续后台自治运行、独立 daemon、完整自治代理行为

## 8. 正式接入时需要注意的点

### 8.1 数据库路径

未显式设置 `database_url` 时，默认库会落到当前平台的用户数据目录，并按“本机用户共享默认库”语义复用。正式接入时仍建议在 `agent-llm-mm.local.toml` 里固定为你可备份、可区分环境的路径，例如：

```toml
sqlite:///Users/<you>/Library/Application%20Support/agent-llm-mm-codex.sqlite
```

### 8.1.1 配置文件

推荐用法：

- 从 `examples/agent-llm-mm.example.toml` 复制一份本地配置
- 写入自己的 `database_url`
- 选择 `provider`
- 填入自己的 API key
- 不要把 `agent-llm-mm.local.toml` 提交到仓库

配置加载语义需要特别注意：

- 服务默认启动路径走 `AppConfig::load()`，会先读 `AGENT_LLM_MM_CONFIG` 或默认 `agent-llm-mm.local.toml`，然后再允许 `AGENT_LLM_MM_DATABASE_URL` 覆盖数据库路径
- 如果你的集成或测试直接走 `AppConfig::load_from_path()`，显式文件里的 `database_url` 会被保留
- 但如果该文件省略 `database_url`，`load_from_path()` 仍可能通过 `AppConfig::default()` 继承 `AGENT_LLM_MM_DATABASE_URL` 派生出的默认路径

实践建议：

- 正式接入时不要同时依赖 TOML 里的 `database_url` 和外部注入的 `AGENT_LLM_MM_DATABASE_URL`，除非你就是想显式覆盖
- 如果要诊断“为什么接到了另一份 SQLite”，先检查客户端启动环境里是否偷偷带上了 `AGENT_LLM_MM_DATABASE_URL`

### 8.2 数据隔离

建议至少区分：

- 正式接入库
- 手工测试库
- 开发实验库

避免把反思、修订和测试事件混入正式记忆。

### 8.3 并发访问

SQLite 非常适合本机 MVP，但它仍然是单写者模型。若多个 AI 客户端并发共享同一数据库文件，需要预期：

- 锁等待
- 写入竞争
- 调试时状态互相影响

更稳妥的做法是每个环境单独一份数据库文件。

### 8.4 日志与 stdout

这是 `stdio` MCP 服务，因此：

- MCP 协议通信依赖标准输入输出
- 不应在 `serve` 模式额外向 `stdout` 打印杂讯
- tracing 诊断固定写入 `stderr`；`doctor` 的 JSON 与 `serve` 的 MCP frames 保持在 `stdout`
- 启用无认证 dashboard 时，host 只允许 `localhost` 或 loopback IP；非 loopback 配置会在启动前失败

### 8.5 能力边界

当前这条分支已经具备可嵌入的最小记忆闭环，但还不是完整产品：

- 默认 MCP transport 仍是 `stdio`；只有显式设置 `[dashboard].enabled = true` 时，才会额外启动本机只读 HTTP dashboard
- 无远程 Web 管理后台、写入型 dashboard 或 MCP HTTP transport
- 无更丰富的 evidence 自动检索
- 已有最小 `identity_update` / `commitment_updates` 反思修订，但仍无 richer schema、版本化策略与更细粒度生命周期
- 无 Azure / 本地模型 provider；OpenRouter 仅完成本地 OpenAI-compatible transport 切片
- automatic self-revision 当前 MCP-wired automatic path 仅限 `ingest_interaction -> failure`、`ingest_interaction -> conflict`、`decide_with_snapshot -> conflict`、`build_self_snapshot -> periodic`
- `decide_with_snapshot` / `build_self_snapshot` 仍要求显式 `auto_reflect_namespace`；`decide_with_snapshot` 还要求显式 conflict-compatible `trigger_hints`，并且只在非 blocked 决策后 best-effort 运行
- automatic self-revision 仍受 trigger ledger、证据门槛和慢更新约束保护，不是完整自治 daemon

## 9. 推荐验证顺序

```zsh
cd ~/code/agent-llm-mm
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
./scripts/agent-llm-mm.sh doctor
```

如果你刚改了 self-revision / runtime 接线，再追加：

```zsh
cargo test --test application_use_cases auto_reflection_runs_once_for_repeated_failure_and_records_handled_ledger -v
cargo test --test mcp_stdio decide_with_snapshot_can_trigger_conflict_auto_reflection_without_breaking_decision_flow -v
cargo test --test mcp_stdio ingest_interaction_does_not_auto_reflect_conflict_with_non_conflict_trigger_hints -v
cargo test --test mcp_stdio build_self_snapshot_can_trigger_periodic_auto_reflection_once_for_explicit_namespace -v
cargo test --test failure_modes auto_reflection_returns_structured_diagnostics_for_suppressed_trigger -v
cargo test --test failure_modes auto_reflection_returns_structured_diagnostics -v
cargo test --test failure_modes auto_reflection_rejects_model_proposed_evidence_outside_trigger_window -v
cargo test --test bootstrap doctor_reports_self_revision_runtime_coverage -v
```

如果都通过，再把它挂到本机 MCP 客户端配置里。
