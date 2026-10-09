#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TIER="${1:-fast}"

cd "${REPO_ROOT}"

case "${TIER}" in
  fast)
    cargo test --lib \
      --test decision_flow \
      --test domain_invariants \
      --test domain_snapshot \
      --test evidence_query_dto
    ;;
  core)
    cargo test
    ;;
  full)
    cargo test --all-features
    ;;
  *)
    echo "usage: $0 [fast|core|full]" >&2
    exit 2
    ;;
esac
