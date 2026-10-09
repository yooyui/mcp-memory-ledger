# MCP Memory Ledger Project Overview

## Summary

MCP Memory Ledger is a Rust-based local MCP `stdio` memory demo that validates a minimal loop for long-term memory, self-snapshot construction, and reflection. The current version uses SQLite for persistence and is best described as a technical demo, integration prototype, or research-oriented MVP rather than a complete product.

Compatibility note: the current Rust crate, binary, scripts, configuration examples, and some historical docs still use the technical identifier `agent_llm_mm` / `agent-llm-mm`. Use MCP Memory Ledger as the public project name.

## Current Scope

- Local MCP `stdio` server
- SQLite persistence
- Config-file driven provider loading
- `openai-compatible` / OpenRouter provider
- `ingest_interaction`
- `search_memory` / `get_memory` / `get_reflection_history` / `get_self_model_history` / `get_evidence_relation` / `supersede_memory`
- `build_self_snapshot`
- `decide_with_snapshot`
- `run_reflection`
- `doctor` / `serve` entry points

## Current Boundaries

- M1 has completed sixteen slices through M1.2.7: four scoped search/lookup types, union, Claim history, self-model audit, evidence-relation runtime, and scoped Claim supersede. The next slice is M1.3.0 current-schema structural readback
- `decide_with_snapshot` can now use an `openai-compatible` or OpenRouter provider, but its output contract is still a minimal action string
- There is no remote HTTP transport
- `get_evidence_relation` has a scoped runtime first slice; it is still not richer ranking / weighting
- Azure and local-model providers are not implemented yet; OpenRouter uses the OpenAI-compatible transport, and the explicit live runner only generates provider preflight evidence, not provider quality, SLA, or gateway certification
- The release evidence index, provider certification preflight, packaging preflight, and richer memory semantics projection are local read-only / preflight capabilities; they do not create missing product evidence, certify provider quality, build installers, or complete the broader multi-layer memory model

## Best Fit

- Local AI client integration experiments
- Self-agent memory demos and technical validation
- A minimal Rust + MCP + SQLite reference implementation

## Documentation Discipline

- After finishing each task, update the corresponding documentation whenever that task changes behavior, capability boundaries, integration flow, configuration, verification commands, or collaboration rules.
- Do not defer documentation updates until the end of a larger batch of work; code and docs should be closed out together whenever possible.
- The [formalization improvement plan](formalization-improvement-plan-2026-08-25.md) is a gap and acceptance map, not a second execution queue. During the current catch-up, integrate through the existing `dev-work -> main` pull request; `main` requires fresh CI and explicit human review.

## Verification Status

As of `2026-08-25`:

- tests are split into `fast`, `core`, and `full` tiers; default core checks do not compile release tooling
- the `release-tools` feature retains release-evidence, packaging, and provider-certification verification
- MCP `stdio` currently exposes 10 tools; `run_reflection` remains the only durable write path for identity / commitment / reflection
- M0 is closed; M1 is active through M1.2.7, and the next slice is M1.3.0
- the fresh local CI-equivalent gate passes format, all-target/all-feature Clippy, the full tier, status sync, and diff checks; remote PR checks remain a separate gate
- `doctor` returns `status = ok`

## Acknowledgement

This repository was developed, discussed, and documented with active support from OpenAI Codex as a collaborative development tool. Thanks to OpenAI for the tooling and research ecosystem that made this workflow possible.
