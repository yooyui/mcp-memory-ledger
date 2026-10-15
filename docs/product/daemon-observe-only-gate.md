# Daemon Observe-Only Gate

## Scope

This gate protects future daemon work from jumping directly into write-capable
autonomy. Local Alpha keeps the daemon disabled by default. `doctor` may report
daemon configuration, but it must not start a daemon.

Observe-only mode is a diagnostic phase. It may read local state and report what
it would consider later, but it must not change identity, commitments,
reflections, claims, events, or tool responses.

## Required Observe-Only Behavior

An observe-only daemon may:

- read configured local stores and durable operation-log summaries
- inspect trigger candidates for repeated failures, conflicts, or periodic
  freshness checks
- compute diagnostics about suppression, cooldown, concurrency, and errors
- expose local-only diagnostic summaries for manual review
- exit cleanly without starting remote listeners or background write workers

It must be safe to run with formal local data because it does not write semantic
memory, identity, commitments, or reflection records.

## Observe-Only Lifecycle Contract

Start condition:

- `doctor` never starts the daemon handle; it only reads config and local
  operation-log diagnostics.
- The local handle may start only when `[daemon].enabled = true`; the `serve`
  entrypoint wires this handle in observe-only mode and stops it when the stdio
  service exits.
- The default config keeps `[daemon].enabled = false`, so normal Local Alpha
  first-run and default `serve` examples do not opt into daemon lifecycle
  behavior.

Stop condition and safe shutdown:

- A caller must keep the `DaemonHandle` and call `stop()` to signal shutdown.
- Disabled handles return promptly without entering a polling loop.
- Observe-only handles wait for either the configured poll interval or the
  shutdown signal, then exit without spawning write workers.
- Dropping a handle without calling `stop()` aborts the local lifecycle task so
  a forgotten handle cannot detach an orphan observe-only loop.
- Tests bound shutdown time so cancellation cannot hang the process during local
  validation.

Orphaned-work recovery:

- Observe-only mode has no durable work queue and no in-flight semantic writes.
  Restart recovery is therefore diagnostic-only: the next preflight reads the
  bounded local `operation_log` candidate counts again.
- There are no orphaned reflection tasks to resume because observe-only mode
  does not call `run_reflection`, does not create reflection audit rows, and
  does not write identity or commitments.

Candidate scan interval:

- The configured `poll_interval_ms` is the observe-only tick interval for the
  local handle.
- `doctor.daemon_observe_only` candidate counts are not scheduled scans; they
  are one-shot read-only summaries over local `operation_log`.
- Candidate reads are bounded to 25 rows per operation kind/status pair and
  currently cover `tool` / `trigger` entries with `failed` / `suppressed`
  status.

Proof of no daemon reflection writes:

- `run_reflection_allowed_from_daemon = false`
- `semantic_writes_allowed = false`
- `write_capable_daemon_gate_status = "blocked"`
- daemon lifecycle tests inspect the daemon source for absence of
  `run_reflection`, semantic append calls, remote listeners, and dashboard
  service startup

## Forbidden Behavior

During observe-only Local Alpha work, the daemon must not:

- call `run_reflection`
- write identity updates
- write commitment updates
- write reflection audit records
- create a side-channel durable write path
- start a remote listener
- ingest remote triggers
- expose remote management controls
- change MCP tool responses
- claim continuous autonomy, self-governance, or all-entry automatic
  self-revision

The historical future-daemon policy is preserved in
`codex/archive/pre-mainline-reset-2026-07-10` at
`docs/superpowers/specs/2026-04-27-local-daemon-trigger-policy.md`. It does not
authorize `run_reflection` calls in the current observe-only gate. Any future
daemon write proposal must re-enter the active plan as a new governed slice.

## Required Diagnostics

Observe-only diagnostics must be explicit enough to debug without implying write
authority:

- daemon enabled/disabled state and effective config
- polling interval and max concurrency settings
- local-only data sources inspected
- candidate read data source, operation kinds, statuses, and per-query bound
- explicit read-only marker for candidate reads
- trigger candidates observed
- trigger candidates skipped or suppressed
- suppression diagnostics
- cooldown status and diagnostics
- clean shutdown status
- in-flight task count
- errors encountered while reading local state
- whether a write-capable daemon gate has been approved; for Local Alpha this is
  `false`
