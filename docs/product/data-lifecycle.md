# Data Lifecycle

This document covers local data handling for the current validated local MVP
entering productization. It does not claim GA readiness, production-grade
autonomy, remote write administration, or multi-tenant operation. The durable
identity, commitment, and reflection write path remains `run_reflection`.

## Data Classes

Use separate `database_url` values for formal, test, and demo data. Do not reuse
one SQLite file across these classes.

| Class | Intended use | Example shape |
| --- | --- | --- |
| formal | Local data the operator intends to preserve, inspect, backup, or use for productization readiness checks | `sqlite:///Users/<you>/agent-llm-mm/formal/agent-llm-mm.sqlite` |
| test | Manual smoke checks, local debugging, migration trials, and integration experiments | `sqlite:///Users/<you>/agent-llm-mm/test/manual-test.sqlite` |
| demo | Deterministic demo runs and generated evidence artifacts | `sqlite:///Users/<you>/agent-llm-mm/demo/demo.sqlite` |

Set the separation in a private config file such as
`agent-llm-mm.local.toml`, or with `AGENT_LLM_MM_DATABASE_URL` when a one-off
command needs an override. Example profile files are templates; copy them before
using a real local path.

## Isolation Rules

- Formal data must not be used by demo runners, smoke-test staging directories,
  throwaway migration tests, or provider stub experiments.
- Test data may be deleted, recreated, or migrated while validating behavior.
  Keep a test database only when it is needed to reproduce a specific issue.
- Demo data belongs to the generated demo evidence directory or an explicitly
  named demo database path. Demo runs must remain deterministic and local-only.
- `prod-local` configuration means "local formal data profile"; it is not a
  hosted production deployment or a multi-user service boundary.
- Dashboard and support-bundle flows remain local diagnostic surfaces. They do
  not grant remote write management and should not be exposed as public admin
  interfaces without a separate product gate.

## Explicit Database Lifecycle

The product entrypoints do not change the database implicitly:

- `init` creates a missing file with the current schema, migration ledger,
  baseline commitment, and default identity. It refuses an existing file.
- `migrate` requires an existing file. It creates a pre-migration backup,
  migrates a restored rehearsal copy first, rejects a moving source, and then
  migrates the original in one transaction with row-count and foreign-key
  checks.
- `doctor` and `doctor --read-only` are equivalent no-write inspections.
  Missing, old, newer-than-supported, incomplete, and read-only databases are
  reported without create, migration, or seed.
- `doctor --allow-bootstrap` is the only doctor form allowed to initialize or
  migrate, and that permission is explicit in the command and JSON report.
- `serve` requires a current database and otherwise fails with the required
  lifecycle command.

macOS:

```bash
./scripts/agent-llm-mm.sh init agent-llm-mm.local.toml
./scripts/agent-llm-mm.sh doctor --read-only agent-llm-mm.local.toml
./scripts/agent-llm-mm.sh migrate agent-llm-mm.local.toml
```

The matching PowerShell modes are `init`, `doctor --read-only`, and `migrate`.
`PRAGMA user_version` and `schema_migrations` are the authoritative version and
ledger pair; the current schema version is reported by every lifecycle command.

## Backup

Run SQLite backup before schema migration work, before changing a formal
`database_url`, and before packaging or inspecting local formal data for a
productization gate.

```bash
./scripts/backup-sqlite.sh "sqlite:///absolute/path/to/formal.sqlite"
```

By default, backups are written under:

```text
target/backups/sqlite/
```

The helper accepts a second argument for a different backup directory:

```bash
./scripts/backup-sqlite.sh \
  "sqlite:///absolute/path/to/formal.sqlite" \
  /safe/local/backup-dir
```

The helper is intentionally conservative: it refuses empty database paths,
refuses a backup directory inside the live database directory tree, refuses
unsafe path shapes, and refuses to overwrite an existing backup file. When
`sqlite3` is available, it uses SQLite's online backup command. If `sqlite3` is
missing, it falls back to file copy and prints a warning; avoid the fallback for
a busy database unless the local MCP service is stopped.

Backup files are tightened to owner read/write permissions. When a checksum tool
is available, the helper writes a sidecar `.sha256` file. Missing checksum tools
do not fail the backup, but the command reports that checksum generation was
skipped.

## Restore

Default restore rule: restore to a new path first. Do not restore over the
current formal SQLite file. Restore rejects in-memory SQLite targets, invalid
SQLite URL percent encoding, unsafe path characters, `..` target path
components, and any target path that already exists.

```bash
./scripts/restore-sqlite.sh \
  target/backups/sqlite/formal.sqlite.20260511-120000.12345.bak \
  "sqlite:///absolute/path/to/restore-check/formal-restore.sqlite"
```

After restore, point a private config at the restored database:

```toml
database_url = "sqlite:///absolute/path/to/restore-check/formal-restore.sqlite"
```

