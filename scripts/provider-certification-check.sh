#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: ./scripts/provider-certification-check.sh [config_path] [evidence_root] [output_dir]

Runs the local provider certification preflight. This is a read-only evidence
summary. It does not perform network or remote operations, or create live
certification evidence.

When output_dir is provided, writes:
  <output_dir>/provider-certification-summary.json
  <output_dir>/provider-certification-summary.md
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 3 ]]; then
  usage
  exit 2
fi

config_path="${1:-}"
evidence_root="${2:-.}"
output_dir="${3:-}"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
project_root="$(cd "${script_dir}/.." && pwd -P)"
cd "${project_root}"

args=(--evidence-root "${evidence_root}")
if [[ -n "${config_path}" ]]; then
  args+=(--config-path "${config_path}")
fi
if [[ -n "${output_dir}" ]]; then
  mkdir -p "${output_dir}"
  args+=(--output-json "${output_dir}/provider-certification-summary.json")
  args+=(--output-md "${output_dir}/provider-certification-summary.md")
fi

cargo run --quiet --features release-tools --bin provider_certification_check -- "${args[@]}"
