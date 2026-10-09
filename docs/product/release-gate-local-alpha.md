# Local Alpha Release Gate

## Scope

This gate is for the Local Product Alpha phase after the validated local MCP `stdio` MVP. The MVP gate in [`../release-gate.md`](../release-gate.md) remains the authority for the technical demo / MVP; this document adds the product alpha checks needed before the repository can be described as a Local Product Alpha.

Current wording must stay conservative until every gate below has fresh evidence: validated local MVP entering productization. Passing the MVP gate alone does not certify Local Alpha, GA, production self-governance, remote write admin, remote team service, or multi-tenancy.

## Minimum Local Alpha Gate

Run the same baseline commands from a clean, reviewable working tree:

```bash
cargo fmt --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
./scripts/test-tier.sh full
./scripts/agent-llm-mm.sh doctor
```

Expected evidence:

- formatting and whitespace checks exit with code `0`
- `cargo clippy` exits with code `0` and no warnings
- `./scripts/test-tier.sh full` exits with code `0`
- `doctor` reports `status = ok`
- `doctor` continues to report `self_revision_write_path = run_reflection`
- `doctor` does not expose provider secrets
- bootstrap documentation remains config-first then doctor-second, and wrapper
  scripts keep the `serve|init|migrate|doctor|bootstrap-local` contract with
  unsupported modes returning exit code `2`
- `bootstrap-local` must refuse to overwrite an existing config, must copy only
  the safe dev example profile, and must not create secrets, run `doctor`, start
  `serve`, enable daemon behavior, or claim installer / production readiness
- relative `bootstrap-local` targets are documented and tested as repository-root
  relative; cross-directory examples should prefer absolute config paths
- first-run bootstrap smoke evidence must prove a local-only
  `bootstrap-local -> init -> doctor --read-only` simulation in an isolated output directory, with
  `fresh_machine_simulation = true` and `real_fresh_machine_evidence = false`
- first-run bootstrap smoke must clear config/database environment overrides,
  rewrite the generated dev config to an isolated SQLite path, write
  `doctor.json` and `summary.json`, refuse non-empty evidence directories before
  writing artifacts, and must not start `serve`, call product smoke, call the
  demo wrapper, invoke remote commands, or trigger daemon/reflection writes
- PowerShell runtime parity must have either a Windows runner / Windows machine
  execution record, or an explicit note that local verification only covered
  static script contract assertions because `pwsh` was unavailable

### Fresh Evidence: 2026-05-16 Formal Product Readiness Slice

Ran from branch `codex/support-bundle-log-excerpts` in an isolated
worktree:

```bash
cargo fmt --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
./scripts/agent-llm-mm.sh doctor
```

Result:

- `cargo fmt --check`: passed
- `git diff --check`: passed
- `cargo clippy --all-targets --all-features -- -D warnings`: passed
- `cargo test`: passed, 194 tests
- `./scripts/agent-llm-mm.sh doctor`: passed with `status = ok`
- `doctor.self_revision_write_path = "run_reflection"`
- `doctor.daemon_observe_only.writes_allowed = false`

This evidence predates the `bootstrap-local` first-run helper and is retained as
historical formal product readiness evidence, not as proof of the expanded
first-run bootstrap contract.

### Fresh Evidence: 2026-05-16 First-Run Bootstrap Slice

Ran from branch `codex/install-first-run-capability` in an isolated worktree:

```bash
cargo test --test bootstrap -v
git diff --check
```

Result:

- `cargo test --test bootstrap -v`: passed, 24 tests
- `git diff --check`: passed

Manual spot checks for this slice passed:

- `bootstrap-local` created a missing target from
  `examples/agent-llm-mm.dev.example.toml`
- `bootstrap-local` refused existing targets
- `bootstrap-local` refused missing parent directories
- `bootstrap-local` refused dangling symlink targets on Unix
- relative targets resolved from the repository root, not the caller working
  directory
- printed bash next commands quoted paths with spaces

Windows / PowerShell limitation: if the verification host lacks `pwsh`, this
slice can only cover the PowerShell wrapper through static contract assertions;
Windows runner or Windows machine evidence remains required before claiming
runtime parity.

