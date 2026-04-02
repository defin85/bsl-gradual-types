# Change: Сделать query_bundle truthful root-cause surface для Completion Timeline

## Почему

Incident bundle `2026-04-02T15:44:53Z` показывает, что delivered pre-dispatch split уже отделяет transport ingress от server dispatch backlog, но следующий dominant seam остался внутри completion handler:

- authoritative trace показывает `adapter_to_dispatch_wait_ms=2-3ms` и `turn_wait=0`, то есть ingress path больше не выглядит bottleneck;
- при этом `query_bundle` доминирует на seconds-scale tail (`3593ms`), а superseded request замечает cancel только в конце длинного handler window;
- human-readable projection всё ещё может выдать `adapter_before_dispatch_dominant`, потому что сравнивает ingress wait только с `transport_to_method_wait_ms` и `method_prelude_exec_ms`, игнорируя dominant query-body stage;
- cancelled request, прерванный внутри `query_bundle`, теряет spent time в `unattributed_overhead`, потому что aggregate stage публикуется только на happy path.

Следовательно, следующий change нужен не для нового transport rewrite, а для truthful root-cause attribution и bounded cancellation accounting внутри query-body path.

## Что меняется

- authoritative completion timeline поднимается `19 -> 20`, а contiguous contract baseline поднимается `contracts/lsp-completion-timeline/v16 -> v17`;
- query-body path перестаёт быть opaque aggregate: timeline получает bounded `query_bundle` stage breakdown как минимум для `pool_wait`, `deps_and_file_snapshot`, `ir_query`, optional `ir_retry` и `other`;
- если request вошёл в `query_bundle`, trace MUST публиковать соответствующий `query_bundle*` stage и на `cancelled/failed` path, а не терять spent time в `unattributed_overhead`;
- blocking runtime path для interactive completion получает request-local observed split между pool queue wait и blocking exec, чтобы incident analysis мог отделить saturation от actual compute;
- derived extension verdicts и incident summary перестают обвинять `adapter_before_dispatch_dominant`, когда authoritative `dominant_stage`/`stages` доказывают query-body dominance;
- webview/clipboard/incident projections для `v20` используют единый truthful verdict source, а на `v19` деградируют явно без invented `query_bundle` breakdown.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `contracts/lsp-completion-timeline/v17/*`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle*.ts`
  - `vscode-extension/src/test/suite/*completion*`
