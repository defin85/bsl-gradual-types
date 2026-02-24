## ADDED Requirements

### Requirement: Superseded diagnostics задачи отменяются до завершения heavy path (MUST)
При появлении более новой ревизии документа система MUST помечать соответствующие in-flight diagnostics задачи как superseded и инициировать их отмену до завершения тяжелых стадий.

`DebouncedFull` и `IdleHeavy` профили MUST поддерживать supersession cancellation.

#### Scenario: Burst didChange отменяет устаревшие heavy diagnostics
- **GIVEN** для файла запущена heavy diagnostics задача на ревизии `R`
- **AND** приходит более новая ревизия `R+1`
- **WHEN** scheduler пересчитывает очередность задач
- **THEN** задача `R` переводится в superseded cancellation
- **AND** heavy стадии для `R` не продолжаются до полного завершения, если достигнут cancel checkpoint

### Requirement: Cancellation checkpoints обязательны между heavy diagnostics стадиями (MUST)
Система MUST иметь кооперативные cancellation checkpoints как минимум:
- перед запуском parse/syntax heavy стадии;
- между syntax и semantic heavy стадиями;
- перед publish diagnostics.

Задача, получившая superseded cancel, MUST завершаться без publish.

#### Scenario: Superseded задача не публикует diagnostics
- **GIVEN** heavy diagnostics для ревизии `V` уже вычислена частично
- **AND** до publish приходит ревизия `V+1`
- **WHEN** выполняется финальный checkpoint перед publish
- **THEN** результат `V` не публикуется
- **AND** publish выполняется только для актуальной ревизии

### Requirement: Observability различает superseded cancellation и прочие причины cancel (MUST)
Система MUST публиковать low-cardinality signals для diagnostics cancellation с фиксированными причинами:
- `superseded_generation`
- `superseded_version`
- `client_cancel`
- `other_cancel`

#### Scenario: Root-cause cancel виден в метриках
- **GIVEN** под churn устаревшие diagnostics регулярно отменяются
- **WHEN** анализируется observability snapshot
- **THEN** в метриках видны отмены по `superseded_generation`/`superseded_version`
- **AND** они не смешиваются с `client_cancel`