### Fresh Evidence: 2026-05-16 First-Run Bootstrap Smoke Slice

Ran from branch `codex/support-bundle-log-excerpts` in an isolated worktree:

```bash
bash -n scripts/first-run-bootstrap-smoke-local.sh
cargo test --test first_run_bootstrap_smoke -v
```

Result:

- `bash -n scripts/first-run-bootstrap-smoke-local.sh`: passed
- `cargo test --test first_run_bootstrap_smoke -v`: passed, 4 tests
- success-path smoke generated local-only `doctor.json`, `summary.json`,
  `agent-llm-mm.local.toml`, and an isolated `first-run.sqlite`
- env-isolation coverage proved `AGENT_LLM_MM_DATABASE_URL`, `HOME`, and
  `XDG_DATA_HOME` did not redirect the smoke database or receive writes
- non-empty output directories were rejected before config, SQLite, doctor, or
  summary artifacts were written
- static coverage confirmed the helper does not call `serve`,
  `product-smoke-local.sh`, `run-self-revision-demo.sh`, or remote-copy commands

This is local fresh-machine simulation evidence. It does not prove a real
fresh-machine install, Windows runner parity, installer packaging, remote
bootstrap, Local Alpha completion, Beta, or GA readiness.

## Product Smoke Gate

Run the local product smoke script from the repository root with the repo-relative script path:

```bash
./scripts/product-smoke-local.sh [config_path]
```

From another working directory, invoke the same script by absolute path:

```bash
/path/to/agent-llm-mm/scripts/product-smoke-local.sh [config_path]
```

If you pass `config_path` from another working directory, use an absolute config
path. The repo-relative example below assumes you are already in the repo root;
the absolute-path example below applies from any working directory.

Required evidence:

- the script exits with code `0`
- `doctor` exits with code `0`
- when `[config_path]` is provided, the path exists, is resolved to an absolute path, and is passed to `doctor`
- the deterministic self-revision demo wrapper generates artifacts in a staging directory and promotes them to `target/reports/self-revision-demo/latest` only after the same smoke run passes the artifact checks
- all 8 required self-revision demo artifacts listed in the Self-Revision Evidence Gate are present and non-empty

### Fresh Evidence: 2026-05-16

Ran from branch `codex/support-bundle-log-excerpts`:

```bash
rm -rf target/reports/self-revision-demo/latest
./scripts/product-smoke-local.sh
```

Result:

- product smoke exited with code `0`
- `doctor` ran as part of product smoke and returned `status = ok`
- deterministic self-revision demo ran in a staging directory
- staging artifacts were promoted to `target/reports/self-revision-demo/latest`
- the required 8 release artifacts were present and non-empty:
  - `doctor.json`
  - `snapshot-before.json`
  - `snapshot-after.json`
  - `decision-before.json`
  - `decision-after.json`
  - `timeline.json`
  - `sqlite-summary.json`
  - `report.md`

Generated demo support files in `latest`, such as `demo.sqlite` and
`agent-llm-mm.demo.toml`, remain local artifacts and are not support-bundle
shareables.

Important limitation: `[config_path]` applies only to `doctor`. `scripts/run-self-revision-demo.sh` currently accepts only an output directory, so the product smoke script keeps the existing deterministic demo contract and does not pass a config path to the demo wrapper.

This gate proves the current local wrapper path, optional config bootstrap health, and the self-revision demo evidence chain. It does not prove fresh-machine install, guided config profiles, backup/restore, GA readiness, production self-governance, remote write admin, remote team service, or multi-tenancy.

## Data Lifecycle Gate

Local Alpha data lifecycle rules are documented in
[`data-lifecycle.md`](data-lifecycle.md). Product evidence must keep formal,
test, and demo SQLite data separated by explicit `database_url` values.

Required boundary:

- formal data, manual test data, and demo data use separate SQLite files
- backup uses a local SQLite backup or conservative copy helper before schema
  migration or formal data path changes
- restore defaults to a new path first; never overwrite a formal database as
  the first restore step
