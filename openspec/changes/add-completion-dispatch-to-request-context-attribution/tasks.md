## 1. Контракт `v10` для bounded dispatch-to-request-context attribution
- [ ] 1.1 Зафиксировать additive `v10` semantics для `transport_received_at_ms_provenance`, `jsonrpc_dispatch_received_at_ms` и `dispatch_to_request_context_wait_ms`.
- [ ] 1.2 Зафиксировать relationship между legacy `transport_received_at_ms` и новым outer dispatch cut без guessed interpretation.
- [ ] 1.3 Сохранить bounded contract discipline и existing `v9` trustworthy semantics без free-text/high-cardinality полей.

## 2. Wiring backend producer path
- [ ] 2.1 Добавить outer dispatch hook до `RequestContextService`, чтобы authoritative path мог фиксировать `jsonrpc_dispatch_received_at_ms`.
- [ ] 2.2 Протянуть новый ingress cut через completion timeline producer path и сериализовать `dispatch_to_request_context_wait_ms`.
- [ ] 2.3 Сохранить honest fallback semantics, если outer dispatch timestamp недоступен, и не сломать overlapping/fallback request paths.

## 3. Projection и handoff
- [ ] 3.1 Обновить Completion Timeline panel и clipboard под `v10` dispatch split и explicit `v9` degradation.
- [ ] 3.2 Обновить request-centric incident bundle summary / findings / gaps так, чтобы dispatch split был виден без invented data, а на `v9` limitation называлась явно.

## 4. Проверка и фиксация
- [ ] 4.1 Добавить backend tests для `v10` contract, ingress provenance и derived dispatch-to-request-context wait.
- [ ] 4.2 Добавить extension tests для `v10` projection и explicit `v9` degradation в panel / clipboard / incident bundle.
- [ ] 4.3 Обновить smoke/runbook expectations для нового bounded split и для `v9` note `dispatch split unavailable by design`.
- [ ] 4.4 Зафиксировать `Requirement -> Code -> Test` traceability.
- [ ] 4.5 Провалидировать change через `openspec validate add-completion-dispatch-to-request-context-attribution --strict --no-interactive`.
- [ ] 4.6 Синхронизировать versioned contract baseline и canonical OpenSpec truth с shipped `v10` payload.
