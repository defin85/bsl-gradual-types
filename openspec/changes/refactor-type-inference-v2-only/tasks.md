## 1. Подготовка (аудит)
- [ ] 1.1 Инвентаризировать все места использования `bsl-semantic` (включая `backend/`, `analysis-v2/`, тесты, docs).
- [ ] 1.2 Инвентаризировать legacy пути inference (прямые вызовы `TypeResolver`/альтернативные IR кеши/обход v2 runtime).
- [ ] 1.3 Зафиксировать реализацию AST→IR по варианту A (перенос в `bsl-analysis-v2`) и определить целевой модуль/публичный API.

## 2. Перенос AST→IR и выравнивание зависимостей
- [ ] 2.1 Перенести код AST→IR из `bsl-semantic` в `bsl-analysis-v2` (новый модуль `ast_to_ir` или эквивалент).
- [ ] 2.2 Убедиться, что AST→IR является минимальным и не содержит самостоятельного inference/эвристик (IR только структура/связи/позиции; типы вычисляются поверх IR в v2).
- [ ] 2.3 Обновить `bsl-analysis-v2` и `bsl-backend` зависимости: больше не зависят от `bsl-semantic`.

## 3. Удаление legacy пути
- [ ] 3.1 Удалить/выпилить legacy‑код в backend, который делает inference вне v2 snapshot.
- [ ] 3.2 Ввести/закрепить единый сервис/фасад в backend application, который получает типовую информацию только из v2.
- [ ] 3.3 Удалить/обновить устаревшие re-export обёртки (например, `backend/src/application/ast_to_ir/mod.rs`), чтобы не было скрытых обходов.

## 4. Удаление `bsl-semantic` из workspace
- [ ] 4.1 Удалить crate `semantic/` из `[workspace.members]` и `[workspace.dependencies]`, обновить импорты.
- [ ] 4.2 Удалить связанные упоминания в скриптах сборки (например, `scripts/build-all.sh`).

## 5. Тесты и документация
- [ ] 5.1 Обновить/починить тесты, которые зависели от `bsl_semantic::AstToIrConverter` (перевести на v2 API).
- [ ] 5.2 Обновить документацию v2 roadmap/архитектуры, где `bsl-semantic` описан как слой (переписать на v2‑only).

## 6. Валидация
- [ ] 6.1 `cargo test -p bsl-analysis-v2 -p bsl-backend` проходит.
- [ ] 6.2 `rg -n "bsl_semantic|bsl-semantic" -S .` не находит ссылок в коде (допускаются только исторические упоминания в archive docs, если явно помечены как архив).
- [ ] 6.3 `openspec validate refactor-type-inference-v2-only --strict --no-interactive`.
