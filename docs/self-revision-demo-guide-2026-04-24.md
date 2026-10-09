# Self-Revision Demo Guide (2026-04-24)

## 目标

这套 demo package 用于在本机、零外网依赖的条件下证明当前 automatic self-revision MVP 的真实闭环：

- baseline memory 会进入 SQLite 并出现在 snapshot 中
- 没有显式 conflict hint 的输入不会触发 conflict auto-reflection
- 带显式 conflict hint 的输入会触发 governed automatic self-revision
- 通过治理后的 proposal 仍复用 `run_reflection` durable write path
- `snapshot-before` / `snapshot-after` 下的 `decide_with_snapshot` 会产生不同动作

它是可重复的技术 demo，不是新的 MCP tool、后台 daemon、Web UI 或完整自治系统。

## 一键运行

```zsh
./scripts/run-self-revision-demo.sh
```

默认产物会写入：

```text
target/reports/self-revision-demo/<timestamp>/
```

如果要固定输出目录：

```zsh
./scripts/run-self-revision-demo.sh target/reports/self-revision-demo/latest
```

脚本会先构建本地二进制，再运行 `run_self_revision_demo`。demo runner 会启动本地 deterministic `openai-compatible` stub provider，并通过真实 MCP `stdio` 服务调用写入 / 快照 / 决策 / 反思这 4 个既有 tool。当前运行时另有 `search_memory` / `get_memory` / history / `supersede_memory` 等读取与纠错工具，但不在该旧 demo story 内。

`decision-before.json` 会在 reflection 写入前通过真实 `decide_with_snapshot` 调用生成；`decision-after.json` 会在 reflection 写入并重新构建 snapshot 后生成。两次调用都读取调用时刻的服务端 commitment store，不依赖在修订后回放旧 snapshot 来制造 before / after 差异。

## 产物

运行完成后应能看到：

- `doctor.json`
- `snapshot-before.json`
- `snapshot-after.json`
- `decision-before.json`
- `decision-after.json`
- `timeline.json`
- `sqlite-summary.json`
- `report.md`

`report.md` 是给人读的摘要；其余 JSON 文件是可复核的证据链。

## 场景顺序

1. 写入 baseline self claim。
2. 构建 `snapshot-before`。
3. 调用会被 commitment gate 拦截的 direct identity write 决策。
4. 写入一条不带 conflict hint 的 negative 事件，验证不会自动修订。
5. 写入一条带 conflict hint 的 positive 事件，触发 automatic self-revision。
6. 构建 `snapshot-after`，确认新增 commitment。
7. 分别基于 before / after snapshot 调用 `decide_with_snapshot`。
8. 查询 SQLite 中的 trigger ledger、commitments、reflections，并生成 report。

## 预期结论

- `doctor.json` 中 `self_revision_write_path` 仍指向 `run_reflection`。
- `timeline.json` 中 negative 事件前后 handled conflict 数量不变。
- `timeline.json` 中 positive 事件后 handled conflict 数量增加。
- `snapshot-after.json` 包含 `prefer:confirm_conflicting_commitment_updates_before_overwrite`。
- `decision-before.json` 的 action 是 baseline action。
- `decision-after.json` 的 action 是 revised action。

## 边界

- 不需要真实 API key。
- 不访问外网。
- 不改变 MCP tool 列表。
- 不新增 durable write path。
- 不代表所有 MCP entry point 都会自动反思。
- 不代表 richer memory schema、后台自治或产品化封装已经落地。
