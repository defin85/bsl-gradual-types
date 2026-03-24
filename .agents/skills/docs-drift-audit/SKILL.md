---
name: docs-drift-audit
description: Audit whether onboarding docs, `docs/agent/*`, and layered `AGENTS.md` files still match the actual workspace. Use when documentation, agent instructions, or readiness scripts may have drifted.
---

# Docs Drift Audit

Используй этот skill, когда нужно проверить, что onboarding docs, `docs/agent/*` и layered `AGENTS.md` не разошлись с реальным workspace.

## Steps

1. Запусти `./scripts/run-agent-readiness-checks.sh`.
2. Проверь primary onboarding docs:
   `README.md`
   `docs/README.md`
   `docs/BUILD_GUIDE.md`
   `docs/guides/development-workflow.md`
3. Убедись, что root `AGENTS.md` остаётся каноническим dispatcher, а локальные `AGENTS.md` не дублируют global playbook.
4. Если drift найден, правь docs до того, как менять более широкий README/CI narrative.

## Expected Outcome

- stale binary/package names отловлены fail-closed
- canonical docs и local instructions существуют и ссылаются друг на друга корректно
