## 0. Architecture Lock
- [x] 0.1 Подтвердить в implementation PR, что реализация следует зафиксированной целевой архитектуре per-file event-driven orchestrator без архитектурных отклонений.

## 1. Specification & Contracts
- [x] 1.1 Уточнить delta-spec: event envelope (`file_id`, `file_seq`, `request_id`, `request_epoch`, `version_hint`) и publish-инварианты latest-wins.
- [x] 1.2 Уточнить delta-spec: bounded queue + overflow policy (coalescing `DidChange`, вытеснение устаревших completion, non-droppable `Cancel`).
- [x] 1.3 Уточнить delta-spec: cancellation contract (`$/cancelRequest` -> `Cancel(request_id)` -> checkpoint stop + no late publish).
- [x] 1.4 Уточнить delta-spec: mode rollout (`off|shadow|canary|on`) и kill-switch rollback.
- [x] 1.5 Уточнить delta-spec: mode-aware observability (`mode=legacy|event_driven|shadow`) и rollout gates.
- [x] 1.6 Зафиксировать baseline метрик (conf_big start/cold/warm, дата + n + p95/p99) как reference для acceptance.

## 2. Runtime Configuration
- [x] 2.1 Добавить runtime key `BSL_INTELLISENSE_V2_COMPLETION_MODE` (`off|shadow|canary|on`, default=`off`).
- [x] 2.2 Добавить runtime key `BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT` (`0..100`, deterministic routing input).
- [x] 2.3 Добавить runtime key `BSL_INTELLISENSE_V2_COMPLETION_QUEUE_CAPACITY` (bounded capacity с clamp).
- [x] 2.4 Добавить тесты валидации/нормализации runtime keys (tier/default/clamp).

## 3. Orchestrator Core (LSP/runtime boundary)
- [x] 3.1 Ввести queue abstraction и per-file dispatcher registry (`file_id -> actor`).
- [x] 3.2 Реализовать actor lifecycle: create on first event, cleanup on `DidClose`, drain/stop semantics.
- [x] 3.3 Реализовать deterministic ordering по `file_seq` и monotonic `request_epoch`.
- [x] 3.4 Реализовать latest-wins scheduling и publish guard (`request_epoch == latest_epoch`).
- [x] 3.5 Реализовать overflow strategy для saturation (без неограниченного backlog).

## 4. Cancellation & Fallback Semantics
- [x] 4.1 Добавить request-level cancellation registry (`request_id -> token + file_id + epoch`).
- [ ] 4.2 Пробросить `Cancel(request_id)` в orchestrator и runtime stage-checkpoints (`wait/snapshot/ir/collect/rank/format/publish`).
- [ ] 4.3 Гарантировать no-late-publish для cancelled/superseded completion.
- [ ] 4.4 Централизовать fallback/degraded policy в orchestrator/runtime, удалить adapter-local дублирование.

## 5. Rollout & Routing
- [ ] 5.1 Реализовать mode routing: `off` legacy only, `shadow` dual-exec/legacy-response, `canary` percentage routing, `on` event-driven default.
- [ ] 5.2 Сделать canary routing детерминированным и воспроизводимым.
- [ ] 5.3 Реализовать kill-switch rollback в `off` без рестарта.

## 6. Observability
- [ ] 6.1 Расширить canonical observability contract mode-aware low-cardinality dimension.
- [ ] 6.2 Добавить mode-split completion stage metrics для `runtime_wait_for_file_version`, `runtime_snapshot_with_deps`, `ir_query`, `parse_result_query`.
- [ ] 6.3 Сохранить deterministic dual-write projection (drilldown primary, legacy compatibility).
- [ ] 6.4 Добавить contract tests на mode dimension + projection parity.

## 7. Validation
- [ ] 7.1 Добавить контрактные тесты порядка событий и latest-wins поведения под burst-нагрузкой.
- [ ] 7.2 Добавить контрактные тесты cancellation propagation и отсутствия late publish.
- [ ] 7.3 Добавить тесты bounded backpressure/fairness (interactive не starving под background и наоборот).
- [ ] 7.4 Добавить parity-тесты между `off`/`shadow`/`canary`/`on` на фиксированной ревизии.
- [ ] 7.5 Прогнать профильные наборы (`p26` + acceptance suite) и зафиксировать pass/fail по rollout gates.
- [ ] 7.6 Прогнать `openspec validate refactor-v2-completion-event-driven-pipeline --strict --no-interactive`.
- [ ] 7.7 Задокументировать baseline vs event-driven (cold/warm snapshot + SLO pass/fail + mode-split метрики) в change-артефактах.
