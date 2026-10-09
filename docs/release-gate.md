# Release Gate

## Scope

This gate is for the local MCP `stdio` technical demo / MVP. Passing it does not certify production autonomy, remote administration, multi-tenant deployment, or background daemon behavior.

For the next productization stage, use the separate [Local Alpha Release Gate](product/release-gate-local-alpha.md). That product alpha gate does not replace this MVP gate: this file remains the minimum release gate for the technical demo / MVP, while the Local Alpha gate adds install/config/product-smoke, dashboard local-only, daemon observe-only, and product wording checks.

Release engineering rules for version naming, source-only artifact shape,
changelog entries, evidence directories, compatibility matrix, soak evidence,
and deprecation policy live in
[`product/release-engineering.md`](product/release-engineering.md). Those rules
do not replace this gate; they define what must be recorded before a release
candidate is described publicly.

本 gate 的验证命令显式分为三层：**最小验证集（Minimum Verification Set）** 用于日常提交前，**全功能发布验证（Full-Feature Release Verification）** 在发布前必须通过，**建议验证集（Recommended Verification Set）** 按变更范围补跑并保留证据。三层都不证明生产自治。

## Minimum Verification Set（最小验证集）

日常提交前必须通过的默认运行时门槛：

- `cargo fmt --check`
- `git diff --check`
- `./scripts/test-tier.sh fast`
- `cargo clippy --all-targets -- -D warnings`
- `./scripts/test-tier.sh core`
- `./scripts/status-sync-check.sh`
- `./scripts/agent-llm-mm.sh doctor`

## Full-Feature Release Verification（全功能发布验证）

发布前额外必须通过；它重新启用非默认 `release-tools`，不会因为日常减重而跳过发布证据、打包或 provider certification 测试：

- `cargo clippy --all-targets --all-features -- -D warnings`
- `./scripts/test-tier.sh full`

## Recommended Verification Set（建议验证集）

发布前应当补跑的更深检查。以下各 Gate 同属建议验证集：Release Engineering Gate、Self-Revision Evidence Gate、Dashboard Gate、Operation Log Gate、Daemon Config Gate。它们补足发布前的可读证据与边界证明，但不替代最小验证集。

### Release Engineering Gate

- Use a source-only tag or equivalent conservative source artifact for the first
  productization-stage release; do not claim binary packaging, installer,
  hosted service, Beta, GA, or production-ready delivery.
- Record a candidate-specific release evidence directory instead of relying on
  stale `latest` artifacts.
- For local release soak evidence, use
  `./scripts/release-soak-local.sh <candidate-name> [config_path]` and keep the
  generated `target/reports/releases/<candidate-name>/` directory as candidate
  evidence.
- Record version naming, changelog boundaries, compatibility matrix results,
  soak command results when required, and deprecation status according to
  [`product/release-engineering.md`](product/release-engineering.md).
- Confirm release notes and public docs keep remote write/admin claims blocked:
  no remote write admin, production self-governance, remote team service,
  multi-tenancy, or `run_reflection` replacement claim may be made from this
  gate.

### Self-Revision Evidence Gate

本 gate 产出的 self-revision demo package 是「自动修订能力的发布前可读证据」：`report.md` 供人阅读核对修订前后的 identity / commitment / decision 变化，其余 JSON（doctor、snapshot before / after、decision before / after、timeline、SQLite summary）供机器复核同一条可重复链路。固定保留该 package 作为发布前证据，但它只证明当前 MVP 边界内的可重复链路，不新增 MCP tool、daemon 或新的 durable write path。

- `cargo test --test demo_openai_compatible_stub --test self_revision_demo_runner --test openai_compatible_model --test mcp_stdio -v`
- `./scripts/run-self-revision-demo.sh target/reports/self-revision-demo/latest`

Required artifacts:

- `target/reports/self-revision-demo/latest/doctor.json`
- `target/reports/self-revision-demo/latest/snapshot-before.json`
- `target/reports/self-revision-demo/latest/snapshot-after.json`
- `target/reports/self-revision-demo/latest/decision-before.json`
- `target/reports/self-revision-demo/latest/decision-after.json`
- `target/reports/self-revision-demo/latest/timeline.json`
- `target/reports/self-revision-demo/latest/sqlite-summary.json`
- `target/reports/self-revision-demo/latest/report.md`

### Dashboard Gate

- `cargo test --test dashboard_config --test dashboard_recorder --test dashboard_projection --test dashboard_http`
- `cargo test --test mcp_stdio dashboard_enabled_does_not_corrupt_mcp_stdout_and_records_tool_event -v`

The dashboard must remain local-only and read-only unless a separate productization plan explicitly changes that boundary.

### Operation Log Gate

- `cargo test --test operation_log -v`

### Daemon Config Gate

- `cargo test --test daemon_config -v`

The daemon must remain disabled by default. Doctor reports config without starting the daemon.

## Failure Interpretation

Sandbox-only failures must be called out separately from code failures. If a command writes outside the workspace or opens a local listener, rerun it in an environment that permits that operation and record both results.
