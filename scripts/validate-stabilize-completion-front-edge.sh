#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

CHANGE_ID=stabilize-completion-front-edge \
REAL_MODULE_PROFILES="churn" \
  "${ROOT_DIR}/scripts/validate-v2-completion-gates.sh" "$@"
