## ADDED Requirements

### Requirement: Syntax diagnostics использует инкрементальный parse для последовательных ревизий файла (MUST)
Система MUST использовать incremental parse path для syntax diagnostics, когда доступна предыдущая ревизия того же файла и валидное описание изменений (`didChange` edits).

Incremental path MUST:
- переиспользовать предыдущее parse tree текущего файла;
- применять edit mapping к предыдущему дереву;
- выполнять parse новой ревизии с учетом предыдущего дерева;
- обновлять canonical parse state файла на текущую ревизию.

#### Scenario: Последовательные правки большого модуля обслуживаются через incremental parse
- **GIVEN** файл уже имеет предыдущую синтаксическую ревизию
- **AND** приходит новая ревизия через `didChange` с валидными edits
- **WHEN** запускается syntax diagnostics
- **THEN** система использует incremental parse path
- **AND** canonical parse state обновляется на новую ревизию

### Requirement: Incremental syntax path имеет детерминированный fail-safe fallback на full parse (MUST)
Система MUST выполнять full parse текущей ревизии, если incremental parse path не может быть корректно применен.

Fallback MUST срабатывать при любом из условий:
- отсутствует предыдущее parse tree для файла;
- edit mapping некорректен или не может быть применен;
- incremental parse возвращает невалидный результат для текущей ревизии.

Fallback MUST сохранять корректность и детерминированность user-facing diagnostics.

#### Scenario: Невалидный incremental update не ломает diagnostics
- **GIVEN** для ревизии файла incremental update не может быть применен
- **WHEN** система строит syntax diagnostics
- **THEN** система выполняет full parse текущей ревизии
- **AND** возвращает корректные diagnostics без деградации semantic контракта

### Requirement: Observability фиксирует эффективность incremental syntax path (MUST)
Система MUST публиковать low-cardinality observability для incremental syntax path, включая:
- incremental hit/miss/fallback счётчики;
- причины fallback;
- stage-level latency для syntax diagnostics по каждому пути.

#### Scenario: Метрики показывают, где incremental path не сработал
- **GIVEN** mixed нагрузка, где часть ревизий обрабатывается incremental, часть через fallback
- **WHEN** запрашивается observability snapshot
- **THEN** в метриках доступны hit/miss/fallback и причины fallback
- **AND** latency syntax стадии можно сравнить между incremental и full parse путями
