## 1. Backend contract и instrumentation
- [x] 1.1 Обновить delta для `bsl-intellisense-v2`: зафиксировать `contract=v12`, shape `first_poll_contention_attribution` и bounded vocabulary для contender-class / uri-scope semantics.
- [x] 1.2 Протянуть server-side contention snapshot через `request_context` и completion timeline producer path так, чтобы `service_future_created -> first_poll` получал bounded contender facts без request-id/URI leakage и без free-text debug payload.
- [x] 1.3 Добавить backend tests на `v12` contract, no-fabrication rules, bounded vocabulary и truthful `mixed|none_visible|unavailable` semantics.

## 2. Existing completion surfaces
- [x] 2.1 Обновить extension model/projection для Completion Timeline panel, clipboard export и request-centric incident bundle summary, чтобы новый `v12` attribution был виден рядом с existing `v11` first-poll / first-wake split.
- [x] 2.2 Добавить extension tests для `v12` projection и явной деградации на `v11`, без guessed blocker claims и без подмены server-side attribution client probes-данными.

## 3. Contracts, docs и validation
- [x] 3.1 Обновить versioned contract baseline `contracts/lsp-completion-timeline/v9/*` и связанный validation/tooling path.
- [x] 3.2 Обновить operator-facing manual/runbook evidence для `Completion Timeline v12` и incident bundle handoff.
- [x] 3.3 Прогнать `openspec validate add-completion-first-poll-contention-attribution --strict --no-interactive` и зафиксировать change в валидном состоянии.