- `cargo test --test sqlite_backup_restore` covers the local backup/restore
  script gate: backup-to-restore roundtrip, restore overwrite refusal, live DB
  subdirectory backup refusal, in-memory database refusal, invalid SQLite URL
  percent-encoding refusal, and `..` restore target refusal
- support bundle summaries are not full database exports
- demo SQLite artifacts are demo evidence, not formal user data export
- migration work validates against a test or restored database before changing
  the formal `database_url`

This gate does not certify fresh-machine install, remote backup service,
multi-tenant data lifecycle, or production disaster recovery.

## Support Bundle Gate

Local Alpha support bundle behavior now has a first local-only generator:

```bash
./scripts/generate-support-bundle.sh <output_dir> [config_path] [--log-file <path>] [--correlation-id <id>]
```

Review [`support-bundle-local-alpha.md`](support-bundle-local-alpha.md) before
sharing debugging material. The generator is evidence for local diagnostic
packaging, not evidence that Local Alpha, production support, or remote upload
flows are complete.

Required boundary:

- `<output_dir>` must not exist yet or must be empty; generation must fail before
  writing bundle artifacts if the requested directory already contains files
- allowed contents are limited to redacted `doctor` shape, redacted config shape,
  bounded operation summaries, release metadata, product smoke evidence summary,
  bounded explicit local log excerpts, and bundle manifest metadata
- support bundle generation must not call normal runtime bootstrap, create or
  migrate SQLite databases, or seed default identity / commitments
- operation summaries must use read-only local SQLite access and mark themselves
  unavailable when the database or `operation_log` table is absent
- operation summaries may be explicitly filtered with generated
  `--correlation-id mcp-tool-call-<uuid-v4>` values, must record only the filter shape,
  and must still omit request / response / diagnostic payload summaries
- non-generated, non-canonical, or non-v4 correlation id filter values must fail
  before bundle artifacts or output directories are created
- local log excerpts must be generated only from an explicit `--log-file <path>`
  argument; support bundle generation must not scan default log directories,
  home directories, system logs, browser profiles, SSH directories, shell
  history, or `target/` output
- local log excerpts must be bounded and redacted, must omit raw provider
  payloads, prompt text, request bodies, response bodies, tool arguments, cookies,
  browser session material, local private paths, provider URL userinfo/path/query
  values, API keys, bearer values, tokens, passwords, and secrets, and must not
  copy the raw `.log` file
- oversized explicit log files must be read from a bounded tail window, discard
  partial first retained lines, and mark tail-scoped line numbers instead of
  implying original file line numbers
- excluded contents include API keys, `Authorization` / `Bearer` values, raw
  provider payloads with secrets, full SQLite databases by default, unredacted
  TOML files, provider URL userinfo/path/query secrets, SSH keys, cookies, and
  browser session data
- support bundle tests must prove redaction, bounded operation summaries, and
  bounded explicit local log excerpts
- no support bundle flow may upload data or claim production support readiness

Recommended verification when support bundle behavior changes:

```bash
cargo test --test support_bundle -v
bash -n scripts/generate-support-bundle.sh
rm -rf target/support-bundles/manual-check
./scripts/generate-support-bundle.sh target/support-bundles/manual-check
rm -rf target/support-bundles/manual-correlation-check
./scripts/generate-support-bundle.sh target/support-bundles/manual-correlation-check --correlation-id mcp-tool-call-018fbc89-9ac1-4f5d-8b2a-1f6f5f27b205
find target/support-bundles/manual-check -maxdepth 1 -type f -print | sort
rg -n 'api_key|api-key|x-api-key|Authorization|Bearer|sk-|provider_token|openai_api_key|password|secret|sqlite:///|token=' target/support-bundles/manual-check || true
find target/support-bundles/manual-check \( -name '*.sqlite' -o -name '*.toml' -o -name '*.log' \) -print
```

### Fresh Evidence: 2026-05-16

Ran from branch `codex/support-bundle-log-excerpts`:

