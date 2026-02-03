## 1. Proposal / Design
- [x] 1.1 Зафиксировать effective контракт включения flow-sensitive для IDE/LSP, Web API и MCP (default OFF) и перечислить breaking места (если есть).
- [x] 1.2 Согласовать минимальную bias‑модель `node_at_byte_offset(offset, bias)` для completion/hover/diagnostics.

## 2. CFG contract (shared + analysis-v2)
- [x] 2.1 Сделать `SemanticProgram.cfg` всегда `Some` (минимум: `Entry -> Exit` даже без исполняемых конструкций) и добавить тесты.
- [x] 2.2 Вынести/реализовать единый детерминированный API `ControlFlowGraph::node_at_byte_offset(offset, bias)` и покрыть тестами (границы токенов, пустые ветки).
- [x] 2.3 Расширить null-safety: учитывать null-check в `LoopHeader { condition }` и добавить регрессионные тесты.

## 3. analysis-v2 flow-sensitive queries
- [x] 3.1 Добавить отдельные v2 queries для flow-sensitive результатов (type-at-position + diagnostics), которые вызываются только при включении.
- [x] 3.2 Обеспечить fallback семантику: если flow-sensitive не применим, интерфейс возвращает базовый v2 результат без ухудшения UX.

## 4. LSP wiring
- [x] 4.1 Добавить workspace setting `enableFlowSensitive` (default false) и гарантировать, что при OFF flow-sensitive queries не вызываются.
- [x] 4.2 Hover/Completion/Definition/SignatureHelp/Diagnostics: при включении использовать flow-sensitive результаты; добавить тесты ON/OFF.

## 5. Web API wiring
- [x] 5.1 Перевести параметры на `includeFlowSensitive` (default false) и явно отклонять legacy `include_flow_sensitive` (breaking) с `400 Bad Request`.
- [x] 5.2 Добавить смоук/интеграционные тесты Web API на ON/OFF (и на отказ legacy ключа).

## 6. MCP (bsl-agent) wiring
- [x] 6.1 Добавить `include_flow_sensitive` (default false) в `bsl_type_at_position_start`, `bsl_members_start`, `bsl_diagnostics_start`.
- [x] 6.2 Добавить в ответы явный индикатор effective режима (например, `flow_sensitive_enabled: bool`) и тесты контрактов ON/OFF.

## 7. Docs / Migration notes
- [x] 7.1 Обновить документацию/спеки о включении flow-sensitive и о breaking изменениях Web API (если применимо).

## 8. Validation
- [x] 8.1 `openspec validate fix-flow-sensitive-v2-wiring --strict --no-interactive`.
- [x] 8.2 `cargo test --workspace` (после реализации; до архивации change).
