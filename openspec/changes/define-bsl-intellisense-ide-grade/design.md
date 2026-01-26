# Design: целевой IntelliSense (IDE‑grade) для BSL (1С)

## 0) Позиционирование
Эта change задаёт **target‑spec** (north star) `bsl-intellisense-ide-grade`.

Важно:
- `openspec/specs/bsl-intellisense/spec.md` фиксирует **текущее** минимальное поведение (core).
- `openspec/specs/bsl-intellisense-ide-grade/spec.md` фиксирует **целевое** IDE‑grade состояние (не обязательно реализовано сегодня).

## 1) Приоритеты MUST vs SHOULD (решение)
В терминах IDE‑grade для BSL мы считаем MUST то, без чего пользовательский опыт “ломается” в ежедневной работе:
- полнота completion по выражениям + stdlib+metadata (ядро 1С‑проектов),
- стабильность/детерминизм/инкрементальность (иначе подсказкам нельзя доверять),
- базовая навигация и refactor‑гигиена: symbols + references + rename (пусть и с чётко описанными границами).

SHOULD — то, что сильно улучшает UX, но требует дополнительных продуктовых решений/стиля/глубокой интеграции:
- форматирование (в 1С часто есть внешний “каноничный” форматтер/стандарты),
- code actions (богатый набор quick fixes/рефакторингов),
- inlay hints (type hints) как отдельный слой UX.

## 2) Scope и источники
Основной клиент: VS Code.
Требования формулируются так, чтобы оставаться совместимыми с любым LSP‑клиентом (не привязываться к уникальным UI VS Code там, где можно).

Вне scope (отдельно и позже):
- Поддержка языка запросов 1С внутри строк (completion/diagnostics/hover для query‑DSL).

## 3) Декомпозиция на работы (куда лягут реализации)
Текущие candidate changes для реализации частей target‑spec:
- Symbols: `openspec/changes/add-bsl-lsp-symbols/`
- References+Rename: `openspec/changes/add-bsl-lsp-references-and-rename/`
- Formatting strategy: `openspec/changes/evaluate-bsl-formatting/`
- VS Code providers (no stubs by default): `openspec/changes/implement-bsl-vscode-enhanced-providers/`

Completion IDE‑grade (выражения + stdlib+metadata): см. `docs/roadmap/intellisense-v2-roadmap/intellisense-v2-roadmap.md`.

### Mapping требований target‑spec → roadmap / changes

- **Core IntelliSense (LSP):** уже зафиксирован в `openspec/specs/bsl-intellisense/spec.md` (минимальный контракт). Target‑spec `bsl-intellisense-ide-grade` требует сохранять/расширять это ядро.
- **IDE‑grade completion по выражениям + stdlib + metadata (MUST):** реализуется по roadmap IntelliSense v2 (milestones M1–M8).
- **Детерминизм/инкрементальность/отмена/No blocking I/O (MUST):** реализуется как нефункциональная часть roadmap IntelliSense v2 (например, M1/M2/M7) и должна применяться ко всем LSP фичам.
- **Symbols (MUST):** `openspec/changes/add-bsl-lsp-symbols/`.
- **Find References + Rename (MUST):** `openspec/changes/add-bsl-lsp-references-and-rename/` (включая документирование поддерживаемых классов символов).
- **Formatting (SHOULD):** `openspec/changes/evaluate-bsl-formatting/` (выбор стратегии/интеграции форматтера и критерии детерминизма/минимального diff).
- **Code Actions / Quick Fixes (SHOULD):** `openspec/changes/implement-bsl-vscode-enhanced-providers/` (как минимум: не регистрировать пустые заглушки; далее: наполнение quick fixes).
- **Inlay hints (SHOULD):** `openspec/changes/implement-bsl-vscode-enhanced-providers/` (как минимум: не регистрировать пустые заглушки; далее: наполнение type hints).