```bash
rm -rf target/support-bundles/local-alpha-gate
./scripts/generate-support-bundle.sh target/support-bundles/local-alpha-gate
find target/support-bundles/local-alpha-gate -maxdepth 1 -type f -print | sort
rg -n 'api_key|api-key|x-api-key|Authorization|Bearer|sk-|provider_token|openai_api_key|password|secret|sqlite:///|token=' target/support-bundles/local-alpha-gate || true
find target/support-bundles/local-alpha-gate \( -name '*.sqlite' -o -name '*.toml' -o -name '*.log' \) -print
```

Result:

- support bundle generation exited with code `0`
- generated files were limited to:
  - `config-shape.json`
  - `doctor.json`
  - `local-log-excerpts.json`
  - `manifest.json`
  - `operation-summaries.json`
  - `product-smoke-summary.json`
  - `release-metadata.json`
- sensitive-token search produced no unredacted secret hits
- `.sqlite`, `.toml`, and raw `.log` exclusion check produced no files

## Local Alpha Evidence Summary Gate

The Local Alpha evidence summary is a read-only rollup for reviewers:

```bash
./scripts/local-alpha-evidence-summary.sh \
  --evidence-root . \
  --output-json target/reports/local-alpha/evidence-summary.json \
  --output-md target/reports/local-alpha/evidence-summary.md
```

This command summarizes existing evidence into JSON and optional Markdown. It
does not run the product smoke, start `serve`, upload files, start daemon
behavior, or perform durable reflection writes. It is not an automatic
certification mechanism.

Expected inputs under `--evidence-root`:

- `target/reports/self-revision-demo/latest/` with the 8 product smoke
  artifacts listed in the Product Smoke Gate; every required artifact must be
  non-empty
- first-run bootstrap evidence at `first-run-bootstrap/summary.json`, or the
  documented release evidence path
  `target/first-run-bootstrap-smoke/local-alpha-gate/summary.json`
- Windows parity evidence at `windows-parity/summary.json`, or the documented
  release evidence path `target/windows-parity/local-alpha-gate/summary.json`;
  the summary must include a Windows runner or Windows platform marker,
  `evidence_kind = windows_runtime_parity`, non-empty `captured_at`, and
  successful bootstrap / doctor / product-smoke command evidence, not only a
  generic `verified` status
- support bundle evidence at `support-bundle/`, or the documented release
  evidence path `target/support-bundles/local-alpha-gate/`, containing only the
  allowed local diagnostic files listed in the Support Bundle Gate

Required boundary:

- JSON output must include `overall_status`, `local_only`, `summary_boundary`,
  `gates`, `external_blockers`, `human_blockers`, and
  `unimplemented_capability_blockers`
- every gate must include `name`, `status`, and either `evidence_path` or a
  concrete `reason`
- blocker arrays must remain machine-readable and must not turn missing
  external evidence into local evidence; fresh-machine and Windows blockers
  stay explicit until real evidence is recorded, and the human release decision
  remains a separate blocker
- missing required artifacts keep the relevant gate `open`
- missing Windows runner / Windows machine evidence keeps Windows parity
  `not_verified`
- `real_fresh_machine_evidence = false` in the first-run summary must prevent a
  Local Alpha complete status
- `real_fresh_machine_evidence = true` is not sufficient by itself; real
  fresh-machine evidence must also include a real fresh-machine evidence kind,
  `fresh_machine_simulation = false`, non-empty `captured_at`, non-empty
  `source_checkout`, and successful `bootstrap-local` / `doctor` command
  evidence with explicit `exit_code = 0`
- `fresh_machine_simulation = true` in the same summary may satisfy the
  separate `first_run_simulation` gate, but must not satisfy the real
  fresh-machine `first_run_bootstrap` gate by itself
- first-run summary evidence must also preserve `doctor_status = ok`,
  `sqlite_database_exists = true`, `self_revision_write_path = run_reflection`,
  and the local-only / no-serve / no-product-smoke / daemon-write-closed boundary
- support bundle summary must validate the key generated JSON markers:
  `manifest.bundle_format`, `manifest.local_only`, `manifest.upload_performed`,
  `doctor.self_revision_write_path`, `doctor.runtime_bootstrap_performed`, and
  `product-smoke-summary.self_revision_write_path_expected`
