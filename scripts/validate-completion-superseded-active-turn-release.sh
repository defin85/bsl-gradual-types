#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

CHANGE_ID=refactor-completion-superseded-active-turn-release \
  "${ROOT_DIR}/scripts/validate-v2-completion-gates.sh" "$@"
