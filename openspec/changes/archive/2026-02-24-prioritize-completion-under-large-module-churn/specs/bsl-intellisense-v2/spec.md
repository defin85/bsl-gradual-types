## ADDED Requirements

### Requirement: Scale-aware diagnostics policy защищает интерактивный путь на больших модулях при churn (MUST)
Система MUST определять состояние `large + churn` для текущего документа и в этом состоянии MUST переключать diagnostics orchestration в интерактивно-безопасный режим.

Для состояния `large + churn`:
- `textDocument/didChange` MUST выполнять только fast diagnostics path;
- тяжелые diagnostics стадии (`debounced_full`, `idle_heavy`) MUST NOT запускаться синхронно на каждый `didChange`;
- тяжелые стадии MUST запускаться только по `idle` и/или `didSave` trigger;
- strict latest-version publish инварианты для diagnostics MUST сохраняться.

#### Scenario: Heavy diagnostics не конкурирует с completion на каждый символ в `large + churn`
- **GIVEN** открыт большой модуль, и IDE генерирует burst `didChange`
- **WHEN** система классифицирует состояние как `large + churn`
- **THEN** на `didChange` выполняется только fast path
- **AND** heavy diagnostics переносится на `idle`/`didSave`
- **AND** интерактивный completion обслуживается без синхронного ожидания heavy path

### Requirement: Runtime scheduling имеет явный интерактивный приоритет с fairness для background (MUST)
Система MUST обслуживать интерактивные операции (`completion`, `hover`, `signatureHelp`) с приоритетом относительно background diagnostics задач в runtime очередях.

Система MUST одновременно обеспечивать fairness:
- background diagnostics MUST получать гарантированный прогресс;
- интерактивный приоритет MUST NOT приводить к бесконечному starvation background diagnostics.

#### Scenario: Интерактивный запрос не блокируется backlog background задач
- **GIVEN** в runtime очереди накоплен background diagnostics backlog
- **WHEN** приходит интерактивный completion запрос
- **THEN** интерактивный запрос обслуживается с приоритетом
- **AND** background backlog продолжает выполняться по fairness-правилам

### Requirement: Observability отражает policy-переходы `large + churn` и причины deferred heavy-path (MUST)
Система MUST публиковать low-cardinality observability сигналы для scale-aware policy:
- факт входа/выхода из `large + churn`;
- причины отложенного heavy diagnostics запуска;
- связь policy-переходов с stage-level latency completion пути.

#### Scenario: Root-cause задержки completion локализуется через policy и stage метрики
- **GIVEN** растет latency интерактивного completion на большом модуле
- **WHEN** анализируется observability snapshot
- **THEN** видны события policy-переходов `large + churn`
- **AND** по stage-level метрикам можно отделить queue contention от query bottleneck
