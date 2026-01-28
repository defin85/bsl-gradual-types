## 1. Реализация
- [ ] 1.1 Найти в `docs/**` и `scripts/**` ссылки на устаревшие пути (включая `backend/src/system/tree_sitter_adapter.rs`).
- [ ] 1.2 Обновить `docs/guides/roadmap-verification.md` на актуальные пути и команды проверки.
- [ ] 1.3 Обновить `scripts/compact_completed_milestones.py` (ссылки на файлы/пути) на актуальные.
- [ ] 1.4 Добавить в `docs/guides/roadmap-verification.md` короткий раздел «Как проверять актуальность ссылок на файлы» (ручной чек: `rg`/`find`).

## 2. Валидация
- [ ] 2.1 `rg -n "backend/src/system/tree_sitter_adapter\\.rs" -S .` возвращает пусто.
- [ ] 2.2 `openspec validate update-docs-and-scripts-consistency --strict --no-interactive`.

