# Agent Task Artifacts

## Intent -> Plan -> Code

- OpenSpec intent:
  `openspec/changes/<change-id>/proposal.md`
  `openspec/changes/<change-id>/tasks.md`
  `openspec/changes/<change-id>/design.md`
  `openspec/changes/<change-id>/specs/**/spec.md`
- Beads execution graph:
  `.beads/`
  Commands: `bd ready`, `bd show <id>`, `bd close <id>`, `bd vc status`
- Code and docs implementation:
  workspace files under the touched crate/doc/script paths

## Validation And Readiness Assets

- Active CI wiring:
  `.github/workflows/ci.yml`
- OpenSpec governance gate:
  `scripts/check-openspec-change-governance.py`
- Protected assets gate:
  `scripts/check-protected-assets-gate.py`
- Agent-facing docs gate:
  `scripts/run-agent-readiness-checks.sh`
  `scripts/check-agent-readiness.py`
  `scripts/test-agent-readiness.py`
- Smoke/readiness assets:
  `scripts/run-intellisense-tests.sh`
  `scripts/test-intellisense-smoke-gate.py`
  `scripts/test-intellisense-readiness-assets.py`
- Repo-local skills:
  `.agents/skills/verify-workspace/SKILL.md`
  `.agents/skills/bsl-agent-mcp-smoke/SKILL.md`
  `.agents/skills/openspec-delivery-matrix/SKILL.md`
  `.agents/skills/docs-drift-audit/SKILL.md`
- Perf/readiness gates:
  `scripts/run-intellisense-perf.sh`
  `scripts/validate-v2-completion-gates.sh`

## Requirement -> Code -> Test Traceability

Для каждого MUST в active change собери:

1. Requirement
   Файл `openspec/changes/<change-id>/specs/**/spec.md`
2. Artifact
   Конкретные repo files, которые доставляют требование
3. Validation
   Команда, script, unittest или change-specific evidence path

Минимальный формат:

```text
Requirement -> Artifact -> Validation
```

Пример:

```text
Agent verification runbook -> docs/agent/verification.md -> ./scripts/run-agent-readiness-checks.sh
```

## Handoff Checklist

- `openspec validate <change-id> --strict --no-interactive`
- relevant `bd` issues закрыты
- change tasks checklist синхронизирован с delivered state
- validation artifacts не overclaim'ят `complete`, если остаётся MUST backlog
