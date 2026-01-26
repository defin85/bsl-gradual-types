# Change: define-bsl-intellisense-v2

## Why
В репозитории есть подробная дорожная карта IntelliSense v2 (completion‑completeness) и фактическая реализация в коде/тестах, но в OpenSpec нет отдельной capability, которая фиксирует текущее состояние v2‑подхода, его границы и критерии готовности.

Нужна спецификация “current truth” для IntelliSense v2, чтобы:
- связывать target‑spec `bsl-intellisense-ide-grade` с проверяемыми критериями,
- иметь опорный контракт для регрессий (golden + LSP integration tests),
- упрощать планирование следующих улучшений и ревью.

## What Changes
- Добавить новую capability `bsl-intellisense-v2` (через change) с требованиями и сценариями:
  - IDE‑grade completion по выражениям,
  - детерминизм/инкрементальность/отмена,
  - интеграция stdlib + metadata,
  - однозначный resolve кандидатов,
  - тестовый “доказательный” набор (matrix + LSP incremental).
- Зафиксировать ссылки на источники истины: roadmap файлы, тесты и скрипты запуска.

## Impact
- Спецификация: новая capability `openspec/specs/bsl-intellisense-v2/spec.md` (через change).
- Код: без изменений (define/spec change).
