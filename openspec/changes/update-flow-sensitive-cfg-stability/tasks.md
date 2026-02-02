## 1. Proposal / Design
- [x] 1.1 Зафиксировать контракт CFG v2: `cfg` всегда `Some`, минимальный скелет Entry→Exit, правила spans для условных/ветвящих узлов.
- [x] 1.2 Согласовать контракт “byte offset → CFG node” (bias‑модель для completion/hover/diagnostics) и критерии детерминизма.

## 2. CFG always-present (analysis-v2)
- [x] 2.1 Изменить построение CFG в `bsl-analysis-v2`, чтобы `SemanticProgram.cfg` всегда был `Some(ControlFlowGraph)` (минимум: Entry→Exit).
- [x] 2.2 Уточнить spans для `Conditional`/`LoopHeader`: span “шапки” не должен покрывать тело веток/цикла; тело должно попадать в отдельные узлы (включая пустые ветки через placeholder).

## 3. Deterministic position→CFG mapping (shared + runtime)
- [x] 3.1 Добавить в `ControlFlowGraph` публичный API `node_at_byte_offset(offset, bias)` (или эквивалент) с детерминированным выбором “самого специфичного” узла по span.
- [x] 3.2 Перевести flow-sensitive runtime wiring (hover/completion) на этот API и убрать эвристику “32 байта назад” и угадывание ветки, если оно становится не нужно при корректных spans.

## 4. Flow-sensitive null-safety in loops
- [x] 4.1 Расширить null-safety анализатор так, чтобы `LoopHeader { condition }` учитывался как условие null-check (аналогично `Conditional { condition }`).
- [x] 4.2 Добавить тесты на `Пока x <> Null Цикл` / `Пока ЗначениеЗаполнено(x) Цикл` и отсутствие warning при dereference в теле.

## 5. Tests
- [x] 5.1 Тест: `cfg` всегда `Some` (включая “только декларации”, пустые тела процедур/функций).
- [x] 5.2 Тесты на позиционирование: completion на `.` (owner слева), hover на границах токенов, пустые ветки if/loop.
- [x] 5.3 Тест: детерминизм (повторные вызовы mapping дают одинаковый результат для одинаковых входов).

## 6. Validation
- [x] 6.1 `openspec validate update-flow-sensitive-cfg-stability --strict --no-interactive`.
- [x] 6.2 `cargo test --workspace` (после реализации; до архивации change).
