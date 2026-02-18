## 1. Spec and Contract
- [x] 1.1 Добавить/уточнить требования `bsl-intellisense-v2` для root-cause drilldown метрик (`origin+operation+stage+outcome/reason`) с bounded cardinality.
- [x] 1.2 Зафиксировать единый канонический observability контракт и deterministic mapping legacy fixed keys как compatibility-проекцию (без отдельной семантики).
- [x] 1.2.1 Зафиксировать формальную event schema: metric families, обязательные/контекстные dimensions и допустимые комбинации.
- [x] 1.3 Добавить требования по saturation/singleflight observability (waiters/permits/queue depth/effectiveness).
- [x] 1.4 Обновить требования `mcp-bsl-agent` для parity drilldown-контракта и operation-level сопоставимости с LSP.
- [x] 1.5 Зафиксировать perf-поведение batch MCP tools: long-running file-scan операции выполняются в background CPU class.

## 2. Runtime Instrumentation
- [x] 2.1 Расширить `BasicObservability` и `SystemCoordinator` для emission канонических drilldown и saturation метрик.
- [x] 2.2 Реализовать dual-write через projection-слой: legacy fixed keys формируются из канонического контракта, а не отдельными независимыми ветками emission.
- [x] 2.2.1 Реализовать единый mapping-реестр каноника -> legacy keys; отсутствие обязательного mapping считать контрактной ошибкой.
- [x] 2.2.2 Зафиксировать backend-first ownership emission: адаптеры не публикуют drilldown/legacy напрямую, а передают только канонические события в shared projection pipeline.
- [x] 2.3 Инструментировать shared facade/runtime (`prepare/run_optional_query/singleflight/runtime scheduling`) operation-aware и reason-aware метриками.
- [x] 2.4 Добавить метрики эффективности singleflight по `query_kind` и сигнал `key_unavailable`.
- [x] 2.5 Экспортировать saturation gauges/counters runtime budget-ов (waiters, permits, queue depth) в observability snapshot.
- [x] 2.6 Добавить fail-fast проверку недопустимых combinations dimensions в канонических событиях.

## 3. bsl-agent Adoption
- [x] 3.1 Обновить `bsl-agent` semantic paths так, чтобы новые drilldown метрики автоматически эмитились через shared facade/runtime.
- [x] 3.2 Перевести batch file-scan semantic tools (`bsl_symbol_search`, `bsl_references`, и эквивалентные долгие path-ы) на background work class.

## 4. Validation
- [x] 4.1 Обновить контрактные тесты LSP и MCP на наличие drilldown + legacy ключей и корректную parity-интерпретацию.
- [x] 4.2 Добавить инвариантные tests каноника -> legacy projection (значения fixed keys соответствуют агрегированным drilldown series).
- [x] 4.2.1 Добавить schema-validation tests: недопустимые combinations dimensions не публикуются как метрики и фиксируются контрактным сигналом.
- [x] 4.2.2 Добавить tests на projection ownership/invariance: один канонический event детерминированно материализует обе проекции (drilldown + legacy) без adapter-local bypass.
- [x] 4.3 Добавить тесты на saturation/singleflight observability (включая смешанную interactive/background нагрузку).
- [x] 4.4 Добавить perf smoke для `bsl-agent`: под batch-нагрузкой интерактивные запросы не должны деградировать до starvation.
- [x] 4.5 Запустить `cargo test` для затронутых crates и зафиксировать результаты.
