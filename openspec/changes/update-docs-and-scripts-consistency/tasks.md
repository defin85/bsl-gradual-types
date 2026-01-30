## 1. Реализация
- [x] 1.1 Найти в `docs/**` и `scripts/**` ссылки на устаревшие пути (включая устаревшую ссылку на tree-sitter адаптер в backend system-слое).
- [x] 1.2 Обновить `docs/guides/roadmap-verification.md` на актуальные пути и команды проверки.
- [x] 1.3 Обновить `scripts/compact_completed_milestones.py` (ссылки на файлы/пути) на актуальные.
- [x] 1.4 Добавить в `docs/guides/roadmap-verification.md` короткий раздел «Как проверять актуальность ссылок на файлы» (ручной чек: `rg`/`find`).

## 2. Валидация
- [x] 2.1 `rg -n "backend/src/system/" -S --glob '!openspec/**' . | rg -n "tree_sitter_adapter\\.rs"` возвращает пусто.
- [x] 2.2 `openspec validate update-docs-and-scripts-consistency --strict --no-interactive`.
