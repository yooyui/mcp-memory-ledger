# Contributing

Thanks for your interest in improving `agent_llm_mm`.

## Before You Start

- Read [README.md](README.md)
- Read [macOS 开发与接入指南](docs/development-macos.md) or [Windows 开发与接入指南](docs/development-windows.md)
- Read [当前实现状态](docs/project-status.md)
- Read [路线图](docs/roadmap.md)
- Read [测试指南](docs/testing-guide-2026-03-24.md)

This repository is a public technical demo and MVP. Please keep changes aligned with that positioning. Avoid turning incomplete experimental capabilities into product claims in docs or code comments.

## Environment

平台相关环境要求不再写在本页，请直接进入对应文档：

- macOS：见 [docs/development-macos.md](docs/development-macos.md)
- Windows：见 [docs/development-windows.md](docs/development-windows.md)

## Suggested Workflow

1. Open an issue or describe the scope clearly before making non-trivial changes.
2. Keep the blast radius small and focused.
3. After each completed task, update the corresponding docs whenever behavior, scope, integration, configuration, verification commands, or public-facing wording changes.
4. Prefer explicit wording for what is implemented, partially implemented, and not implemented.
5. For release-facing changes, follow [Release Engineering](docs/product/release-engineering.md): source-only artifacts first, evidence directory recorded, compatibility and soak evidence captured when relevant, and deprecation notes written before removal.
6. Keep `Cargo.toml` and `rust-toolchain.toml` aligned. Toolchain changes must be explicit and pass the Linux/macOS CI-equivalent full gate documented in [Rust Toolchain Policy](docs/toolchain-policy.md).

## Branch And Merge Policy

- `main` is the public integrated branch. Normal feature work must reach it through a pull request; do not force-push `main`.
- During the current mainline catch-up, `dev-work` is the integration branch and the existing `dev-work -> main` pull request is the canonical merge path. Do not create a duplicate mainline PR for the same commit chain.
- Push an integration candidate only after the focused checks for each change and the local CI-equivalent final gate pass. A GitHub push updates source state; it is not a release or production deployment.
- Merge to `main` only when the pull request head has fresh Linux/macOS checks and a human reviewer has explicitly accepted the candidate. Old checks from an earlier head do not satisfy this gate.
- Create candidate tags or release notes only from an exact `main` commit that has passed the applicable release gate and human release decision.

The current formalization gaps and acceptance path are tracked in the [Formalization Improvement And Mainline Sync Plan](docs/formalization-improvement-plan-2026-08-25.md). That document is a cross-cutting gap map; the active project plan remains the only implementation queue.

## Verification

提交前验证命令请按当前平台读取对应文档：

- macOS：见 [docs/development-macos.md](docs/development-macos.md)
- Windows：见 [docs/development-windows.md](docs/development-windows.md)

Before release claims or public release notes, also run the applicable
[Release Gate](docs/release-gate.md) and record the release engineering evidence
described in [docs/product/release-engineering.md](docs/product/release-engineering.md).
For local release soak evidence, use
`./scripts/release-soak-local.sh <candidate-name> [config_path]` and preserve the
generated `target/reports/releases/<candidate-name>/` evidence directory with
the release note or review record.

The repository CI contract is `cargo fmt --all -- --check`, all-target/all-feature
Clippy with `-D warnings`, `./scripts/test-tier.sh full`, and
`./scripts/status-sync-check.sh` on Linux and macOS. Windows runtime parity is a
separate M2 evidence gate, not something inferred from this matrix.
Do not claim Beta, GA, production-ready status, remote write admin, remote team
service, multi-tenancy, or a replacement for `run_reflection` unless a later
gate explicitly approves that boundary.

## License for Contributions

By intentionally submitting a contribution to this repository, you agree that it will be licensed under Apache License 2.0, consistent with the repository license.

## Documentation Expectations

If your change affects project positioning or collaboration, update the relevant docs:

- [README.md](README.md)
- [文档总览](docs/document-map.md)
- [当前实现状态](docs/project-status.md)
- [路线图](docs/roadmap.md)
- [发布准备评估](docs/release-readiness.md)
- [Release Engineering](docs/product/release-engineering.md)

Release-related documentation must keep the product statement conservative.
Remote write/admin claims blocked means no contributor should describe an
ungated remote write path, remote admin surface, production self-governance, or
GA readiness as implemented.

## Acknowledgement

This repository is developed with active support from OpenAI Codex during discussion, iteration, and documentation refinement. Thanks to OpenAI for making that workflow possible.
