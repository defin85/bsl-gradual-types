## 1. Spec and Contract
- [ ] 1.1 Добавить/уточнить требования `bsl-intellisense-v2` для root-cause drilldown метрик (`origin+operation+stage+outcome/reason`) с bounded cardinality.
- [ ] 1.2 Зафиксировать совместимость rollout: legacy fixed keys сохраняются параллельно с новыми drilldown ключами.
- [ ] 1.3 Добавить требования по saturation/singleflight observability (waiters/permits/queue depth/effectiveness).
- [ ] 1.4 Обновить требования `mcp-bsl-agent` для parity drilldown-контракта и operation-level сопоставимости с LSP.
- [ ] 1.5 Зафиксировать perf-поведение batch MCP tools: long-running file-scan операции выполняются в background CPU class.

## 2. Runtime Instrumentation
- [ ] 2.1 Расширить `BasicObservability` и `SystemCoordinator` для emission новых drilldown и saturation метрик без удаления legacy ключей.
- [ ] 2.2 Инструментировать shared facade/runtime (`prepare/run_optional_query/singleflight/runtime scheduling`) operation-aware и reason-aware метриками.
- [ ] 2.3 Добавить метрики эффективности singleflight по `query_kind` и сигнал `key_unavailable`.
- [ ] 2.4 Экспортировать saturation gauges/counters runtime budget-ов (waiters, permits, queue depth) в observability snapshot.

## 3. bsl-agent Adoption
- [ ] 3.1 Обновить `bsl-agent` semantic paths так, чтобы новые drilldown метрики автоматически эмитились через shared facade/runtime.
- [ ] 3.2 Перевести batch file-scan semantic tools (`bsl_symbol_search`, `bsl_references`, и эквивалентные долгие path-ы) на background work class.

## 4. Validation
- [ ] 4.1 Обновить контрактные тесты LSP и MCP на наличие drilldown + legacy ключей и корректную parity-интерпретацию.
- [ ] 4.2 Добавить тесты на saturation/singleflight observability (включая смешанную interactive/background нагрузку).
- [ ] 4.3 Добавить perf smoke для `bsl-agent`: под batch-нагрузкой интерактивные запросы не должны деградировать до starvation.
- [ ] 4.4 Запустить `cargo test` для затронутых crates и зафиксировать результаты.

