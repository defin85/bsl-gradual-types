## ADDED Requirements

### Requirement: Единый orchestration facade для всех v2 интерфейсов (MUST)
Система MUST выполнять v2 semantic orchestration через единый shared facade в `bsl-runtime` для всех интерфейсов (`LSP`, `web`, `MCP`).

Adapter-layer код MUST оставаться transport-oriented (LSP/HTTP/MCP mapping) и MUST NOT содержать production orchestration цепочки напрямую через ad-hoc `AnalysisHostV2` setup, ручное sequencing `wait/snapshot/query` и adapter-local ветки semantic pipeline.

#### Scenario: LSP, web и MCP используют один orchestration контракт
- **GIVEN** одинаковые входные данные (текст документа, deps snapshot, настройки, позиция)
- **WHEN** клиент запрашивает semantic операцию через LSP, web и MCP
- **THEN** операция выполняется через общий facade path с согласованной стадийной последовательностью
- **AND** различия между интерфейсами ограничены транспортным форматом ответа

### Requirement: Производительные политики v2 централизованы и наследуются всеми адаптерами (MUST)
Система MUST централизовать performance-sensitive политику в shared facade/runtime:
- lazy `parse_result`,
- cancellation policy для IR/syntax/semantic queries,
- bounded blocking/concurrency control,
- queue-wait и stage-latency observability.

Adapter-local reimplementation этих политик MUST NOT использоваться в production semantic path.

#### Scenario: Исправление lazy `parse_result` применяется сразу во всех интерфейсах
- **GIVEN** policy требует не выполнять `parse_result`, если IR недоступен
- **WHEN** первый semantic запрос выполняется через LSP, web и MCP
- **THEN** ни один интерфейс не запускает `parse_result` при отсутствующем IR
- **AND** outcome/latency метрики отражают согласованное поведение во всех интерфейсах

### Requirement: Drift-prevention через cross-interface parity и perf regression (MUST)
Система MUST иметь автоматические проверки, которые предотвращают расхождение поведения между `LSP`, `web` и `MCP`:
- semantic parity tests на общих fixture/snapshot,
- cold/warm perf regression tests на крупных модулях,
- observability parity checks для стадий v2 pipeline.

#### Scenario: Drift в одном интерфейсе блокируется тестами
- **GIVEN** изменение в semantic orchestration влияет только на один адаптер
- **WHEN** запускаются parity/perf regression проверки
- **THEN** проверка завершается ошибкой
- **AND** изменение не считается принятым до восстановления parity
