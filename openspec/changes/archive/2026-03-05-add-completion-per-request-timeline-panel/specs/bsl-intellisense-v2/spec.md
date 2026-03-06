## ADDED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `1`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов или агрегированных observability-метрик.

Контракт `v1` MUST включать:
- `version` (числовой номер контракта);
- `traces` (массив completion trace записей).

Каждый trace MUST включать:
- `trace_id`, `request_id`, `uri`, `trigger_mode`;
- `outcome`, `started_at_ms`, `total_duration_ms`;
- `dominant_stage`;
- `stages`.

Каждый stage entry MUST включать:
- `name`;
- `status` (`completed|cancelled|failed|skipped`);
- `started_offset_ms`;
- `duration_ms`.

#### Scenario: Клиент получает детерминированный timeline для завершённого completion
- **GIVEN** completion-запрос успешно обработан
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** response содержит trace со стадиями в порядке исполнения
- **AND** `total_duration_ms` не меньше максимального stage end offset
- **AND** `dominant_stage` совпадает с этапом максимальной длительности в trace

#### Scenario: Клиент получает корректный timeline для cancelled/superseded completion
- **GIVEN** completion-запрос отменён или superseded до полного завершения pipeline
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** response содержит partial trace с terminal outcome cancelled/superseded
- **AND** trace не маркируется как успешный completed

#### Scenario: VS Code клиент получает timeline через `workspace/executeCommand`
- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v1` с server-generated traces
- **AND** клиент не строит timeline из текстовых логов или p95/p99 агрегатов

### Requirement: Timeline stage taxonomy bounded и совместима с completion observability (MUST)
Stage names в per-request timeline MUST использовать bounded taxonomy, согласованную с completion stage observability.

Timeline MUST NOT включать high-cardinality stage labels (динамические URI/пути/произвольные тексты) как часть имени stage.

#### Scenario: Stage labels остаются low-cardinality при разных файлах
- **GIVEN** completion выполняется для разных документов и рабочих областей
- **WHEN** формируются timeline traces
- **THEN** stage names остаются в пределах фиксированного словаря
- **AND** payload не содержит stage-name взрыва по cardinality

### Requirement: Timeline retention bounded и deterministic (MUST)
Per-request completion timeline хранилище MUST быть bounded по количеству записей (count-based ring buffer), с deterministic eviction oldest-first.

Retention default MUST быть задан как `max_entries=200`.

#### Scenario: Переполнение retention удаляет самые старые traces
- **GIVEN** в timeline buffer уже хранится `max_entries` traces
- **WHEN** добавляется новый completion trace
- **THEN** удаляется самый старый trace
- **AND** новые traces остаются доступными через `bsl.getCompletionTimeline`

### Requirement: Timeline instrumentation не меняет completion semantics и SLO-инварианты (MUST)
Запись per-request timeline MUST быть side-effect-safe:
- не должна менять user-facing completion response semantics;
- не должна добавлять блокирующий sync compute в request path;
- при внутренней ошибке timeline capture completion MUST продолжать работу в fail-open режиме для пользователя.

#### Scenario: Ошибка timeline capture не ломает completion ответ
- **GIVEN** во время записи timeline произошла внутренняя ошибка instrumentation
- **WHEN** completion pipeline формирует ответ пользователю
- **THEN** completion response возвращается по обычному контракту
- **AND** ошибка instrumentation не приводит к падению LSP completion handler