Then run read-only `doctor` against the restored path. On macOS:

```bash
AGENT_LLM_MM_CONFIG=/absolute/path/to/restore-check.toml ./scripts/agent-llm-mm.sh doctor --read-only
```

On Windows, run the same restore helper from Git Bash, WSL, or an equivalent
bash environment, then validate with the PowerShell entrypoint:

```powershell
$env:AGENT_LLM_MM_CONFIG = 'D:/agent-llm-mm/restore-check.toml'
pwsh -File .\scripts\agent-llm-mm.ps1 doctor --read-only
Remove-Item Env:\AGENT_LLM_MM_CONFIG
```

Only after the restored database has been validated should a human decide
whether to update the formal `database_url`. Keep the original formal database
untouched until that decision is made.

## Export Boundary

There is no general full-database export contract for Local Product Alpha yet.
Treat export as bounded, local, and explicit:

- Support bundles may include redacted configuration shape, bounded
  `operation_log` summaries, release metadata, and product smoke summary.
- Support bundles must not include full SQLite files, raw TOML, API keys,
  provider payloads, private prompts, browser sessions, SSH keys, or remote
  credentials by default.
- Demo artifacts may include the deterministic demo SQLite file inside the demo
  output directory. That file is demo evidence, not formal user data export.
- If a maintainer needs a formal SQLite database to reproduce a bug, that is a
  separate manual action after backup, redaction review, and explicit operator
  approval.
- Exporting summaries does not create a new durable write path. Durable
  identity, commitment, and reflection writes continue to go through
  `run_reflection`.

## Retention Expectations

| Data or artifact | Current expectation |
| --- | --- |
| `events` | Preserve in formal data as evidence history. Do not truncate during routine cleanup; use a copied test database for destructive migration trials. |
| `claims` and `evidence_links` | Preserve in formal data because they bind inferred or observed claims to evidence. Rebuild only through verified schema migration or restore-to-new-path checks. |
| `reflections` | Preserve in formal data as audit history for `run_reflection`. Conflict-only or rejected paths should not be cleaned up by deleting history. |
| `identity_claims` and `commitments` | Preserve as the current durable self-state. They should be changed only by governed application paths, with `run_reflection` remaining the durable productization boundary. |
| `reflection_trigger_ledger` | Preserve in formal data for cooldown, handled-window, and self-revision diagnostics. |
| `operation_log` | Preserve recent formal operation metadata for local diagnostics. Support bundles may expose bounded summaries, not raw payloads or full logs. |
| Demo artifacts under `target/reports/self-revision-demo/` | Keep timestamped/manual runs when they are evidence for a review or release gate. `latest` is managed by `product-smoke-local.sh` and may be replaced by a successful smoke run. |
| Backup files under `target/backups/sqlite/` | Keep until the related migration, restore, or release-gate review is complete. Move long-lived formal backups outside volatile build output if they must survive workspace cleanup. |
| Test databases | Disposable after the test objective is complete unless they reproduce a bug. |

## Schema Migration Verification

Before touching a formal database:

1. Confirm the exact formal `database_url`.
2. Stop local processes that may be writing to the database, or confirm the
   backup helper will use SQLite online backup.
3. Run `./scripts/backup-sqlite.sh` and keep the backup path.
4. Restore the backup to a new path with `./scripts/restore-sqlite.sh`.
5. Point a private config at the restored path and run `doctor --read-only`.

For migration implementation or verification:

1. Run `doctor --read-only` against a test or restored database first, then run
   explicit `migrate`; do not start with the formal live path.
2. Confirm required tables exist: `events`, `claims`, `evidence_links`,
   `episode_events`, `reflections`, `reflection_trigger_ledger`,
   `identity_claims`, `commitments`, `operation_log`, and `schema_migrations`.
3. Confirm namespace constraints and legacy backfills for `events` and `claims`
   still match current code expectations.
4. Confirm reflection audit columns still exist:
   `supporting_evidence_event_ids`, `requested_identity_update`, and
   `requested_commitment_updates`.
5. Run the fast repository checks that cover SQLite behavior for the changed
   code path, then run the platform `doctor` command against the migrated
   restored database.
6. If validation fails, do not switch formal `database_url`; keep using the
   original formal database and investigate from the restored/test copy.

Useful local checks:

```bash
cargo test --test sqlite_backup_restore
cargo test --test sqlite_lifecycle
cargo test --test sqlite_store
git diff --check
./scripts/agent-llm-mm.sh doctor --read-only /absolute/path/to/restore-check.toml
```

On Windows, keep the platform command shape separate:

```powershell
cargo test --test sqlite_backup_restore
cargo test --test sqlite_lifecycle
cargo test --test sqlite_store
git diff --check
pwsh -File .\scripts\agent-llm-mm.ps1 doctor --read-only .\restore-check.toml
```