- `overall_status` must remain conservative: when every gate is `satisfied`,
  the summary reports `ready_for_human_review` rather than automatically
  certifying Local Alpha completion; open gates keep the status `in_progress`,
  and unresolved verification-only gaps keep it `not_verified`
- Markdown output, when requested, must state that Local Alpha is not complete
  unless every gate is satisfied and a human release decision is made
- `run_reflection` remains the only durable identity / commitment / reflection
  write path

Recommended verification when the evidence summary changes:

```bash
cargo test --features release-tools --test local_alpha_release_evidence -v
bash -n scripts/local-alpha-evidence-summary.sh
cargo run --quiet --features release-tools --bin local_alpha_evidence_summary -- --evidence-root .
```

The summary may be attached to release notes or review packets as a gate status
index. It must not be used to weaken missing real fresh-machine, Windows,
support bundle, product smoke, dashboard, daemon, or data lifecycle evidence.

## Local Alpha Release-Gate Refresh

When refreshing the locally reproducible part of the Local Alpha gate, use the
combined local refresh script:

```bash
./scripts/local-alpha-release-gate-refresh.sh [config_path]
```

The script runs:

- `scripts/product-smoke-local.sh [config_path]`
- `scripts/first-run-bootstrap-smoke-local.sh target/first-run-bootstrap-smoke/local-alpha-gate`
- `scripts/generate-support-bundle.sh target/support-bundles/local-alpha-gate [config_path]`
- `scripts/local-alpha-evidence-summary.sh --evidence-root . --output-json target/reports/local-alpha/evidence-summary.json --output-md target/reports/local-alpha/evidence-summary.md`

Required boundary:

- it only refreshes local evidence that this checkout can produce
- it may leave `overall_status = in_progress` or `not_verified`; that is the
  correct result while real fresh-machine, Windows runner, remote/team, or
  release-decision evidence is missing
- it must not create `target/windows-parity/local-alpha-gate` or rewrite
  first-run simulation evidence into `real_fresh_machine_evidence = true`
- it does not upload artifacts, start remote listeners, enable daemon writes, or
  certify Local Alpha completion

Recommended verification when the refresh script changes:

```bash
bash -n scripts/local-alpha-release-gate-refresh.sh
cargo test --features release-tools --test local_alpha_release_evidence -v
```

## Local Release Soak Evidence

When preparing a candidate-specific local release evidence directory, use:

```bash
./scripts/release-soak-local.sh <candidate-name> [config_path]
```

The script writes to `target/reports/releases/<candidate-name>/` and runs:

- `./scripts/agent-llm-mm.sh doctor [config_path]`
- `cargo test --test dashboard_http -v`
- `scripts/product-smoke-local.sh [config_path]`
- `scripts/first-run-bootstrap-smoke-local.sh target/first-run-bootstrap-smoke/local-alpha-gate`
- `scripts/generate-support-bundle.sh target/support-bundles/local-alpha-gate [config_path]`
- support-bundle secret scan, support-bundle raw artifact scan, and release
  evidence secret scan after command logs and summaries are redacted
- support-bundle and product-smoke SHA-256 manifest generation
- `scripts/local-alpha-evidence-summary.sh` into the release evidence directory
- `compatibility-matrix.json` and `release-boundaries.json` with local checked
  rows and explicit blocked / not-checked rows for external and unimplemented
  capabilities

Required boundary:

- candidate names must be path-safe and cannot contain `..`
- the evidence directory must be absent or empty before the run
- command logs and exit codes must be recorded in `commands/` and
  `command-summary.tsv`, with config paths recorded only as redacted shapes
- `support-bundle-sha256.txt` and `product-smoke-latest-sha256.txt` must be
  non-empty so the candidate evidence can be tied back to the generated local
  artifacts
- `compatibility-matrix.json` may record the current local shell wrapper path as
  checked by the soak, but Windows must remain `not_checked` unless a Windows
  runner or Windows machine generated separate evidence
- `release-boundaries.json` must keep fresh-machine, Windows parity, human
  release decision, remote/team, security/auth, daemon-write, and packaging
  blockers explicit
- support bundle scans must not report unredacted secret-like markers or raw
  `.sqlite`, `.toml`, or `.log` files
