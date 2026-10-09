#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: ./scripts/packaging-preflight-check.sh <release-candidate> [evidence_root] [output_dir]

Summarizes existing packaging evidence for a release candidate. This is a
read-only preflight over existing local files.

When output_dir is provided, writes:
  <output_dir>/packaging-preflight-summary.json
  <output_dir>/packaging-preflight-summary.md
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
  args+=(--output-json "${output_dir}/packaging-preflight-summary.json")
  args+=(--output-md "${output_dir}/packaging-preflight-summary.md")
fi

cargo run --quiet --features release-tools --bin packaging_preflight_check -- "${args[@]}"
