## 1. Specification
- [ ] 1.1 Добавить requirement в `bsl-intellisense-v2` про event-driven orchestration интерактивного completion pipeline.
- [ ] 1.2 Добавить requirement в `bsl-intellisense-v2` про deterministic ordering + latest-wins semantics под burst `didChange`/completion.
- [ ] 1.3 Добавить requirement в `bsl-intellisense-v2` про rollout/rollback контракт (feature flag + observability + safe fallback path).

## 2. Architecture & Design
- [ ] 2.1 Зафиксировать event model (`DidOpen/DidChange/CompletionRequest/Cancel/DidClose`) и инварианты порядка для per-file stream.
- [ ] 2.2 Спроектировать policy коалесцирования и cancellation propagation до тяжёлых стадий runtime.
- [ ] 2.3 Определить SLI/SLO и observability contract: queue wait, exec latency, cancel ratio, stale/degraded ratio, parity drift.
- [ ] 2.4 Описать migration/rollout план dual-mode и kill-switch rollback.

## 3. Implementation
- [ ] 3.1 Ввести orchestrator queue abstraction и per-file dispatcher в LSP/runtime boundary.
- [ ] 3.2 Перевести completion hot path на event-driven планировщик с latest-wins семантикой.
- [ ] 3.3 Централизовать cancellation/degraded policy в orchestrator слое.
- [ ] 3.4 Добавить feature-flag переключение legacy/runtime-centric vs event-driven путь.

## 4. Validation
- [ ] 4.1 Добавить контрактные тесты порядка событий и latest-wins поведения под burst-нагрузкой.
- [ ] 4.2 Добавить тесты cancellation/starvation и bounded latency для event-driven режима.
- [ ] 4.3 Добавить parity-тесты между legacy/runtime-centric и event-driven режимами на фиксированной ревизии.
- [ ] 4.4 Прогнать профильные наборы и `openspec validate refactor-v2-completion-event-driven-pipeline --strict --no-interactive`.
