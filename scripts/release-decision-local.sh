#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: ./scripts/release-decision-local.sh <release-candidate> [evidence_root] [decision] [human_reviewer] [rollback_note]

Writes a local source-only release decision artifact under:
  target/reports/releases/<release-candidate>/release-decision.json
  target/reports/releases/<release-candidate>/release-decision.md

The default decision is "blocked". "rejected" and "deferred" record explicit
non-approval decisions. "approved" is rejected unless the current Local Alpha
evidence summary is ready for human review and reviewer/rollback fields are
present. This script does not tag, package, upload, run remote commands, or
create missing evidence.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 || $# -gt 5 ]]; then
  usage
  exit 2
fi

release_candidate="$1"
evidence_root="${2:-.}"
decision="${3:-blocked}"
human_reviewer="${4:-}"
rollback_note="${5:-}"

case "${release_candidate}" in
  *[!A-Za-z0-9._-]* | "" | .* | *..* )
    echo "candidate name must contain only letters, numbers, dot, underscore, and dash, and must not contain path traversal" >&2
    exit 2
    ;;
esac

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
project_root="$(cd "${script_dir}/.." && pwd -P)"
cd "${project_root}"

output_dir="target/reports/releases/${release_candidate}"
mkdir -p "${output_dir}"

args=(
  --release-candidate "${release_candidate}"
  --evidence-root "${evidence_root}"
  --decision "${decision}"
  --output-json "${output_dir}/release-decision.json"
  --output-md "${output_dir}/release-decision.md"
)

if [[ -n "${human_reviewer}" ]]; then
  args+=(--human-reviewer "${human_reviewer}")
fi
if [[ -n "${rollback_note}" ]]; then
  args+=(--rollback-note "${rollback_note}")
fi

cargo run --quiet --features release-tools --bin release_decision_local -- "${args[@]}"
