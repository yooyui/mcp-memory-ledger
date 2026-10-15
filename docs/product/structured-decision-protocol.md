# Structured Decision Protocol

## Scope

This document defines the current `decide_with_snapshot` response envelope for the local Rust MCP `stdio` technical demo / MVP. It is a compatibility contract for callers, not a complete decision engine design.

The model provider contract remains conservative: `ModelDecision` still only carries the model-selected `action` string. The structured protocol metadata is added by the application layer around that decision.

## Version 2 Response Envelope

`DecideWithSnapshotResult` preserves the existing caller fields:

- `blocked`: boolean gate outcome.
- `decision`: model decision object when a model was called, or `null` when the commitment gate blocked the action.

Version 2 keeps the earlier metadata and adds local explainability fields:

- `protocol_version`: currently `2`.
- `decision_id`: deterministic local id for the requested action.
- `requested_action`: action requested by the caller.
- `selected_action`: model-selected action when the model runs; it remains present when that selected action is rejected by the commitment gate, and is `null` when the requested action was blocked before the provider call.
- `confidence`: bounded local metadata for the current provider path; this is not calibrated statistical confidence.
- `status`: `blocked` when the commitment gate stops either the requested or provider-selected action, or `model_decision` when the selected action passes.
- `reason`: `commitment_gate_blocked_action` when the requested action is rejected, `commitment_gate_blocked_selected_action` when the provider-selected action is rejected, and `null` for accepted model decisions.
- `decision_authority`: `not_applicable_blocked` for blocked paths and `experimental_non_authoritative` for a returned provider action string. This is the machine-readable authority boundary; `status = model_decision` remains only for compatibility and does not make the result authoritative.
- `policy_scope`: currently `server_commitment_gate_only`. It states exactly which bounded policy check ran and prevents `gate.blocked = false` from being interpreted as a complete policy-passed verdict.
- `gate`: commitment-gate metadata with `name`, `blocked`, and `reason`.
- `policy_checks`: current policy-check list. The first slice contains the commitment gate.
- `provider_diagnostics_class`: bounded local label for the envelope's own diagnostics-carrying level. It is `not-applicable-gate-blocked` when the requested action is rejected before the provider call, `bounded-local-policy-rejected` when a provider-selected action is rejected, and `bounded-local-only` on accepted model decisions. It explicitly declares that this envelope does not carry provider-native structured diagnostics; it is a local explainability label only.
- `non_claims`: explicit boundaries for this local protocol.

Before either gate runs, the application loads the current commitment store and replaces the caller-provided commitment list in the model context. A caller cannot remove the baseline commitment or inject a new policy rule through `snapshot.commitments`.

Requested-action blocks must not call the model. Selected-action blocks occur after one provider call. Both keep `decision: null`; selected-action blocks retain the rejected `selected_action` for bounded explanation. Non-blocked responses keep the original `decision` payload shape, currently `{ "action": "..." }`, but are explicitly experimental and non-authoritative. The commitment gate only performs bounded literal matching against the server-side commitments; it is not structured action validation or broader policy arbitration.

## Non-Goals

- This does not add multi-step decision planning, confidence scoring, or policy arbitration.
- This does not require providers to emit structured JSON decisions.
- This does not turn an accepted provider action string into an authoritative policy decision.
- This does not make caller-provided identity, claims, evidence, or episodes authoritative and does not create a trusted snapshot handle.
- This does not change `run_reflection`; it remains the only durable self-revision write path.
- This does not change provider readiness or provider matrix status files.

## Verification

The compatibility slice is covered by:

```zsh
cargo test --test decision_flow -v
cargo test --test mcp_stdio fresh_stdio_runtime_blocks_forbidden_action_with_seeded_commitment -- --exact
cargo test --test mcp_stdio provider_selected_forbidden_action_is_blocked_over_stdio -- --exact
cargo test --test mcp_stdio decide_with_snapshot_over_stdio_uses_openai_compatible_provider_from_config_file -- --exact
```
