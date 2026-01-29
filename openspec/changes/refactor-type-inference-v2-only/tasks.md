## 1. Подготовка (аудит)
- [x] 1.1 Инвентаризировать все места использования `bsl-semantic` (включая `backend/`, `analysis-v2/`, тесты, docs).
- [x] 1.2 Инвентаризировать legacy пути inference (completion resolve без candidate_id, `parse_to_ir`/`parse_and_analyze`, любые обходы v2 snapshot). См. `legacy-audit.md`.
- [x] 1.3 Зафиксировать реализацию AST→IR по варианту A (перенос в `bsl-analysis-v2`) и определить целевой модуль/публичный API.

## 2. Перенос AST→IR и выравнивание зависимостей
- [x] 2.1 Перенести код AST→IR из `bsl-semantic` в `bsl-analysis-v2` (новый модуль `ast_to_ir` или эквивалент).
- [ ] 2.2 Сделать AST→IR **минимальным**: удалить inference/эвристики и любые поля `TypeResolution` в IR; типы вычисляются поверх IR в v2 queries.
- [x] 2.3 Обновить `bsl-analysis-v2` и `bsl-backend` зависимости: больше не зависят от `bsl-semantic`.

## 3. IR byte offsets (позиционирование)
- [ ] 3.1 Перевести IR spans на byte offsets (вместо line/column) и обновить позиционирование через v2 line-index слой.
- [ ] 3.2 Обновить APIs поиска узлов/символов (hover/definition/diagnostics), чтобы входом был byte offset, а не (line, column).
- [ ] 3.3 Добавить/обновить тесты на корректность byte offset ↔ UTF‑16 конвертаций на реальных кейсах (кириллица, emoji, смешанные строки).

## 4. v2 type inference поверх минимального IR
- [ ] 4.1 Добавить v2 queries для вычисления типов (например: тип выражения/receiver chain, тип символа в scope) поверх минимального IR (без чтения I/O, только deps snapshot).
- [ ] 4.2 Перевести completion/hover/signatureHelp/definition/diagnostics на новый v2 inference API (без использования типовой информации из IR).
- [ ] 4.3 Зафиксировать контракт snapshot-safety: completion/hover/resolve используют один snapshot и стабильные идентификаторы (CandidateId вместо ExprId).

## 5. Удаление `bsl-semantic` из workspace
- [x] 5.1 Удалить crate `semantic/` из `[workspace.members]` и `[workspace.dependencies]`, обновить импорты.
- [x] 5.2 Удалить связанные упоминания в скриптах сборки (например, `scripts/build-all.sh`).

## 6. Тесты и документация
- [x] 6.1 Обновить/починить тесты, которые зависели от `bsl_semantic::AstToIrConverter` (перевести на v2 API).
- [x] 6.2 Обновить документацию v2 roadmap/архитектуры, где `bsl-semantic` описан как слой (переписать на v2‑only).
- [ ] 6.3 Обновить/починить тесты и golden snapshots, которые зависят от старых IR spans (line/column) или от `TypeResolution` внутри IR.

## 7. Удаление legacy путей (полностью)
- [ ] 7.1 Удалить `ParserCoordinator::parse_to_ir` и любые пути, которые строят IR вне v2 snapshot (включая `AnalysisEngine::parse_and_analyze`, если применимо).
- [ ] 7.2 Удалить legacy `completionItem/resolve` fallback (резолвинг без `candidate_id`).
- [ ] 7.3 Удалить legacy signatureHelp путь (и/или тестовый legacy handler), оставив только v2 API.
- [ ] 7.4 Убедиться, что LSP/web/CLI не имеют альтернативных inference путей (нет “best effort” обходов v2).

## 8. Валидация
- [ ] 8.1 `cargo test -p bsl-analysis-v2 -p bsl-backend` проходит.
- [ ] 8.2 `rg -n "parse_to_ir\\(|resolve_legacy\\(|handle_signature_help\\(" -S backend/ shared/` не находит legacy путей.
- [ ] 8.3 `rg -n "bsl_semantic|bsl-semantic" -S .` не находит ссылок в коде (кроме `openspec/` и `docs/roadmap/archive/`).
- [ ] 8.4 `openspec validate refactor-type-inference-v2-only --strict --no-interactive`.
