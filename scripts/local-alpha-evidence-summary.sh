#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: ./scripts/local-alpha-evidence-summary.sh [--evidence-root <path>] [--output-json <path>] [--output-md <path>]

Summarizes existing Local Product Alpha gate evidence into JSON and optional
Markdown. This is a local read-only rollup: it does not start the MCP server,
run product smoke, upload files, start daemon behavior, or perform durable
reflection writes.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
project_root="$(cd "${script_dir}/.." && pwd -P)"

cd "${project_root}"

cargo run --quiet --features release-tools --bin local_alpha_evidence_summary -- "$@"
