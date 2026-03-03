## ADDED Requirements
### Requirement: Observability/perf pipeline MUST иметь единую архитектурную границу `ingest -> validate -> project -> export`
Система MUST выполнять обработку observability/perf событий через centralized pipeline core, который:
- валидирует canonical schema;
- применяет registry-driven materialization;
- публикует drilldown/legacy представления детерминированно.

Adapter/runtime код MUST NOT публиковать metric keys напрямую в экспортный слой в обход pipeline boundary.

#### Scenario: Adapter-local bypass блокируется как архитектурное нарушение
- **GIVEN** адаптер пытается опубликовать метрику напрямую, минуя pipeline core
- **WHEN** запускаются architecture/contract checks
- **THEN** изменение отклоняется как policy violation
- **AND** метрика не считается валидной частью observability surface

### Requirement: Projection mapping MUST быть registry-compiled и полным
Наборы допустимых `operation/stage/reason/outcome` MUST задаваться typed registry.
Canonical normalization, drilldown/legacy projection и allow-lists MUST генерироваться из этого же registry.

Добавление taxonomy значения без полного mapping MUST детектироваться до merge.

#### Scenario: Новый taxonomy label без mapping не проходит CI
- **GIVEN** добавлен новый `reason` в taxonomy
- **WHEN** не обновлены generated projection tables и completeness checks
- **THEN** contract/parity validation падает
- **AND** change не может быть принят до восстановления полной materialization

### Requirement: Perf evidence provenance MUST быть fail-closed
Perf artifacts MUST содержать обязательный provenance envelope:
- `change_id`
- `generated_at`
- `profile`
- `schema_version`
- `contract_version`

Validator MUST отклонять artifact при mismatch или отсутствии любого обязательного provenance поля.

#### Scenario: Foreign `change_id` делает perf artifact невалидным
- **GIVEN** perf прогон выполняется для change `X`
- **WHEN** report содержит `change_id = Y` или не содержит `change_id`
- **THEN** validator возвращает invalid evidence
- **AND** quality gate не использует artifact для acceptance решения

### Requirement: Unified instrumentation API MUST покрывать все interactive type-index serve outcomes
Interactive операции (`completion`, `hover`, `signatureHelp`, `definition`) MUST использовать единый instrumentation API для emission bounded `type_index` serve outcomes.

Reason labels MUST оставаться low-cardinality и нормализоваться в `other` для неизвестных значений.

#### Scenario: Все interactive операции публикуют совместимый bounded reason set
- **GIVEN** выполняются completion/hover/signatureHelp/definition запросы
- **WHEN** каждая операция проходит serve-only path
- **THEN** outcome reasons публикуются через один контрактный emission path
- **AND** неизвестные значения не увеличивают cardinality и сворачиваются в `other`

### Requirement: Rewrite rollout MUST поддерживать dual-write, canary parity и rollback
Внедрение rewrite pipeline MUST идти поэтапно:
- dual-write (legacy + rewrite);
- canary parity checks;
- controlled cutover;
- rollback readiness до завершения cutover.

Cutover MUST NOT выполняться без подтвержденной parity-совместимости и валидного provenance evidence.

#### Scenario: Canary parity drift инициирует rollback вместо cutover
- **GIVEN** rewrite pipeline включён в canary phase
- **WHEN** parity checks фиксируют drift выше контрактного порога
- **THEN** система возвращается на legacy primary path
- **AND** cutover откладывается до устранения drift
