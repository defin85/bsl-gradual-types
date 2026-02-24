## Context
Текущая observability показывает, что на больших модулях интерактивный completion в warm-path имеет тяжёлый хвост, тогда как на маленьких модулях latency остаётся низкой.

Практический риск: оптимизации могут либо
- улучшать только synthetic/small профиль,
- либо ускорять large-path ценой регрессии на small-path.

Нужен единый, воспроизводимый и объективный критерий ускорения именно для больших модулей.

## Goals / Non-Goals
- Goals:
  - Ввести объективный scale-aware gate (`large` + `small`) для completion v2.
  - Привязать ускорение `large` к baseline ratio-целям, а не к субъективной оценке.
  - Сохранять видимость root-cause по стадиям completion-контура.
- Non-Goals:
  - Переписывать completion ranking/candidate semantics.
  - Менять внешний LSP-контракт completion.

## Decisions
- Decision 1: Базовый протокол измерения
  - Используем LSP observability snapshot в трёх фазах: `start`, `cold`, `warm`.
  - Для acceptance используем `warm` как primary signal, `start/cold` как диагностический контекст.

- Decision 2: Scale-aware профили
  - Проверяем минимум два профиля в одном прогоне:
    - `large` (реальный тяжёлый модуль);
    - `small` (контрольный лёгкий модуль).
  - Это исключает оптимизации, которые улучшают только одну сторону.

- Decision 3: Objective targets через ratio к baseline
  - `large` оценивается по ratio относительно frozen baseline, чтобы убрать зависимость от абсолютного железа.
  - `small` оценивается как non-regression ratio guard.

- Decision 4: Stage-level локализация
  - Gate-отчёт обязан включать stage-level completion метрики:
    - `wait_for_file_version_completion`;
    - `snapshot_completion`;
    - `ir_query_completion`.
  - Это позволяет отличать scheduler/wait bottleneck от query bottleneck.

## Target Ratios (for this change)
- `large` warm-path:
  - `p95(wait_for_file_version_completion_ms) <= 0.60 * baseline_large_wait_for_file_version_p95_ms`.
  - `p95(completion_duration_ms) <= 0.75 * baseline_large_completion_duration_p95_ms`.
- `small` warm-path:
  - `p95(completion_duration_ms) <= 1.25 * baseline_small_completion_duration_p95_ms`.

## Risks / Trade-offs
- Риск нестабильности результатов на шумной машине.
  - Митигация: фиксированный сценарий, достаточный объём запросов, ratio к baseline вместо абсолютов.
- Риск «оптимизации под метрику» без реального UX-выигрыша.
  - Митигация: одновременно контролируем `completion_duration` и stage-level decomposition.
- Риск деградации small-path.
  - Митигация: отдельный small non-regression gate.

## Migration / Rollout
1. Зафиксировать baseline artifacts для `large/small`.
2. Включить scale-aware gate в CI как non-blocking report.
3. После стабилизации перевести gate в blocking.
4. Любое изменение target ratios — только через новый OpenSpec change.

## Open Questions
- Нужно ли дополнительно фиксировать `p99` в blocking-gate, или оставить `p95` как основной signal на этом этапе.
