#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: ./scripts/release-evidence-index.sh <release-candidate> [evidence_root] [output_dir]

Builds a local read-only Release Evidence Index from existing Local Alpha and
product-readiness evidence. It does not create missing evidence, approve a
release, upload files, call network endpoints, start serve, or run remote
commands.

When output_dir is provided, writes:
  <output_dir>/release-evidence-index.json
  <output_dir>/release-evidence-index.md
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 || $# -gt 3 ]]; then
  usage
  exit 2
fi

release_candidate="$1"
evidence_root="${2:-.}"
output_dir="${3:-}"

case "${release_candidate}" in
  *[!A-Za-z0-9._-]* | "" | .* | *..* )
    echo "candidate name must contain only letters, numbers, dot, underscore, and dash, and must not contain path traversal" >&2
    exit 2
    ;;
esac

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
project_root="$(cd "${script_dir}/.." && pwd -P)"
cd "${project_root}"

args=(--release-candidate "${release_candidate}" --evidence-root "${evidence_root}")
if [[ -n "${output_dir}" ]]; then
  mkdir -p "${output_dir}"
  args+=(--output-json "${output_dir}/release-evidence-index.json")
  args+=(--output-md "${output_dir}/release-evidence-index.md")
fi

cargo run --quiet --features release-tools --bin release_evidence_index -- "${args[@]}"
