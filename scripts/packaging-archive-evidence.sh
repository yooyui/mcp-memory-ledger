#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: ./scripts/packaging-archive-evidence.sh <release-candidate> [evidence_root]

Writes local-only packaging archive checksum evidence for an existing release
candidate archive set under:
  <evidence_root>/target/reports/releases/<release-candidate>/packaging/

This script does not build binaries, create installers, upload artifacts, tag a
release, or claim production-ready packaging.
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

release_candidate="$1"
evidence_root="${2:-.}"

case "${release_candidate}" in
  *[!A-Za-z0-9._-]* | "" | .* | *..* )
    echo "candidate name must contain only letters, numbers, dot, underscore, and dash, and must not contain path traversal" >&2
    exit 2
    ;;
esac

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
project_root="$(cd "${script_dir}/.." && pwd -P)"
cd "${project_root}"

cargo run --quiet --features release-tools --bin packaging_archive_evidence -- \
  --release-candidate "${release_candidate}" \
  --evidence-root "${evidence_root}"
