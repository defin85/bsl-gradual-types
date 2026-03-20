# Change: truthful ingress verdict attribution for completion observability

## Почему
Последний цикл анализа `Observability Incident Bundle` показал, что текущий derived verdict layer искажает реальную картину completion latency:
- `ingress_before_method_entry` срабатывает даже на hot path, когда `transport_to_method_wait_ms=0` и `method_prelude_exec_ms=0`;
- bundle summary из-за этого формулирует завышенные выводы вроде "все traces bottlenecked before method entry";
- уже существующие данные (`client_to_transport_wait_ms`, `transport_to_method_wait_ms`, `method_prelude_exec_ms`) позволяют различать как минимум два ingress-подслоя, но human-readable projection пока их схлопывает.

В результате следующий цикл отладки опирается не на truthful verdicts, а на эвристику, которая сама создаёт шум.

## Что меняется
- Уточнить contract human-readable ingress verdicts для `Completion Timeline`, clipboard и incident bundle.
- Заменить текущую эвристику `transport_to_method_wait_ms >= method_prelude_exec_ms` на conservative positive-dominance rules:
  - без положительной задержки ingress verdict не появляется;
  - server-side ingress до method entry различается с `handler_prelude`;
  - client-side ingress до `transport_received` появляется только при deterministic probe correlation.
- Добавить в `incident.json` и `summary.md` truthful aggregation по ingress verdicts без переоценки hot traces.
- Явно зафиксировать fail-closed поведение для uncorrelated traces: bundle и completion surfaces не выдумывают client-side ingress verdict.

## Не входит в scope
- Новый server-side custom request.
- Новый completion timeline contract version.
- Изменение raw timeline payload или probe schema.
- Новая логика probe-to-trace correlation.
- Чинить сами latency bottlenecks `wait_for_file_version`, `snapshot_with_deps` или ingress starvation; change касается только truthful attribution/reporting.

## Влияние
- Затронутые спеки:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Затронутый код:
  - `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - `vscode-extension/src/test/suite/completionTimelineDrilldown.test.ts`
  - `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts`
  - `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`
  - `scripts/run-intellisense-tests.sh`
  - `vscode-extension/manual-lsp-test.md`
