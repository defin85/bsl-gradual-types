# Design: IntelliSense v2 (completion‑completeness)

## Позиционирование
Эта change вводит capability `bsl-intellisense-v2` как **current truth** для IntelliSense v2.

Связи:
- `openspec/specs/bsl-intellisense/spec.md` — минимальный LSP “core” контракт.
- `openspec/changes/define-bsl-intellisense-ide-grade/` — target‑spec (north star) для IDE‑grade.
- `bsl-intellisense-v2` — проверяемые требования и критерии, опирающиеся на существующий код/тесты v2.

## Источники истины
- Roadmap: `docs/roadmap/intellisense-v2-roadmap/intellisense-v2-roadmap.md`.
- Milestones M1–M8: `docs/roadmap/intellisense-v2-roadmap/m*-implementation-plan.md`.
- Тесты и скрипты запуска: как минимум то, что перечислено в M1 и M8 (incremental/LSP/matrix).

## Принципы v2
- IR‑first и пер‑документные снапшоты (без повторного тяжёлого анализа в hot path).
- Детерминизм результатов completion (стабильный порядок, стабильные идентификаторы кандидатов).
- Инкрементальность и корректность после `didChange` (позиции UTF‑16 ↔ byte ↔ tree‑sitter согласованы).
- Отмена запросов и отсутствие блокирующего I/O в горячих LSP путях.
