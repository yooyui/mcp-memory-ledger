# `rmcp 0.5` to `2.2.0` Compatibility Spike

状态：`completed / no-go for an in-place M0.5 upgrade`

日期：`2026-07-14`

## Decision

Do not upgrade the repository from `rmcp 0.5.0` to `2.2.0` inside M0.5.

The current official crate documentation identifies `2.2.0`, published on
2026-07-08, as the latest release. The existing server can be made to compile
against it with a small set of source migrations, but the isolated probe still
produced a warning that fails this repository's Clippy policy and changed one
handler-reached error contract in the MCP `stdio` regression suite.

A dedicated migration slice is allowed later, but only after it preserves the
four current tools and their error/schema contracts. It must not add remote
transport, tasks, OAuth, resources, prompts, or any other capability as part of
the dependency upgrade.

Official references:

- [`rmcp 2.2.0` crate documentation](https://docs.rs/rmcp/2.2.0/rmcp/)
- [Official Rust SDK repository](https://github.com/modelcontextprotocol/rust-sdk)
- [Official 1.x migration guide](https://github.com/modelcontextprotocol/rust-sdk/discussions/716)

## Repository Usage Inventory

The current runtime depends on these `rmcp 0.5.0` surfaces:

- `ServerHandler`, `ServiceExt`, `Server::serve`, and `transport::stdio`;
- `#[tool_router]`, `#[tool]`, and `#[tool_handler]`;
- a stored `ToolRouter<Self>` field;
- `handler::server::tool::Parameters`;
- `handler::server::tool::cached_schema_for_type` for four explicit input
  schemas;
- `ServerInfo` struct-literal construction;
- `CallToolResult`, `JsonObject`, and `ErrorData` for the existing structured
  result and error mapping contracts.

No remote transport, task, or OAuth feature is enabled in `Cargo.toml`.

## Isolated Probe Evidence

The probe was run from a temporary archive of the M0.4 checkpoint. Only the
temporary manifest changed from `rmcp = "0.5"` to `rmcp = "2.2.0"`; no probe
source or lockfile was copied back into this repository.

Initial `cargo check --all-targets --all-features` result:

- failed with 6 compiler errors;
- four calls to `cached_schema_for_type` no longer resolved;
- `tool::Parameters` became private at that path and must be imported through
  `handler::server::wrapper::Parameters`;
- `ServerInfo` is non-exhaustive and cannot be built with a struct literal.

After applying only the corresponding temporary mechanical fixes:

- `cargo check --all-targets --all-features` passed;
- the stored `tool_router` field became unused, leaving a warning that would
  fail `-D warnings`;
- `cargo test --test bootstrap --test mcp_stdio` passed bootstrap `26/26` and
  MCP `stdio` `47/48`;
- `handler_reached_missing_fields_append_failed_operation_log_without_changing_error_semantics`
  failed because the expected handler-reached MCP error contract was no longer
  observed.

This is enough to reject a blind version bump even though initialization,
tool listing, tool schemas, most tool calls, dashboard isolation, and most
error paths remained functional in the probe.

## Required Migration Checklist

1. Change the dependency in a dedicated branch and keep the feature set
   explicitly limited to server macros plus server-side `stdio` transport.
2. Move `Parameters` imports to the public wrapper path.
3. Replace `cached_schema_for_type` with the current public schema helper while
   preserving the four existing JSON Schema snapshots.
4. Replace `ServerInfo` literals with the builder/constructor API and preserve
   service name, version, instructions, protocol, and tool capability output.
5. Reconcile the current `ToolRouter<Self>` storage pattern with the new router
   macro expansion; remove dead state only after list/call routing tests prove
   equivalent behavior.
6. Preserve the current distinction between framework-level argument-shape
   rejection and handler-reached validation errors, including operation-log
   behavior and stable MCP error codes/messages.
7. Review every directly constructed or exhaustively matched SDK model for new
   `#[non_exhaustive]` requirements.
8. Review the generated lockfile and transitive dependency/MSRV changes.
9. Run the complete matrix below before changing the pinned dependency.
10. Update public docs only after the migrated checkpoint passes with zero
    warnings and no capability expansion.

## Required Test Matrix

| Area | Required evidence |
| --- | --- |
| Compile contract | `cargo check --all-targets --all-features` and all-feature Clippy with zero warnings on pinned Rust |
| Protocol initialization | Real child-process initialize handshake and graceful shutdown |
| Tool discovery | Exactly the existing four tool names; stable descriptions and input-schema snapshots |
| Tool execution | Success paths for ingest, snapshot, decision, and reflection over real `stdio` |
| Error mapping | Framework-shape errors, missing fields, invalid params, provider failures, and stable operation-log behavior |
| Transport isolation | MCP stdout contains protocol frames only; tracing stays on stderr; dashboard remains side-channel only |
| State and governance | Shared runtime state, commitment gates, reflection evidence rules, transaction failure atomicity, and trigger suppression |
| Lifecycle | Explicit `init`/`migrate`, current-only `serve`, and read-only `doctor` remain unchanged |
| Platforms | Linux and macOS CI on the pinned Rust toolchain; Windows remains an M2 parity gate |
| Full repository | `./scripts/test-tier.sh full` and `./scripts/status-sync-check.sh` |

## Explicit Exclusions

The future dependency-migration slice is not authorization to add:

- Streamable HTTP or another remote transport;
- MCP tasks or task persistence;
- OAuth, authentication, authorization, or team mode;
- resources, prompts, elicitation, sampling, or new MCP tools;
- daemon writes or another durable identity/commitment/reflection path.

Those capabilities remain governed by their own roadmap gates.
