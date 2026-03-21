# Traceability: add-observability-incident-bundle-export

## Requirement -> Code -> Test

### Requirement 1: VS Code extension экспортирует AI-friendly observability incident bundle
- Команда и default export path:
  - Code: `vscode-extension/src/commands/observability.ts`
  - Test: `vscode-extension/src/test/suite/observabilityCommands.test.ts`
- User-facing entry points из Observability и Completion Timeline:
  - Code: `vscode-extension/src/providers/observabilityProvider.ts`
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Test: `vscode-extension/src/test/suite/observabilityProvider.test.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts`
- Bundle builder, `summary.md`, `incident.json`, `raw/*` attachments:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`

### Requirement 1 / Scenario: Raw evidence остаётся отдельным от derived summary
- Raw attachments пишутся отдельно и не зависят от Output dump:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Code: `vscode-extension/src/commands/observability.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`
  - Test: `vscode-extension/src/test/suite/observabilityCommands.test.ts`

### Requirement 2: Incident bundle деградирует предсказуемо при частичной недоступности данных
- Completion timeline fail-closed и capability cache reset после restart:
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Code: `vscode-extension/src/commands/debug.ts`
  - Code: `vscode-extension/src/lsp/client/lifecycle.ts`
  - Test: `vscode-extension/src/test/suite/customRequests.test.ts`
- Truthful `unsupported` vs `unavailable` semantics для observability metrics:
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Test: `vscode-extension/src/test/suite/customRequests.test.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`
- Export из Completion Timeline использует текущий snapshot панели:
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Code: `vscode-extension/src/commands/observability.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts`
  - Test: `vscode-extension/src/test/suite/observabilityCommands.test.ts`

## Operational truth
- Focused smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
  - Test: `scripts/test-intellisense-readiness-assets.py`
- Manual/runbook examples:
  - Doc: `vscode-extension/manual-lsp-test.md`
  - Doc: `vscode-extension/src/test/README.md`
  - Doc: `scripts/README.md`
