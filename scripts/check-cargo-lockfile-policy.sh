#!/usr/bin/env bash
set -euo pipefail

if [[ "${1-}" != "" ]]; then
  echo "Usage: $0" >&2
  exit 2
fi

if git check-ignore -v Cargo.lock >/dev/null 2>&1; then
  echo "ERROR: Cargo.lock must not be ignored." >&2
  git check-ignore -v Cargo.lock >&2 || true
  exit 1
fi

if ! git ls-files --error-unmatch Cargo.lock >/dev/null 2>&1; then
  echo "ERROR: Cargo.lock must be tracked in git." >&2
  echo "Fix: git add Cargo.lock" >&2
  exit 1
fi

