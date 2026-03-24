---
name: verify-workspace
description: Verify that the current workspace, agent-facing docs, and the default smoke path remain consistent. Use for quick post-change validation before handoff or after documentation/runtime updates.
---

# Verify Workspace

Используй этот skill, когда нужно быстро проверить, что текущий workspace, agent-facing docs и default smoke path остаются согласованными.

## Steps

1. Запусти `./scripts/run-agent-readiness-checks.sh`.
2. Запусти `./scripts/run-intellisense-tests.sh smoke`.
3. Если change затронул Rust runtime, добей `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`.

## Expected Outcome

- canonical docs и instruction layering валидны
- default smoke path проходит
- дополнительные Rust gates прогоняются только если change реально меняет runtime behavior
