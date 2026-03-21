# Change: add-completion-bottleneck-drilldown

## Почему
Текущий completion timeline и incident bundle уже позволяют увидеть общие симптомы вроде `transport_to_handler_wait`, `prepare_timeout` и `exact_deadline`, но для реального root-cause анализа всё ещё приходится читать raw JSON вручную.

Часть bounded drilldown уже присутствует только в structured payload (`prepare.progress`, `exact_wait`, `dispatcher_resolution_latency_ms`), но не выводится в человекочитаемых проекциях. При этом authoritative per-request contract пока не фиксирует достаточно явный bottleneck drilldown для `prepare_stateful` и `exact_wait`, чтобы локализовать проблему без ad-hoc логов и ручной корреляции.

## Что меняется
- Расширить authoritative контракт `bsl.getCompletionTimeline` до `v5` bounded bottleneck drilldown-полями для:
  - ingress/disptacher attribution;
  - `prepare_stateful` subphase/runtime split;
  - `exact_wait` waiter/task state.
- Зафиксировать low-cardinality vocabulary и fail-open инварианты для новых полей.
- Добавить человекочитаемую проекцию этих полей в Completion Timeline UI, clipboard export и AI-friendly incident handoff summary, чтобы типовые bottleneck'и читались без raw JSON.
- Не вводить отдельный unbounded лог-файл и не добавлять новый server API: authoritative surface остаётся `bsl.getCompletionTimeline`.

## Влияние
- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `vscode-extension/src/providers/completionTimeline*`
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
