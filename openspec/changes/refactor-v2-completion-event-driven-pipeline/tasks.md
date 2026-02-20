## 1. Specification
- [ ] 1.1 Добавить requirement в `bsl-intellisense-v2` про per-file event-driven dispatcher/actor и bounded queue для completion pipeline.
- [ ] 1.2 Добавить requirement в `bsl-intellisense-v2` про deterministic ordering + latest-wins semantics под burst `didChange`/completion.
- [ ] 1.3 Добавить requirement в `bsl-intellisense-v2` про cancellation propagation (`Cancel(request_id)` -> остановка тяжелых стадий на checkpoint'ах).
- [ ] 1.4 Добавить requirement в `bsl-intellisense-v2` про rollout/rollback контракт mode-based feature flag (`off|shadow|canary|on`).
- [ ] 1.5 Добавить requirement в `bsl-intellisense-v2` про mode-aware observability разрез для сравнения legacy vs event-driven.
- [ ] 1.6 Зафиксировать baseline метрик (conf_big start/cold/warm, дата + n + p95/p99) как reference для acceptance.

## 2. Architecture & Design
- [ ] 2.1 Зафиксировать event model (`DidOpen/DidChange/CompletionRequest/Cancel/DidClose`) и инварианты порядка (`file_seq`, `request_epoch`) для per-file stream.
- [ ] 2.2 Спроектировать latest-wins/coalescing policy и bounded backpressure policy (размер очереди + overflow strategy).
- [ ] 2.3 Спроектировать cancellation propagation contract до тяжёлых стадий runtime (`wait/snapshot/ir/collect/rank/format` checkpoints).
- [ ] 2.4 Определить SLI/SLO и observability contract: queue wait, exec latency, cancel ratio, stale/degraded ratio, parity drift, `mode`.
- [ ] 2.5 Описать migration/rollout план dual-mode (`off|shadow|canary|on`) и kill-switch rollback.
- [ ] 2.6 Привязать SLO-гейты rollout к автоматизированным smoke/regression тестам (`p26`/acceptance suite).

## 3. Implementation
- [ ] 3.1 Ввести runtime key для event-driven mode (`off|shadow|canary|on`) и wiring в LSP/runtime.
- [ ] 3.2 Ввести orchestrator queue abstraction и per-file dispatcher actor в LSP/runtime boundary (bounded queue).
- [ ] 3.3 Перевести completion hot path на event-driven планировщик с latest-wins семантикой.
- [ ] 3.4 Добавить request-level cancellation registry (`request_id -> token`) и propagation `Cancel(request_id)` до stage checkpoints.
- [ ] 3.5 Централизовать fallback/degraded policy в orchestrator/runtime слое и убрать adapter-local дублирование policy.
- [ ] 3.6 Добавить mode-aware observability разрез (`legacy|event_driven|shadow`) для completion stage metrics.

## 4. Validation
- [ ] 4.1 Добавить контрактные тесты порядка событий и latest-wins поведения под burst-нагрузкой.
- [ ] 4.2 Добавить контрактные тесты cancellation propagation (`Cancel(request_id)`) и гарантии отсутствия зависаний response path.
- [ ] 4.3 Добавить тесты bounded backpressure/fairness (interactive не starving под background и наоборот).
- [ ] 4.4 Добавить parity-тесты между legacy/runtime-centric и event-driven режимами на фиксированной ревизии (включая `shadow` сравнение).
- [ ] 4.5 Прогнать профильные наборы и `openspec validate refactor-v2-completion-event-driven-pipeline --strict --no-interactive`.
- [ ] 4.6 Задокументировать сравнение baseline vs event-driven (cold/warm snapshot + SLO pass/fail + mode-split метрики) в change-артефактах.
