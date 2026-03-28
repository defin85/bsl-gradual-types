#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

CHANGE_ID=isolate-completion-pre-dispatch-ingress \
REAL_MODULE_PROFILES="outline" \
  "${ROOT_DIR}/scripts/validate-v2-completion-gates.sh" "$@"
