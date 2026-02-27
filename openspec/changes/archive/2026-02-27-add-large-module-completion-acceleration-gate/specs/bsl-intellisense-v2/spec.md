## MODIFIED Requirements

### Requirement: Interactive latency quality gate фиксирует warm-path SLO (MUST)
Система MUST выполнять интерактивный latency gate для completion v2 в двух профилях одного тестового цикла:
- `large` профиль (реальный тяжёлый модуль);
- `small` профиль (контрольный лёгкий модуль).

Gate MUST использовать versioned baseline artifact и рассчитывать ratio к baseline для каждого профиля.

Для `large` warm-path MUST выполняться оба условия:
- `p95(intellisense_v2_wait_for_file_version_completion_ms) <= 0.60 * baseline_large_wait_for_file_version_p95_ms`;
- `p95(completion_duration_ms) <= 0.75 * baseline_large_completion_duration_p95_ms`.

Для `small` warm-path MUST выполняться non-regression условие:
- `p95(completion_duration_ms) <= 1.25 * baseline_small_completion_duration_p95_ms`.

Дополнительно quality gate MUST проверять устойчивость completion outcomes для каждого профиля:
- `completion_cancelled_rate <= 0.10`, где `completion_cancelled_rate = intellisense_v2_completion_result_total_cancelled / completion_total`;
- прогон каждого профиля MUST включать не менее `50` последовательных completion-запросов в рамках одной сессии.

#### Scenario: Large profile показывает objective ускорение относительно baseline
- **GIVEN** выполнен warm-path прогон `large` профиля и доступен baseline artifact
- **WHEN** рассчитываются ratio для `wait_for_file_version_completion_ms` и `completion_duration_ms`
- **THEN** оба ratio укладываются в целевые границы (`<=0.60` и `<=0.75`)
- **AND** `completion_cancelled_rate` не превышает 10%

#### Scenario: Small profile не деградирует при оптимизации large profile
- **GIVEN** выполнен warm-path прогон `small` профиля и доступен baseline artifact
- **WHEN** рассчитывается `completion_duration_ms` ratio
- **THEN** ratio не превышает `1.25`
- **AND** `completion_cancelled_rate` не превышает 10%

## ADDED Requirements

### Requirement: Scale-aware baseline artifact для completion latency является обязательным и versioned (MUST)
Система MUST сохранять и использовать versioned baseline artifact для latency gate completion v2.

Baseline artifact MUST включать:
- профили `large` и `small`;
- фазы `start`, `cold`, `warm`;
- минимум следующие метрики для completion-контура:
  - `completion_duration_ms`;
  - `intellisense_v2_wait_for_file_version_completion_ms`;
  - `intellisense_v2_snapshot_completion_ms`;
  - `intellisense_v2_ir_query_completion_ms`;
- sample size (`n`) для каждой фазы/метрики;
- явный `pass/fail` summary по gate-критериям.

Gate MUST падать, если baseline artifact отсутствует, повреждён или не содержит обязательных полей.

#### Scenario: Gate использует baseline artifact и даёт воспроизводимый verdict
- **GIVEN** baseline artifact присутствует и валиден
- **WHEN** выполняется scale-aware perf прогон
- **THEN** система вычисляет ratio/threshold verdict детерминированно из baseline и текущих метрик
- **AND** итоговый отчёт содержит `pass/fail` и все обязательные поля

#### Scenario: Отсутствующий baseline блокирует принятие результата
- **GIVEN** baseline artifact отсутствует или невалиден
- **WHEN** запускается quality gate
- **THEN** gate завершается ошибкой конфигурации
- **AND** прогон не считается валидным доказательством ускорения
