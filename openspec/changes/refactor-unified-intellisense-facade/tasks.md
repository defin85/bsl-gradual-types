## 1. Shared Contract
- [x] 1.1 Зафиксировать канонический API общего IntelliSense v2 фасада в `bsl-runtime` для semantic операций всех интерфейсов.
- [x] 1.2 Зафиксировать единый execution-context контракт: версия файла, deps snapshot, flow-sensitive флаг, cancellation, settings detail level.
- [x] 1.3 Зафиксировать единый observability контракт (stage names/counters/histograms/outcomes), общий для LSP/web/MCP.

## 2. Shared Runtime Layer
- [x] 2.1 Вынести и обобщить stateful runtime orchestration (writer-thread, wait/snapshot/deps synchronization) в `bsl-runtime`.
- [x] 2.2 Добавить ephemeral execution path для one-shot web операций через тот же facade-контракт.
- [x] 2.3 Сконцентрировать runtime/perf knobs в shared слое, убрать adapter-local ветвления orchestration.

## 3. Full Migration (No MVP)
- [x] 3.1 Мигрировать LSP semantic paths на общий фасад (completion/hover/signatureHelp/definition/diagnostics и связанные команды).
- [x] 3.2 Мигрировать web semantic handlers на общий фасад и удалить дубли `AnalysisHostV2` setup из handlers.
- [x] 3.3 Мигрировать `bsl-agent` semantic tools на общий фасад (`bsl_diagnostics`, `bsl_type_at_position`, `bsl_members`, `bsl_definition`, `bsl_symbol_search`, `bsl_references`).
- [x] 3.4 Удалить adapter-local orchestration после миграции, оставить в адаптерах только transport mapping и lifecycle glue.

## 4. Unified Performance Policy
- [x] 4.1 Реализовать централизованную lazy policy для `parse_result` (выполнять только когда нужно операции и доступен IR).
- [x] 4.2 Реализовать централизованную cancellation policy для IR/syntax/semantic запросов и единые outcome-коды.
- [x] 4.3 Реализовать bounded blocking/concurrency control для CPU-heavy веток, чтобы исключить starvation очередей.
- [x] 4.4 Гарантировать, что актуальные perf-оптимизации применяются один раз в shared фасаде и автоматически наследуются LSP/web/MCP.

## 5. Verification
- [x] 5.1 Добавить cross-interface parity тесты (единые fixtures/snapshots, сравнение semantic результата между LSP/web/MCP).
- [x] 5.2 Добавить cold/warm perf regression suite для больших модулей (включая `examples/conf_big/.../Module.bsl`) с зафиксированными порогами.
- [x] 5.3 Добавить проверку observability parity: совместимость stage-метрик между LSP `bsl.getObservabilityMetrics` и MCP `workspace_get_observability_metrics`.
- [x] 5.4 Прогнать `cargo fmt`, профильные `cargo check`/`cargo test` по затронутым крейтам.
- [x] 5.5 Прогнать `openspec validate refactor-unified-intellisense-facade --strict --no-interactive`.