- explicit closure of semantic writes, daemon `run_reflection` calls, connected
  daemon loops, and background autonomy

Diagnostics may be logged or displayed as local observation data, but they must
not be treated as identity or commitment updates.

Current Local Alpha implementation exposes these diagnostics through
`doctor.daemon_observe_only`. The report is local-only and observe-only:

- `mode = "observe_only"`
- `local_only = true`
- `write_gate_approved = false`
- `writes_allowed = false`
- `remote_listener_enabled = false`
- `data_sources` includes `daemon_config` and `operation_log`
- `candidate_read_data_source = "operation_log"`
- `candidate_read_operation_kinds = ["tool", "trigger"]`
- `candidate_read_statuses = ["failed", "suppressed"]`
- `candidate_read_limit_per_kind_status = 25`
- `candidate_reads_are_read_only = true`
- `trigger_candidates_observed` counts bounded local `operation_log` entries
  with `operation_kind` in `tool` / `trigger` and `status = failed`, capped
  at 25 rows per kind/status read
- `trigger_candidates_suppressed` counts bounded local `operation_log` entries
  with `operation_kind` in `tool` / `trigger` and `status = suppressed`,
  capped at 25 rows per kind/status read
- `cooldown_status = "observe_only"`
- `cooldown_diagnostics = "diagnostic_only_no_scheduling"`
- `suppression_diagnostics = "read_only_status_count"`
- `clean_shutdown_status = "not_started_by_doctor"`
- `lifecycle_regression_status = "verified_by_handle_stop_test"`
- `semantic_writes_allowed = false`
- `run_reflection_allowed_from_daemon = false`
- `write_capable_daemon_gate_status = "blocked"`
- `background_autonomy_enabled = false`
- `daemon_loop_connected = false`
- `daemon_started_by_doctor = false`
- `in_flight_task_count = 0`
- `read_errors` records operation-log read failures as diagnostics, not as
  semantic memory updates

This `doctor` field is a preflight diagnostic surface. It does not start a
daemon loop, does not verify shutdown during the current doctor process, does
not call `run_reflection`, and does not authorize daemon writes. The explicit
false/blocked fields are part of the Local Alpha wording guardrail: they are
diagnostic boundaries, not latent feature flags.

The local daemon handle also has an observe-only lifecycle proof:

- disabled handles exit promptly without running a polling loop
- observe-only handles can start and stop cleanly under test
- dropped observe-only handles abort their local lifecycle task instead of
  detaching it
- `serve` starts the handle only when `[daemon].enabled = true` and stops it
  after the stdio service exits
- `mode()` reports `disabled` or `observe_only`
- `writes_allowed()` is always `false`
- `remote_listener_enabled()` is always `false`

This lifecycle proof connects only the observe-only handle to `serve` behind an
explicit config flag. It does not connect MCP requests to daemon-triggered
semantic writes, remote listeners, or remote controls. It proves that the local
handle can be exercised and shut down while the write and remote gates remain
closed.

## Exit Gate Before Write-Capable Daemon Work

Before any daemon can trigger governed self-revision, a separate gate must prove:

- daemon remains disabled by default
- observe-only mode has tests for no identity, commitment, or reflection writes
- local observe-only start / stop has tests for clean shutdown and closed write /
  remote gates
- durable operation-log read paths are stable
- correlation IDs connect daemon diagnostics to operation summaries
- cooldown and concurrency behavior are tested
- failures are visible without changing MCP tool responses
- any future write still calls the existing governed `run_reflection` path
- there is no remote listener, remote trigger ingestion, or remote write admin
  surface

Failing this gate means daemon work remains observe-only or not implemented.

## Local Alpha Release Position

Passing this document's review does not make a daemon implemented. It only means
the product has a safe boundary for future daemon work. Local Alpha may say
"daemon disabled by default" and "observe-only daemon work is gated"; it must not
claim background autonomy or production self-governance.
