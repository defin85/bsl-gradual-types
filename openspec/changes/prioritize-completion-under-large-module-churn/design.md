## Context
В текущем v2 пайплайне completion и diagnostics делят общие runtime ресурсы. На больших модулях при активном `didChange` это повышает latency интерактивного completion.

Наблюдаемая симптоматика из baseline:
- large warm: `p95(completion_duration_ms)=3910ms`;
- large warm: `p95(wait_for_file_version_completion_ms)=3024ms`.

Цель изменения: уменьшить интерактивный tail latency за счет политики исполнения, не меняя semantic correctness и не ломая strict-latest diagnostics publish.

## Goals / Non-Goals
- Goals:
  - Гарантировать интерактивный приоритет completion/hover/signatureHelp при churn.
  - Отвязать heavy diagnostics от каждого `didChange` на больших документах.
  - Сохранить измеримость root-cause через observability.
- Non-Goals:
  - Переписать алгоритмы синтаксического парсинга.
  - Менять внешний LSP контракт completion/diagnostics.

## Decisions
- Decision 1: Ввести scale-aware режим `large + churn`
  - `large` определяется по payload документа (размер/строки).
  - `churn` определяется по плотности `didChange` событий в коротком окне времени.
  - В режиме `large + churn` на `didChange` выполняется только fast diagnostics path.
  - Heavy path переносится в deferred `idle`/`didSave`.

  Alternatives considered:
  - Только увеличить debounce для всех файлов.
    - Отклонено: ухудшает small-path и не решает конкуренцию интерактивного пути с heavy задачами при burst editing.

- Decision 2: Разделить планирование интерактивных и background runtime команд
  - Интерактивные операции (`completion`, `hover`, `signatureHelp`) получают приоритет обслуживания.
  - Background diagnostics сохраняет гарантированный прогресс (fairness квота), чтобы не уйти в starvation.

  Alternatives considered:
  - Единая FIFO очередь с тюнингом таймаутов.
    - Отклонено: не устраняет head-of-line эффект при churn.

- Decision 3: Наблюдаемость policy-переходов
  - Добавляются low-cardinality сигналы:
    - вход/выход из `large + churn`;
    - причины отложенного heavy-path (`deferred_due_churn`, `deferred_due_large`, ...);
    - связь с stage-level latency для completion.

## Risks / Trade-offs
- Риск: stale diagnostics окно станет длиннее на больших файлах при активном наборе.
  - Mitigation: heavy-path остается обязательным на `idle/didSave`, strict latest-version publish сохраняется.
- Риск: приоритет интерактива может подавлять background слишком сильно.
  - Mitigation: фиксированная fairness квота и тесты на отсутствие starvation.
- Риск: неверная калибровка порогов `large/churn`.
  - Mitigation: пороги задаются runtime knobs и валидируются на perf сценарии `large/small`.

## Migration / Rollout
1. Включить режим как report-only policy (без enforced SLA на проде).
2. Подтвердить улучшение на scale-aware gate (large/small).
3. Перевести policy в default-on после стабильного периода.

## Open Questions
- Нужно ли вводить отдельные пороги `large` для bytes и lines, или достаточно одного доминирующего критерия.
- Нужна ли адаптивная fairness квота (динамическая) или фиксированная квота достаточна на первом этапе.