- the generated Local Alpha evidence summary remains a status summary; if it
  reports `in_progress` or `not_verified`, the corresponding gate remains open

The release soak runner creates local candidate evidence only. It does not
create real fresh-machine evidence, Windows runner evidence, remote/team
evidence, upload artifacts, source tags, binary packages, installers,
service-manager state, auto-updaters, release decisions, or Local Alpha
certification.

Recommended verification when the release soak runner changes:

```bash
bash -n scripts/release-soak-local.sh
cargo test --features release-tools --test local_alpha_release_evidence release_soak -v
```

## Correlation ID Gate

Runtime observability must keep MCP calls traceable without creating a new
semantic write path. Review
[`correlation-id-contract.md`](correlation-id-contract.md) when changing MCP
tool handlers, dashboard projection, operation-log runtime wiring, or support
bundle summaries.

Required evidence:

- dashboard event detail exposes `correlation_id`
- each MCP `tools/call` gets a generated `mcp-tool-call-<uuid-v4>` correlation ID
- distinct MCP calls get distinct correlation IDs
- best-effort auto-reflection dashboard diagnostics reuse the triggering MCP
  call correlation ID
- successful MCP tool calls append operation-log metadata with the same
  correlation ID
- local dashboard `GET /api/operation-log` can read durable operation-log
  entries by bounded filters such as `correlation_id`, with a default and
  maximum list limit of 100 entries, without changing the existing live
  `/api/events` in-memory recorder
- correlation metadata does not write identity, commitments, claims, or
  reflections outside governed `run_reflection`

## Self-Revision Evidence Gate

For Local Alpha release evidence, use the Product Smoke Gate above. It runs the
existing self-revision demo wrapper through `scripts/product-smoke-local.sh`,
validates the required artifacts in a staging directory, and promotes them to
`latest` only after the checks pass.

For manual demo-only diagnostics outside a release gate, write to a timestamped
or scratch directory instead of `latest`:

```bash
./scripts/run-self-revision-demo.sh target/reports/self-revision-demo/manual-$(date +%Y%m%d-%H%M%S)
```

Required artifacts under `target/reports/self-revision-demo/latest`:

- `doctor.json`
- `snapshot-before.json`
- `snapshot-after.json`
- `decision-before.json`
- `decision-after.json`
- `timeline.json`
- `sqlite-summary.json`
- `report.md`

The 8 artifacts must come from the same successful self-revision demo run. A pre-existing `latest` directory is not sufficient release evidence. The evidence must still show a before / after decision shift, refreshed snapshot state, SQLite summary, and a report that stays within the current MVP boundary.

## Dashboard Local-Only Gate

The dashboard remains a local-only, read-only inspection surface for Local Alpha.

Required boundary:

- enabled dashboard configuration accepts only `localhost` or a loopback IP;
  non-loopback hosts fail validation before bind
- expose read-only routes for local observation, including bounded live events
  and durable operation-log history for known MCP tool calls whose
  object-shaped arguments reach project handlers
- do not expose write actions from the dashboard
- do not publish the dashboard through a public reverse proxy without a separate gate covering auth, authorization, audit, rollback, and transport risk

Remote/team boundaries are tracked in
[`remote-team-mode-boundary.md`](remote-team-mode-boundary.md) and the local /
remote threat model is tracked in
[`../security/threat-model-local-and-remote.md`](../security/threat-model-local-and-remote.md).
Those documents are planning gates, not evidence that remote/team mode exists.

Recommended verification when dashboard behavior changes:

```bash
cargo test --test dashboard_config --test dashboard_recorder --test dashboard_projection --test dashboard_http
cargo test --test mcp_stdio dashboard_enabled_does_not_corrupt_mcp_stdout_and_records_tool_event -v
cargo test --test mcp_stdio dashboard_exposes_durable_operation_log_history_for_mcp_calls -v
cargo test --test mcp_stdio mcp_tool_failure_appends_failed_operation_log_without_changing_error_semantics -v
cargo test --test mcp_stdio handler_reached_missing_fields_append_failed_operation_log_without_changing_error_semantics -v
cargo test --test mcp_stdio mcp_tool_failure_does_not_persist_provider_error_payload_in_operation_log -v
cargo test --test mcp_stdio dashboard_failed_tool_event_does_not_expose_provider_error_payload -v
cargo test --test mcp_stdio non_object_mcp_tool_arguments_do_not_reach_handler_operation_log -v
```

