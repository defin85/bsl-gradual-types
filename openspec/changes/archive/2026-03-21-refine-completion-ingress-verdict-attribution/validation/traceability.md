# Traceability: refine-completion-ingress-verdict-attribution

## Requirement -> Code -> Test

### Requirement: Human-readable completion ingress verdicts остаются truthful и positive-only
- Hot path без положительного ingress wait не получает ложный ingress verdict:
  - Code: `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineDrilldown.test.ts::hot path with zero waits should not produce ingress or prelude verdicts`
- Server-side ingress до `method_entered` отделяется от handler prelude:
  - Code: `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - Code: `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineDrilldown.test.ts::positive transport-to-method wait should produce server-before-method verdict`
  - Test: `vscode-extension/src/test/suite/completionTimelineDrilldown.test.ts::positive handler prelude should produce handler-prelude verdict`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::copyVisibleTimelineToClipboard should include bounded v6 fact lines and verdicts`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::copyVisible message should write current visible traces to clipboard`
- Client-side ingress supplement появляется только при deterministic correlation и положительной доминирующей задержке:
  - Code: `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::correlated request should expose client-before-transport verdict when client wait dominates`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::v5 completion timeline should keep request-centric report without v6-only findings`

### Requirement: Incident bundle findings агрегируют ingress verdicts truthfully
- Request-centric summary считает server-side и client-side ingress отдельно и не переоценивает hot traces:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::happy path bundle should contain request-centric incident report and all raw attachments`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::correlated request should expose client-before-transport verdict when client wait dominates`
- Uncorrelated requests остаются fail-closed и не получают guessed client-side ingress finding:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::unsupported completion timeline should produce partial bundle without fabricated raw trace`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::completion timeline error should mark authoritative server trace as unavailable`

## Operational truth
- Focused extension-host smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
- Manual/runbook expectations:
  - Doc: `scripts/README.md`
  - Doc: `vscode-extension/manual-lsp-test.md`
  - Doc: `vscode-extension/src/test/README.md`
