## Context

Существующий completion pipeline содержит зрелые элементы (singleflight, stale fallback, bounded interactive knobs), но остаётся частично процедурным: части жизненного цикла `didChange`/completion синхронизируются через локальные блокировки и точечные guard'ы.

Это усложняет обеспечение предсказуемого интерактивного поведения при burst-нагрузке и росте параллелизма.

## Goals / Non-Goals

- Goals:
  - Перейти на event-driven orchestration интерактивного completion пути с явной моделью событий и очередей.
  - Гарантировать deterministic ordering и latest-wins обработку completion для актуальной ревизии документа.
  - Снизить зависимость от блокирующих ожиданий в hot path и стабилизировать tail latency.
  - Ввести безопасный rollout/rollback через feature flag и наблюдаемые SLI/SLO.
- Non-Goals:
  - Изменение доменной семантики completion (какие типы кандидатов возвращаются).
  - Переработка ranking/score модели.
  - Изменение пользовательских editor settings автоматически.

## Decisions

### Decision 1: Per-file event stream как источник оркестрации

Для каждого открытого документа вводится event stream с событиями как минимум:
- `DidOpen(version, text)`,
- `DidChange(version, diff|full_text)`,
- `CompletionRequest(request_id, position, trigger_mode, revision_hint)`,
- `Cancel(request_id)`,
- `DidClose`.

Порядок событий внутри файла MUST быть детерминированным.

### Decision 2: Коалесцирование и latest-wins для интерактивных completion

Completion scheduler MUST коалесцировать устаревшие задания и отдавать приоритет последнему релевантному запросу в интерактивном бюджете.

Запросы, потерявшие актуальность по ревизии/позиции, MUST отменяться до тяжёлых стадий (`snapshot/ir/collect/rank`), а не только в конце пайплайна.

### Decision 3: Разделение ingest и query path

`didChange` ingest не должен блокироваться ожиданием завершения предыдущих вычислений completion/diagnostics.

Согласованность достигается через версионированный state + event ordering, а не через блокирующую сериализацию всех шагов в hot path.

### Decision 4: Политика отмены и деградации как часть контракта orchestrator

Orchestrator MUST централизованно управлять:
- cancellation propagation,
- stale/degraded fallback policy,
- правилами `isIncomplete=true` для частичных ответов.

Это исключает расхождение между адаптером LSP и runtime policy.

### Decision 5: Feature-flag rollout и безопасный rollback

Event-driven режим включается через флаг (например, `bsl.intellisenseV2.eventDrivenCompletion`).
Требуется dual-mode поддержка:
- legacy/runtime-centric путь,
- event-driven путь.

Rollback MUST выполняться переключением флага без изменения пользовательских документов/настроек.

## Architecture Sketch

1. `LSP Adapter` публикует события в per-file orchestrator queue.
2. `Orchestrator` применяет ordering/coalescing/cancellation policy.
3. `Runtime Query Executor` выполняет bounded stages с приоритетами.
4. `Response Assembler` формирует LSP completion response + outcome/latency метрики.

## Migration Plan

1. Добавить флаг и dual-path orchestration каркас (без включения по умолчанию).
2. Подключить event stream для completion в shadow-режиме (метрики, но ответ берётся из legacy пути).
3. Включить event-driven ответы для canary-конфигураций.
4. После достижения целевых SLI/SLO включить по умолчанию.
5. Сохранить rollback до завершения стабилизации.

## Risks / Trade-offs

- Риск роста архитектурной сложности и стоимости сопровождения.
  - Митигация: чёткая ownership model по слоям (adapter/orchestrator/runtime), контрактные тесты.
- Риск скрытых starvation сценариев при неправильной политике приоритетов.
  - Митигация: fairness guards + метрики очередей и отмен.
- Риск временной деградации latency на этапе dual-mode.
  - Митигация: phased rollout, kill-switch, budget limits.

## Open Questions

- Нужен ли единый orchestrator для completion+hover+signatureHelp, или на первом этапе только completion.
- Требуется ли отдельная пер-file backpressure policy для очень больших модулей (порог по размеру/частоте событий).
