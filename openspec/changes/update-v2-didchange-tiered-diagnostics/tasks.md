## 1. Спецификация и контракт
- [ ] 1.1 Уточнить `bsl-intellisense-v2` требования для tiered diagnostics pipeline (`fast` на `didChange`, `debounced_full`, `idle/didSave`).
- [ ] 1.2 Зафиксировать revision-bound cancellation и publish gate по актуальному revision token (включая generation).
- [ ] 1.3 Зафиксировать observability dimensions `trigger/profile/reason` в каноническом event model без high-cardinality.
- [ ] 1.4 Обновить `mcp-bsl-agent` требования для `documents_set`-revision cancellation и профильного разделения interactive/background операций.

## 2. LSP Runtime и Diagnostics Scheduling
- [ ] 2.1 Добавить/обновить scheduler профилей diagnostics: `didChange -> fast`, `debounced_full`, `didSave/idle -> heavy`.
- [ ] 2.2 Реализовать supersede-механику для устаревших задач по версии/поколению: старая задача завершает работу до публикации и до следующей дорогой стадии.
- [ ] 2.3 Обновить publish gate: публикация diagnostics только для актуального `(file_version, deps_id, settings_id, diagnostics_generation)`.
- [ ] 2.4 Ограничить didChange-path только дешёвыми шагами; дорогие проверки вынести в `didSave/idle`.

## 3. Shared Runtime и bsl-agent
- [ ] 3.1 Провести общую policy-реализацию в `bsl-runtime`, чтобы поведение наследовалось LSP и MCP.
- [ ] 3.2 Для `bsl-agent` внедрить отмену batch-задач по `session/document revision` при новых `documents_set`.
- [ ] 3.3 Убедиться, что интерактивные MCP tools (`type_at_position`, `members`, `definition`) не блокируются тяжёлыми batch-проходами.

## 4. Валидация
- [ ] 4.1 Добавить тесты на supersede/cancellation по версии: серия быстрых `didChange` не приводит к последовательной публикации устаревших вычислений.
- [ ] 4.2 Добавить тесты профилей триггеров: heavy-проверки запускаются только на `didSave/idle`, а не на каждый `didChange`.
- [ ] 4.3 Добавить contract tests для observability (`trigger/profile/reason`, discard stale runs, cancellation причины).
- [ ] 4.4 Добавить mixed-load regression для `bsl-agent` (batch + interactive) с проверкой отсутствия starvation интерактивного пути.
- [ ] 4.5 Прогнать `cargo test` для затронутых crates и зафиксировать результаты.
