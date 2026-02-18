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

### Requirement: Канонический event model является единственным источником observability semantics (MUST)
Система MUST описывать emission observability через единый канонический event model (transport-agnostic), общий для LSP/web/MCP.

Каноническое событие MUST включать:
- `family`;
- `origin`;
- `value`;
- `operation` и `stage` для stage-семейств.

Контекстные измерения (`outcome`, `reason`, `query_kind`, `work_class`) MAY применяться только там, где это разрешено schema правилом `family`.

Недопустимые сочетания измерений MUST NOT публиковаться как отдельные метрики и MUST фиксироваться контрактным сигналом нарушения schema.

#### Scenario: Семантика метрик определяется каноническим событием, а не адаптером
- **GIVEN** одинаковая semantic операция выполняется через LSP и MCP
- **WHEN** оба адаптера эмитят observability
- **THEN** формируются канонически эквивалентные события (с разницей только в `origin`)
- **AND** итоговые drilldown/legacy метрики вычисляются из этих событий без adapter-local semantic ветвлений

### Requirement: Dual-write rollout использует единый канонический observability контракт (MUST)
При внедрении drilldown слоя система MUST сохранять backward compatibility fixed-key метрик через dual-write из одного канонического источника событий.

Система MUST соблюдать следующие инварианты:
- канонический контракт задаёт семантику метрик;
- drilldown является primary representation канонического контракта;
- legacy fixed keys являются compatibility-проекцией канонического контракта и MUST NOT иметь отдельную независимую семантику;
- mapping каноника -> fixed keys MUST быть детерминированным и единым для LSP/web/MCP;
- dual-write materialization MUST выполняться в одном centralized projection pipeline (backend-first) в shared runtime;
- adapter-layer MUST NOT публиковать drilldown/legacy метрики напрямую в обход канонического event pipeline.

#### Scenario: Dual-write сохраняет совместимость без semantic drift
- **GIVEN** после внедрения drilldown запрошен observability snapshot
- **WHEN** запускаются текущие контрактные проверки fixed-key метрик и проверка соответствия каноника -> legacy projection
- **THEN** fixed-key проверки проходят без изменения ожидаемых legacy ключей
- **AND** snapshot дополнительно содержит drilldown ключи
- **AND** значения legacy fixed keys согласованы с агрегированной канонической drilldown моделью

#### Scenario: Один канонический event детерминированно материализует обе проекции
- **GIVEN** в shared runtime эмитится каноническое событие наблюдаемости
- **WHEN** выполняется materialization dual-write представления
- **THEN** одновременно формируются корректные drilldown и legacy значения согласно единому mapping-реестру
- **AND** отсутствует adapter-local bypass, который мог бы изменить одну проекцию независимо от другой

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
