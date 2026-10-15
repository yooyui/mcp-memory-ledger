#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [[ -e "${REPO_ROOT}/not-a-sqlite-url" ]]; then
  echo "repository hygiene failed: unexpected root SQLite artifact not-a-sqlite-url" >&2
  exit 1
fi

cd "${REPO_ROOT}"

checker_source="${REPO_ROOT}/src/bin/status_sync_check.rs"
status_source="${REPO_ROOT}/src/support/status_sync.rs"
checker_binary="${REPO_ROOT}/target/tools/status-sync-check"

if [[ ! -x "${checker_binary}" || "${checker_source}" -nt "${checker_binary}" || "${status_source}" -nt "${checker_binary}" ]]; then
  mkdir -p "$(dirname "${checker_binary}")"
  rustc --edition=2024 "${checker_source}" -o "${checker_binary}"
fi

exec "${checker_binary}"
