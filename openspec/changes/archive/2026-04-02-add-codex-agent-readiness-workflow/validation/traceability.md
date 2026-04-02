# Requirement -> Artifact -> Validation

## Requirement: curated agent-facing documentation surface для Codex

- Requirement:
  `openspec/changes/add-codex-agent-readiness-workflow/specs/dev-workflow/spec.md`
- Artifact:
  `docs/agent/index.md`
  `docs/agent/architecture-map.md`
  `docs/agent/verification.md`
  `docs/agent/task-artifacts.md`
- Validation:
  `./scripts/run-agent-readiness-checks.sh`
  `python3 -m unittest scripts/test-agent-readiness.py`

## Requirement: layered `AGENTS.md` contract

- Requirement:
  `openspec/changes/add-codex-agent-readiness-workflow/specs/dev-workflow/spec.md`
- Artifact:
  `AGENTS.md`
  `backend/AGENTS.md`
  `bsl-agent/AGENTS.md`
  `vscode-extension/AGENTS.md`
  `.github/copilot-instructions.md`
- Validation:
  `./scripts/run-agent-readiness-checks.sh`
  `python3 -m unittest scripts/test-agent-readiness.py scripts/test-ci-openspec-governance-workflow.py`

## Requirement: executable agent verification runbook with live runtime surfaces

- Requirement:
  `openspec/changes/add-codex-agent-readiness-workflow/specs/dev-workflow/spec.md`
- Artifact:
  `docs/agent/verification.md`
  `README.md`
  `docs/BUILD_GUIDE.md`
  `docs/guides/development-workflow.md`
- Validation:
  `python3 -m unittest scripts/test-intellisense-smoke-gate.py scripts/test-intellisense-readiness-assets.py`
  `./scripts/run-intellisense-tests.sh smoke`

## Requirement: portable Codex setup and repo-local skills

- Requirement:
  `openspec/changes/add-codex-agent-readiness-workflow/specs/dev-workflow/spec.md`
- Artifact:
  `docs/agent/codex-setup.md`
  `.mcp.json`
  `.agents/skills/verify-workspace/SKILL.md`
  `.agents/skills/bsl-agent-mcp-smoke/SKILL.md`
  `.agents/skills/openspec-delivery-matrix/SKILL.md`
  `.agents/skills/docs-drift-audit/SKILL.md`
  `bsl-agent/README.md`
- Validation:
  `./scripts/run-agent-readiness-checks.sh`
  `python3 -m unittest scripts/test-agent-readiness.py`

## Requirement: machine-checkable freshness checks for agent-facing docs and commands

- Requirement:
  `openspec/changes/add-codex-agent-readiness-workflow/specs/dev-workflow/spec.md`
- Artifact:
  `scripts/check-doc-paths.py`
  `scripts/doc-path-check-targets.txt`
  `scripts/check-agent-readiness.py`
  `scripts/run-agent-readiness-checks.sh`
  `scripts/test-agent-readiness.py`
  `.github/workflows/ci.yml`
  `scripts/test-ci-openspec-governance-workflow.py`
- Validation:
  `./scripts/run-agent-readiness-checks.sh`
  `python3 -m unittest scripts/test-agent-readiness.py scripts/test-ci-openspec-governance-workflow.py`
  `openspec validate add-codex-agent-readiness-workflow --strict --no-interactive`
