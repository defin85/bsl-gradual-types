# Traceability: add-incident-bundle-request-correlation

## Requirement -> Code -> Test

### Requirement: Observability incident bundle даёт request-centric handoff summary поверх raw evidence
- Request-centric derived report contract, capture scope и bounded request list:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::happy path bundle should contain request-centric incident report and all raw attachments`
- Partial export без подмены authoritative request list local probes-данными:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::unsupported completion timeline should produce partial bundle without fabricated raw trace`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::completion timeline error should mark authoritative server trace as unavailable`
- Request-centric file export path на default command wiring:
  - Code: `vscode-extension/src/commands/observability.ts`
  - Test: `vscode-extension/src/test/suite/observabilityCommands.test.ts::exportObservabilityIncidentBundle should write bundle files via command callback`
  - Test: `vscode-extension/src/test/suite/observabilityCommands.test.ts::exportObservabilityIncidentBundle should honor provided capture overrides without refetching timeline`

### Requirement: Probe-to-trace correlation остаётся deterministic и fail-closed
- Deterministic optional correlation и bounded client-side supplement:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::happy path bundle should contain request-centric incident report and all raw attachments`
- Ambiguous correlation остаётся server-centric и не создаёт guessed pair:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::ambiguous correlation should keep request summary server-centric and record a gap`

## Operational truth
- Focused extension-host smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
- Manual/runbook expectations:
  - Doc: `scripts/README.md`
  - Doc: `vscode-extension/manual-lsp-test.md`
  - Doc: `vscode-extension/src/test/README.md`