## Daemon Gate

The daemon remains disabled by default in Local Alpha. `doctor` may report daemon configuration, but it must not start a daemon as part of the preflight check.

Before any daemon write path exists, the daemon must first pass an observe-only gate:

- config defaults keep daemon disabled
- `doctor.daemon_observe_only` reports `mode = "observe_only"`
- `doctor.daemon_observe_only` keeps `write_gate_approved = false`,
  `writes_allowed = false`, and `remote_listener_enabled = false`
- observe-only diagnostics can read local `operation_log` failure/suppression
  candidates without identity, claim, reflection, event, or commitment writes
- `serve` starts observe-only lifecycle behavior only when
  `[daemon].enabled = true`, stops it after stdio service exit, and keeps
  semantic writes closed
- observe-only mode does not call `run_reflection`
- daemon-triggered durable writes are blocked until separately gated
- any future daemon write path still uses governed `run_reflection`
- no remote listener or remote trigger ingestion is present
- no background autonomy claim is made from daemon config or doctor output

Recommended verification when daemon observe-only diagnostics change:

```bash
cargo test --test daemon_config -v
cargo test --test operation_log -v
cargo test --test mcp_stdio serve_starts_observe_only_daemon_when_enabled_without_semantic_writes -v
./scripts/agent-llm-mm.sh doctor
```

The detailed gate is
[`daemon-observe-only-gate.md`](daemon-observe-only-gate.md). The older future
daemon trigger policy does not authorize write-capable daemon behavior in Local
Alpha.

## Release Engineering Gate

Release engineering rules are documented in
[`release-engineering.md`](release-engineering.md). For this productization
stage, the conservative release artifact is a source-only Git tag plus
evidence-backed release notes.

Required boundary:

- no binary package, installer, service manager, auto-updater, remote
  bootstrapper, or packaging automation claim is made before a separate gate
- release notes point to the exact evidence directory for the candidate
- local soak evidence can be generated with
  `./scripts/release-soak-local.sh <candidate-name> [config_path]`, which writes
  candidate-specific evidence under `target/reports/releases/<candidate-name>/`
- compatibility matrix records what was actually checked instead of inferring
  platform parity
- soak evidence is required when runtime, persistence, dashboard, daemon,
  provider, or MCP behavior changes
- deprecations name the deprecated behavior, replacement path, announcement
  candidate, earliest removal candidate, and migration or rollback note

The local soak runner does not create source tags, binary packages,
installers, service-manager state, auto-updaters, Windows runner evidence, real
fresh-machine evidence, remote/team evidence, upload artifacts, release
decisions, or Local Alpha certification. This gate does not certify Beta, GA,
production support, remote write admin, remote team service, or multi-tenancy.

## Product Wording Gate

Before declaring Local Alpha complete, public docs and release notes must not claim:

- GA or production-ready status
- production self-governance
- remote write admin
- multi-tenancy
- remote team service
- all-entry automatic self-revision
- replacement of `run_reflection` as the durable identity / commitment write path

Allowed wording before this gate passes:

- validated local MVP entering productization
- Local Product Alpha in progress
- local MCP `stdio` memory service
- controlled self-revision MVP with `run_reflection` as the durable write path

Allowed wording after this gate passes:

- Local Product Alpha
- local-only product alpha
- installable local alpha, if the install/bootstrap and product smoke evidence are also complete

## Release Decision

Local Alpha is not complete unless all sections above have fresh evidence. If a
section lacks fresh evidence, record it as an open or not-verified gate item
rather than weakening the gate. The Product Smoke Gate is implemented by
`scripts/product-smoke-local.sh`; the Local Alpha Release-Gate Refresh can
refresh locally reproducible gate artifacts; and the Local Alpha Evidence
Summary Gate can summarize current gate status. None of these certifies a
release unless the current successful run provides the required evidence. If any
future gate section remains unimplemented, record that section separately as
open.
