#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: ./scripts/release-soak-local.sh <candidate-name> [config_path]

Runs a bounded local release soak and writes candidate-specific evidence under:
  target/reports/releases/<candidate-name>/

The soak covers:
  1. explicit init of a candidate-isolated SQLite database
  2. ./scripts/agent-llm-mm.sh doctor --read-only [config_path]
  3. cargo test --test dashboard_http -v
  4. scripts/product-smoke-local.sh [config_path]
  5. scripts/first-run-bootstrap-smoke-local.sh target/first-run-bootstrap-smoke/local-alpha-gate
  6. scripts/generate-support-bundle.sh target/support-bundles/local-alpha-gate [config_path]
  7. support-bundle secret and raw-artifact scans
  8. support-bundle and product-smoke SHA-256 manifests
  9. scripts/local-alpha-evidence-summary.sh into the release evidence directory
  10. compatibility-matrix.json and release-boundaries.json blocker artifacts

This command only creates local release evidence. It does not create Windows
runner evidence, real fresh-machine evidence, remote/team evidence, uploads,
daemon write capability, service-manager state, or release certification.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage
  exit 2
fi

candidate_name="$1"
config_path="${2:-}"

if [[ ! "${candidate_name}" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ || "${candidate_name}" == *..* ]]; then
  printf 'release soak failed: candidate name must use only letters, numbers, dot, underscore, or dash, must not start with dot, and must not contain "..": %s\n' "${candidate_name}" >&2
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
project_root="$(cd "${script_dir}/.." && pwd -P)"
resolved_config_path=""
redacted_config_path="default config"

