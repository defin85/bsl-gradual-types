#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

if command -v rg >/dev/null; then
  if rg -n "^[[:space:]]*bsl-backend[[:space:]]*=" bsl-agent/Cargo.toml >/dev/null; then
    fail "bsl-agent/Cargo.toml must not depend on bsl-backend"
  fi
else
  if grep -nE "^[[:space:]]*bsl-backend[[:space:]]*=" bsl-agent/Cargo.toml >/dev/null; then
    fail "bsl-agent/Cargo.toml must not depend on bsl-backend"
  fi
fi

if command -v cargo >/dev/null; then
  if command -v rg >/dev/null; then
    if cargo tree -p bsl-agent -e all | rg -q "bsl-backend"; then
      fail "bsl-agent must not depend on bsl-backend (directly or transitively)"
    fi
  else
    if cargo tree -p bsl-agent -e all | grep -q "bsl-backend"; then
      fail "bsl-agent must not depend on bsl-backend (directly or transitively)"
    fi
  fi
fi

echo "OK: bsl-agent does not depend on bsl-backend"
