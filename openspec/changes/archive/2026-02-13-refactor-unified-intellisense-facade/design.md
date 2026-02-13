# Design: refactor-unified-intellisense-facade

## Context
Текущая реализация v2 семантики уже использует единые core алгоритмы, но orchestration-path расползся:
- LSP имеет собственный stateful runtime и развитую stage-level observability.
- `bsl-agent` и web handlers содержат повторяющиеся ad-hoc цепочки инициализации `AnalysisHostV2` и запросов.

Следствие: исправления latency/cancellation/lazy-query поведения переносятся вручную и легко расходятся между интерфейсами.

## Goals
- Ввести один канонический orchestration путь для v2 semantic операций во всех интерфейсах.
- Исключить adapter-level дубли orchestration (LSP/web/MCP должны быть thin adapters).
- Централизовать performance политику (lazy parse, cancellation, bounded blocking).
- Сохранить текущие публичные контракты LSP/HTTP/MCP и совместимость observability JSON форматов.
- Сделать полный rollout в рамках одного change (без MVP-среза).

## Non-Goals
- Переписывать `bsl-analysis-v2` и его доменные query-алгоритмы.
- Менять пользовательские протоколы LSP/HTTP/MCP без необходимости.
- Вводить fallback на старые adapter-local orchestration пути после миграции.

## Architecture Drivers
- Maintainability: одно место для исправлений и развития orchestration.
- Performance: единая оптимизация hot-path и предсказуемая tail latency.
- Correctness: единая cancellation/queueing семантика.
- Observability: единые stage метрики и outcome-классификация между интерфейсами.
- Drift prevention: обязательные parity/perf regression тесты.

## Options Considered

### Option A: Оставить orchestration в адаптерах, добавить общий helper
- Плюсы: минимальные точечные правки.
- Минусы: сохраняется дублирование цепочек и drift риск; perf фикс все равно переносить вручную.

### Option B: Добавить новый wrapper поверх существующих adapter-path
- Плюсы: проще миграция по шагам.
- Минусы: два активных пути в коде, постоянный риск расхождения поведения и метрик.

### Option C (Chosen): Полный рефакторинг на единый shared фасад в `bsl-runtime`
- Плюсы: один orchestration контракт, единый perf/cancel/metrics слой, устранение дрейфа.
- Минусы: больше объем миграции в одном change; выше требования к regression coverage.

## Chosen Architecture

### 1) Canonical Facade in `bsl-runtime`
Создаётся общий IntelliSense facade, который владеет orchestration для semantic операций:
- file-version synchronization (где применимо),
- snapshot acquisition,
- query sequencing (IR/syntax/semantic/parse_result),
- cancellation mapping,
- stage observability.

Адаптеры вызывают facade и занимаются только transport mapping.

### 2) Unified Runtime Contract
В shared слое вводится единый runtime-контракт с двумя режимами исполнения:
- Stateful runtime: для LSP и `bsl-agent` session-based запросов.
- Ephemeral runtime: для web one-shot запросов.

Оба режима используют одну и ту же facade-логику и одинаковую stage семантику.

### 3) Centralized Performance Policy
Перф-политики задаются и применяются только в shared facade/runtime:
- lazy `parse_result` (только когда требуется операции и есть IR),
- единая cancellation policy (IR/syntax/semantic),
- bounded blocking/concurrency guard для CPU-heavy веток,
- одинаковые slow-path метрики и outcome labels.

### 4) Unified Observability
Сохраняется совместимый JSON snapshot метрик, но источник вычисления стадий становится единым.
Требование: stage names/counters/histograms и outcome-коды для одинаковых операций согласованы между LSP/web/MCP.

## Migration Plan (Full Rollout)
1. Зафиксировать API и контракты shared facade/runtime.
2. Перенести stateful runtime orchestration в `bsl-runtime`.
3. Подключить LSP к shared facade.
4. Подключить web handlers к shared facade.
5. Подключить `bsl-agent` semantic tools к shared facade.
6. Удалить adapter-local orchestration дубли.
7. Включить parity/perf regression gates в тестовом контуре.

Важно: после шага 6 не должно остаться production semantic path, выполняющих ad-hoc orchestration в адаптерах.

## Validation Strategy
- Cross-interface parity tests на одинаковых fixture/snapshot.
- Cold/warm perf regression tests для крупных модулей.
- Cancellation regression tests (consistent outcomes, no hangs).
- Observability parity checks между LSP command и MCP metrics tool.

## Risks and Mitigations
- Риск: рост сложности миграции в одном change.
  - Mitigation: строгая декомпозиция задач + parity tests на каждом этапе.
- Риск: временная деградация latency при переносе runtime.
  - Mitigation: perf regression thresholds и сравнение до/после на фиксированных fixture.
- Риск: расхождение semantics между stateful и ephemeral режимами.
  - Mitigation: единый facade code-path и обязательные parity тесты между режимами.
- Риск: несовместимость метрик с текущими dashboard.
  - Mitigation: сохранить существующие metric keys; расширять только неразрушающими полями.