if [[ -n "${config_path}" ]]; then
  if [[ ! -e "${config_path}" ]]; then
    printf 'release soak failed: config path does not exist: %s\n' "${config_path}" >&2
    exit 2
  fi
  if [[ "${config_path}" = /* ]]; then
    resolved_config_path="${config_path}"
  else
    resolved_config_path="$(cd "$(dirname "${config_path}")" && pwd -P)/$(basename "${config_path}")"
  fi
  redacted_config_path="<local-path>/<redacted-name>"
fi

cd "${project_root}"

evidence_dir="target/reports/releases/${candidate_name}"
commands_dir="${evidence_dir}/commands"
command_summary="${evidence_dir}/command-summary.tsv"
soak_log="${evidence_dir}/release-soak.log"
support_bundle_dir="target/support-bundles/local-alpha-gate"
first_run_dir="target/first-run-bootstrap-smoke/local-alpha-gate"
secret_scan_log="${evidence_dir}/secret-scan.log"
artifact_scan_log="${evidence_dir}/artifact-scan.log"
release_summary_md="${evidence_dir}/release-soak-summary.md"
local_alpha_summary_json="${evidence_dir}/local-alpha-evidence-summary.json"
local_alpha_summary_md="${evidence_dir}/local-alpha-evidence-summary.md"
support_bundle_sha256="${evidence_dir}/support-bundle-sha256.txt"
product_smoke_latest_sha256="${evidence_dir}/product-smoke-latest-sha256.txt"
compatibility_matrix_json="${evidence_dir}/compatibility-matrix.json"
release_boundaries_json="${evidence_dir}/release-boundaries.json"
runtime_dir="target/release-soak-runtime/${candidate_name}"
isolated_database_path="${project_root}/${runtime_dir}/release-soak.sqlite"
isolated_database_url="sqlite://${isolated_database_path//\\//}"

if [[ -e "${evidence_dir}" && -n "$(find "${evidence_dir}" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
  printf 'release soak failed: evidence directory must be absent or empty: %s\n' "${evidence_dir}" >&2
  exit 2
fi
if [[ -e "${runtime_dir}" && -n "$(find "${runtime_dir}" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
  printf 'release soak failed: isolated runtime directory must be absent or empty: %s\n' "${runtime_dir}" >&2
  exit 2
fi

mkdir -p "${commands_dir}" "${runtime_dir}"

case "${isolated_database_path}" in
  "${project_root}/target/release-soak-runtime/${candidate_name}/"*)
    ;;
  *)
    printf 'release soak failed: isolated database escaped candidate runtime directory\n' >&2
    exit 2
    ;;
esac
if [[ -e "${isolated_database_path}" ]]; then
  printf 'release soak failed: isolated database already exists: %s\n' "${isolated_database_path}" >&2
  exit 2
fi

started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'step\tstarted_at\tended_at\texit_code\tcommand\n' > "${command_summary}"
printf 'release soak: candidate=%s started_at=%s\n' "${candidate_name}" "${started_at}" | tee "${soak_log}"

git rev-parse HEAD > "${evidence_dir}/git-head.txt"
git status --short --branch > "${evidence_dir}/git-status-before.txt"
rustc --version > "${evidence_dir}/rustc-version.txt" 2>&1 || true
cargo --version > "${evidence_dir}/cargo-version.txt" 2>&1 || true

quote_command_redacted() {
  local arg
  for arg in "$@"; do
    if [[ -n "${resolved_config_path}" && "${arg}" == "${resolved_config_path}" ]]; then
      printf '%q ' "${redacted_config_path}"
    else
      printf '%q ' "${arg}"
    fi
  done
}

redact_release_evidence_file() {
  local file="$1"
  [[ -f "${file}" ]] || return 0
  if [[ -n "${resolved_config_path}" ]]; then
    AGENT_LLM_MM_RELEASE_SOAK_RAW_CONFIG="${resolved_config_path}" \
      AGENT_LLM_MM_RELEASE_SOAK_REDACTED_CONFIG="${redacted_config_path}" \
      perl -0pi -e 'BEGIN { $raw=$ENV{"AGENT_LLM_MM_RELEASE_SOAK_RAW_CONFIG"}; $red=$ENV{"AGENT_LLM_MM_RELEASE_SOAK_REDACTED_CONFIG"}; } s/\Q$raw\E/$red/g if length($raw);' "${file}"
  fi
  perl -0pi -e 's#sqlite://[^\s",)`]+#sqlite://<local-path>#g; s#https?://[^/\s"`]+:[^@\s"`]+@#https://<redacted>@#g; s#([?&](?:api_key|token|secret|password|access_token|refresh_token|provider_token|client_secret)=)[^&\s"`]+#${1}<redacted>#gi; s#(Bearer\s+)[A-Za-z0-9._~+/=-]+#${1}<redacted>#gi; s#sk-[A-Za-z0-9._-]+#sk-<redacted>#g; s#((?:api_key|api-key|x-api-key|provider_token|openai_api_key|password|secret|token)\s*[:=]\s*)[^,\s"`}]+#${1}<redacted>#gi' "${file}"
}

run_step() {
  local step_name="$1"
  shift
  local log_path="${commands_dir}/${step_name}.log"
  local step_started_at
  local step_ended_at
  local status

  step_started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'release soak: running %s\n' "${step_name}" | tee -a "${soak_log}"
  printf '$ ' > "${log_path}"
  quote_command_redacted "$@" >> "${log_path}"
  printf '\n\n' >> "${log_path}"

  set +e
  "$@" >> "${log_path}" 2>&1
  status=$?
  set -e
  redact_release_evidence_file "${log_path}"

  step_ended_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf '%s\t%s\t%s\t%s\t' "${step_name}" "${step_started_at}" "${step_ended_at}" "${status}" >> "${command_summary}"
  quote_command_redacted "$@" >> "${command_summary}"
  printf '\n' >> "${command_summary}"

  if [[ "${status}" -ne 0 ]]; then
    printf 'release soak failed: %s exited with %s; see %s\n' "${step_name}" "${status}" "${log_path}" >&2
    exit "${status}"
  fi
}

write_sha256_manifest() {
  local source_dir="$1"
  local output_file="$2"
  local has_files=0

  : > "${output_file}"
  if command -v shasum >/dev/null 2>&1; then
    while IFS= read -r file; do
      has_files=1
      shasum -a 256 "${file}" >> "${output_file}"
    done < <(find "${source_dir}" -maxdepth 1 -type f -print | sort)
  elif command -v sha256sum >/dev/null 2>&1; then
    while IFS= read -r file; do
      has_files=1
      sha256sum "${file}" >> "${output_file}"
    done < <(find "${source_dir}" -maxdepth 1 -type f -print | sort)
  else
    printf 'release soak failed: no shasum or sha256sum command found for SHA-256 evidence\n' >&2
    exit 2
  fi
  if [[ "${has_files}" -eq 0 ]]; then
    printf 'release soak failed: no files found for SHA-256 evidence in %s\n' "${source_dir}" >&2
    exit 1
  fi
}

isolated_env=(env -u AGENT_LLM_MM_CONFIG "AGENT_LLM_MM_DATABASE_URL=${isolated_database_url}")
init_command=("${isolated_env[@]}" ./scripts/agent-llm-mm.sh init)
doctor_command=("${isolated_env[@]}" ./scripts/agent-llm-mm.sh doctor --read-only)
product_smoke_command=("${isolated_env[@]}" scripts/product-smoke-local.sh)
support_bundle_command=("${isolated_env[@]}" scripts/generate-support-bundle.sh "${support_bundle_dir}")

if [[ -n "${resolved_config_path}" ]]; then
  init_command+=("${resolved_config_path}")
  doctor_command+=("${resolved_config_path}")
  product_smoke_command+=("${resolved_config_path}")
  support_bundle_command+=("${resolved_config_path}")
fi

run_step database-init "${init_command[@]}"
if [[ ! -f "${isolated_database_path}" ]]; then
  printf 'release soak failed: explicit init did not create the isolated database\n' >&2
  exit 1
fi
run_step doctor "${doctor_command[@]}"
run_step dashboard-http cargo test --test dashboard_http -v
run_step product-smoke "${product_smoke_command[@]}"

rm -rf "${first_run_dir}"
run_step first-run-bootstrap-smoke scripts/first-run-bootstrap-smoke-local.sh "${first_run_dir}"

rm -rf "${support_bundle_dir}"
run_step support-bundle "${support_bundle_command[@]}"

printf 'release soak: scanning support bundle for secret-like markers\n' | tee -a "${soak_log}"
set +e
rg -n 'api_key|Authorization|Bearer|sk-|provider_token|openai_api_key|password|secret|sqlite:///' "${support_bundle_dir}" > "${secret_scan_log}" 2>&1
secret_scan_status=$?
set -e
if [[ "${secret_scan_status}" -eq 0 ]]; then
  printf 'release soak failed: secret-like marker found in support bundle; see %s\n' "${secret_scan_log}" >&2
  exit 1
fi
if [[ "${secret_scan_status}" -gt 1 ]]; then
  printf 'release soak failed: secret scan command failed; see %s\n' "${secret_scan_log}" >&2
  exit "${secret_scan_status}"
fi

printf 'release soak: scanning support bundle for raw SQLite, TOML, or log artifacts\n' | tee -a "${soak_log}"
find "${support_bundle_dir}" \( -name '*.sqlite' -o -name '*.toml' -o -name '*.log' \) -print > "${artifact_scan_log}"
if [[ -s "${artifact_scan_log}" ]]; then
  printf 'release soak failed: support bundle contains raw artifacts; see %s\n' "${artifact_scan_log}" >&2
  exit 1
fi

find "${support_bundle_dir}" -maxdepth 1 -type f -print | sort > "${evidence_dir}/support-bundle-files.txt"
find target/reports/self-revision-demo/latest -maxdepth 1 -type f -print | sort > "${evidence_dir}/product-smoke-latest-files.txt"
write_sha256_manifest "${support_bundle_dir}" "${support_bundle_sha256}"
write_sha256_manifest "target/reports/self-revision-demo/latest" "${product_smoke_latest_sha256}"

run_step local-alpha-evidence-summary scripts/local-alpha-evidence-summary.sh \
  --evidence-root . \
  --output-json "${local_alpha_summary_json}" \
  --output-md "${local_alpha_summary_md}"

rust_toolchain_version="$(tr -d '\n' < "${evidence_dir}/rustc-version.txt")"
case "$(uname -s)" in
  Darwin)
    local_platform="macOS"
    ;;
  *)
    local_platform="$(uname -s)"
    ;;
esac
cat > "${compatibility_matrix_json}" <<EOF
{
  "kind": "release_compatibility_matrix",
  "candidate": "${candidate_name}",
  "local_only": true,
  "rows": [
    {
      "platform": "${local_platform}",
      "shell_wrapper": "scripts/agent-llm-mm.sh",
      "rust_toolchain_version": "${rust_toolchain_version}",
      "config_path_shape": "${redacted_config_path}",
      "sqlite_persistence_path_shape": "sqlite://<local-path>",
      "provider_mode_checked": "deterministic demo",
      "dashboard_status": "local-only test checked",
      "daemon_status": "disabled / observe-only boundary checked",
      "result": "passed",
      "evidence_path": "${evidence_dir}/command-summary.tsv"
    },
    {
      "platform": "Windows",
      "shell_wrapper": "scripts/agent-llm-mm.ps1",
      "rust_toolchain_version": null,
      "sqlite_persistence_path_shape": "not_checked",
      "provider_mode_checked": "not_checked",
      "dashboard_status": "not_checked",
      "daemon_status": "not_checked",
      "result": "not_checked",
      "evidence_path": null,
      "reason": "release-soak-local.sh does not create Windows runner or Windows machine evidence"
    }
  ]
}
EOF

cat > "${release_boundaries_json}" <<EOF
{
  "kind": "release_boundaries",
  "candidate": "${candidate_name}",
  "product_boundary": "local Rust MCP stdio memory MVP / technical demo entering productization",
  "local_only": true,
  "database_isolation": {
    "enforced": true,
    "path_shape": "sqlite://<candidate-isolated-path>",
    "formal_database_path_accepted": false
  },
  "external_blockers": [
    {
      "subject": "fresh_machine",
      "status": "blocked",
      "evidence_path": null,
      "reason": "local soak does not create real fresh-machine clone, unpack, install, or bootstrap evidence"
    },
    {
      "subject": "windows_parity",
      "status": "not_checked",
      "evidence_path": null,
      "reason": "local soak does not run on a Windows runner or Windows machine"
    }
  ],
  "human_blockers": [
    {
      "subject": "release_decision",
      "status": "required",
      "evidence_path": null,
      "reason": "human release decision must be recorded separately"
    }
  ],
  "unimplemented_capability_blockers": [
    {
      "subject": "remote_team",
      "status": "blocked",
      "evidence_path": null,
      "reason": "remote/team memory product behavior is not implemented"
    },
    {
      "subject": "security_auth",
      "status": "blocked",
      "evidence_path": null,
      "reason": "auth, authorization, audit, rate limit, tenant isolation, and rollback gates are not implemented"
    },
    {
      "subject": "daemon_writes",
      "status": "blocked",
      "evidence_path": null,
      "reason": "daemon write capability is not implemented; daemon diagnostics remain observe-only and write-closed"
    },
    {
      "subject": "release_packaging",
      "status": "blocked",
      "evidence_path": null,
      "reason": "binary packaging, installer, service manager, and auto-updater evidence are not implemented"
    }
  ]
}
EOF

git status --short --branch > "${evidence_dir}/git-status-after.txt"
ended_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

cat > "${release_summary_md}" <<EOF
# Local Release Soak Evidence

- candidate: \`${candidate_name}\`
- started_at: \`${started_at}\`
- ended_at: \`${ended_at}\`
- evidence_dir: \`${evidence_dir}\`
- config_path_shape: \`${redacted_config_path}\`
- database_isolation: \`candidate-specific / enforced\`
- database_path_shape: \`sqlite://<candidate-isolated-path>\`
- boundary: local-only release evidence; not Windows parity, real fresh-machine evidence, remote/team evidence, upload, release certification, or GA readiness

## Evidence Files

- \`git-head.txt\`
- \`git-status-before.txt\`
- \`git-status-after.txt\`
- \`command-summary.tsv\`
- \`commands/\`
- \`secret-scan.log\`
- \`artifact-scan.log\`
- \`support-bundle-files.txt\`
- \`support-bundle-sha256.txt\`
- \`product-smoke-latest-files.txt\`
- \`product-smoke-latest-sha256.txt\`
- \`local-alpha-evidence-summary.json\`
- \`local-alpha-evidence-summary.md\`
- \`compatibility-matrix.json\`
- \`release-boundaries.json\`
EOF

redact_release_evidence_file "${command_summary}"
redact_release_evidence_file "${release_summary_md}"

printf 'release soak: scanning release evidence for secret-like markers\n' | tee -a "${soak_log}"
set +e
rg -n 'api_key|api-key|x-api-key|Authorization|Bearer|sk-|provider_token|openai_api_key|password|sqlite:///|token=' \
  "${evidence_dir}" \
  --glob '!secret-scan.log' \
  --glob '!artifact-scan.log' \
  --glob '!release-soak.log' \
  > "${secret_scan_log}" 2>&1
secret_scan_status=$?
set -e
if [[ "${secret_scan_status}" -eq 0 ]]; then
  printf 'release soak failed: secret-like marker found in release evidence; see %s\n' "${secret_scan_log}" >&2
  exit 1
fi
if [[ "${secret_scan_status}" -gt 1 ]]; then
  printf 'release soak failed: release evidence secret scan command failed; see %s\n' "${secret_scan_log}" >&2
  exit "${secret_scan_status}"
fi

printf 'release soak: evidence written to %s\n' "${evidence_dir}" | tee -a "${soak_log}"
printf 'release soak: summary: %s\n' "${release_summary_md}" | tee -a "${soak_log}"
