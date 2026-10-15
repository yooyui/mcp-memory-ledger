#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: scripts/first-run-bootstrap-smoke-local.sh [output_dir]

Runs a local-only first-run bootstrap simulation:
  1. ./scripts/agent-llm-mm.sh bootstrap-local <output_dir>/agent-llm-mm.local.toml
  2. rewrites database_url to <output_dir>/first-run.sqlite
  3. ./scripts/agent-llm-mm.sh init <output_dir>/agent-llm-mm.local.toml
  4. ./scripts/agent-llm-mm.sh doctor --read-only <output_dir>/agent-llm-mm.local.toml
  5. writes init.json, doctor.json and summary.json under output_dir

This is fresh-machine simulation evidence only. It does not start serve, run the
product smoke script, prove a real fresh-machine install, or prove Windows
runtime parity.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 1 ]]; then
  usage
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
project_root="$(cd "${script_dir}/.." && pwd -P)"
requested_output_dir="${1:-}"

cd "${project_root}"

if [[ -z "${requested_output_dir}" ]]; then
  default_parent="${project_root}/target/first-run-bootstrap-smoke"
  mkdir -p "${default_parent}"
  output_dir="$(mktemp -d "${default_parent}/run.XXXXXX")"
else
  if [[ "${requested_output_dir}" = /* ]]; then
    output_dir="${requested_output_dir}"
  else
    output_dir="${project_root}/${requested_output_dir}"
  fi

  if [[ -L "${output_dir}" ]]; then
    printf 'first-run bootstrap smoke failed: output directory must not be a symlink: %s\n' "${output_dir}" >&2
    exit 1
  fi
  if [[ -e "${output_dir}" && ! -d "${output_dir}" ]]; then
    printf 'first-run bootstrap smoke failed: output path exists but is not a directory: %s\n' "${output_dir}" >&2
    exit 1
  fi
  if [[ -d "${output_dir}" ]] && find "${output_dir}" -mindepth 1 -maxdepth 1 -print -quit | grep -q .; then
    printf 'first-run bootstrap smoke failed: output directory is not empty: %s\n' "${output_dir}" >&2
    exit 1
  fi
  mkdir -p "${output_dir}"
  output_dir="$(cd "${output_dir}" && pwd -P)"
fi

config_path="${output_dir}/agent-llm-mm.local.toml"
database_path="${output_dir}/first-run.sqlite"
database_url="sqlite://${database_path//\\//}"
doctor_path="${output_dir}/doctor.json"
init_path="${output_dir}/init.json"
summary_path="${output_dir}/summary.json"
bootstrap_stdout="${output_dir}/bootstrap.stdout"
bootstrap_stderr="${output_dir}/bootstrap.stderr"
doctor_stderr="${output_dir}/doctor.stderr"
init_stderr="${output_dir}/init.stderr"

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

toml_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

replace_database_url() {
  local temp_config="${config_path}.tmp"
  local replaced=0
  local escaped_database_url
  escaped_database_url="$(toml_escape "${database_url}")"

  while IFS= read -r line || [[ -n "${line}" ]]; do
    if [[ "${line}" == database_url\ =* ]]; then
      printf 'database_url = "%s"\n' "${escaped_database_url}" >> "${temp_config}"
      replaced=1
    else
      printf '%s\n' "${line}" >> "${temp_config}"
    fi
  done < "${config_path}"

  if [[ "${replaced}" -ne 1 ]]; then
    rm -f "${temp_config}"
    printf 'first-run bootstrap smoke failed: generated config did not contain database_url\n' >&2
    exit 1
  fi

  mv "${temp_config}" "${config_path}"
}

require_doctor_field() {
  local pattern="$1"
  local description="$2"

  if ! grep -q "${pattern}" "${doctor_path}"; then
    printf 'first-run bootstrap smoke failed: doctor.json missing %s\n' "${description}" >&2
    exit 1
  fi
}

env -u AGENT_LLM_MM_CONFIG -u AGENT_LLM_MM_DATABASE_URL \
  ./scripts/agent-llm-mm.sh bootstrap-local "${config_path}" > "${bootstrap_stdout}" 2> "${bootstrap_stderr}"
replace_database_url

env -u AGENT_LLM_MM_CONFIG -u AGENT_LLM_MM_DATABASE_URL \
  ./scripts/agent-llm-mm.sh init "${config_path}" > "${init_path}" 2> "${init_stderr}"

env -u AGENT_LLM_MM_CONFIG -u AGENT_LLM_MM_DATABASE_URL \
  ./scripts/agent-llm-mm.sh doctor --read-only "${config_path}" > "${doctor_path}" 2> "${doctor_stderr}"

require_doctor_field '"status": "ok"' 'status ok'
require_doctor_field '"provider": "mock"' 'mock provider'
require_doctor_field '"self_revision_write_path": "run_reflection"' 'run_reflection write path'
require_doctor_field '"daemon_enabled": false' 'disabled daemon'
require_doctor_field '"mode": "observe_only"' 'observe-only daemon mode'
require_doctor_field '"write_gate_approved": false' 'closed daemon write gate'
require_doctor_field '"writes_allowed": false' 'observe-only daemon write gate'
require_doctor_field '"remote_listener_enabled": false' 'disabled remote daemon listener'

if [[ ! -f "${database_path}" ]]; then
  printf 'first-run bootstrap smoke failed: sqlite database was not created: %s\n' "${database_path}" >&2
  exit 1
fi

escaped_output_dir="$(json_escape "${output_dir}")"
escaped_config_path="$(json_escape "${config_path}")"
escaped_database_path="$(json_escape "${database_path}")"
escaped_database_url="$(json_escape "${database_url}")"

cat > "${summary_path}" <<SUMMARY
{
  "kind": "local_first_run_bootstrap_simulation",
  "local_only": true,
  "fresh_machine_simulation": true,
  "real_fresh_machine_evidence": false,
  "output_dir": "${escaped_output_dir}",
  "config_path": "${escaped_config_path}",
  "database_path": "${escaped_database_path}",
  "database_url": "${escaped_database_url}",
  "init_path": "init.json",
  "doctor_path": "doctor.json",
  "doctor_status": "ok",
  "provider": "mock",
  "self_revision_write_path": "run_reflection",
  "daemon_enabled": false,
  "daemon_observe_only": {
    "mode": "observe_only",
    "write_gate_approved": false,
    "writes_allowed": false,
    "remote_listener_enabled": false
  },
  "daemon_writes_allowed": false,
  "sqlite_database_exists": true,
  "started_serve": false,
  "ran_product_smoke": false,
  "windows_runtime_parity": false
}
SUMMARY

printf 'first-run bootstrap smoke: local simulation passed under %s\n' "${output_dir}"
printf 'first-run bootstrap smoke: init evidence: %s\n' "${init_path}"
printf 'first-run bootstrap smoke: doctor evidence: %s\n' "${doctor_path}"
printf 'first-run bootstrap smoke: summary evidence: %s\n' "${summary_path}"
