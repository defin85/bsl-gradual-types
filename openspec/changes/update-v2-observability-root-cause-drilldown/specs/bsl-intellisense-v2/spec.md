## ADDED Requirements

### Requirement: Root-cause drilldown метрики semantic pipeline имеют фиксированную low-cardinality размерность (MUST)
Система MUST публиковать дополнительный stage-level observability слой, позволяющий локализовать latency/regression до комбинации:
- `origin` (минимум: `lsp`, `agent`),
- `operation` (значения из фиксированного `SemanticOperation` enum),
- `stage` (значения из фиксированного `ObservabilityStage` enum),
- `outcome` или `reason` (фиксированный набор).

Drilldown слой MUST оставаться low-cardinality:
- значения MUST браться только из фиксированных enum/классификаторов;
- metric keys MUST NOT включать путь файла, URI, symbol name, свободный пользовательский ввод.

Система MUST предоставлять минимум следующие семейства drilldown-метрик:
- stage totals;
- stage latency histograms;
- cancellation/outcome/reason counters;
- parse/IR skip reason counters.

#### Scenario: Узкое место локализуется до operation+stage+reason
- **GIVEN** в warm-path профиле растет `completion_duration_ms`
- **WHEN** анализируется observability snapshot
- **THEN** по drilldown-метрикам можно однозначно определить проблемную комбинацию `operation+stage`
- **AND** видно, что вклад вызван конкретной `reason` (например, cancellation или skip), а не агрегированным `*_other`

### Requirement: Rollout drilldown-метрик сохраняет fixed-key backward compatibility (MUST)
При внедрении drilldown слоя система MUST публиковать существующие fixed-key observability метрики параллельно с новыми drilldown метриками.

Семантика legacy fixed-key метрик MUST оставаться совместимой с текущим контрактом observability, чтобы существующие parity tests/dashboards не требовали одномоментной миграции.

#### Scenario: Существующие проверки observability продолжают проходить
- **GIVEN** после внедрения drilldown запрошен observability snapshot
- **WHEN** запускаются текущие контрактные проверки fixed-key метрик
- **THEN** проверки проходят без изменения ожидаемых legacy ключей
- **AND** snapshot дополнительно содержит новые drilldown ключи

### Requirement: Runtime saturation и singleflight effectiveness наблюдаемы отдельным слоем (MUST)
Система MUST публиковать observability-метрики, которые отделяют queue/CPU contention от логики semantic стадий.

Обязательные группы saturation/effectiveness метрик:
- waiters/permits/queue-depth для runtime budget/очередей;
- singleflight effectiveness по `query_kind` (leader/shared);
- сигнал о невозможности построить singleflight key (`key_unavailable`).

Все значения MUST быть low-cardinality и пригодны для агрегирования между интерфейсами.

#### Scenario: Queue contention различим от проблем semantic query
- **GIVEN** наблюдается рост `runtime_queue_wait` latency
- **WHEN** анализируется saturation/effectiveness слой
- **THEN** можно определить, вызван ли рост нехваткой runtime budget (waiters/permits/queue depth)
- **AND** можно оценить, помог ли singleflight (`shared`) или не сработал из-за `key_unavailable`

