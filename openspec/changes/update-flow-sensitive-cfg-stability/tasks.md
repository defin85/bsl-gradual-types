## 1. Proposal / Design
- [ ] 1.1 Зафиксировать контракт CFG v2: `cfg` всегда `Some`, минимальный скелет Entry→Exit, правила spans для условных/ветвящих узлов.
- [ ] 1.2 Согласовать контракт “byte offset → CFG node” (bias‑модель для completion/hover/diagnostics) и критерии детерминизма.

## 2. CFG always-present (analysis-v2)
- [ ] 2.1 Изменить построение CFG в `bsl-analysis-v2`, чтобы `SemanticProgram.cfg` всегда был `Some(ControlFlowGraph)` (минимум: Entry→Exit).
- [ ] 2.2 Уточнить spans для `Conditional`/`LoopHeader`: span “шапки” не должен покрывать тело веток/цикла; тело должно попадать в отдельные узлы (включая пустые ветки через placeholder).

## 3. Deterministic position→CFG mapping (shared + runtime)
- [ ] 3.1 Добавить в `ControlFlowGraph` публичный API `node_at_byte_offset(offset, bias)` (или эквивалент) с детерминированным выбором “самого специфичного” узла по span.
- [ ] 3.2 Перевести flow-sensitive runtime wiring (hover/completion) на этот API и убрать эвристику “32 байта назад” и угадывание ветки, если оно становится не нужно при корректных spans.

## 4. Flow-sensitive null-safety in loops
- [ ] 4.1 Расширить null-safety анализатор так, чтобы `LoopHeader { condition }` учитывался как условие null-check (аналогично `Conditional { condition }`).
- [ ] 4.2 Добавить тесты на `Пока x <> Null Цикл` / `Пока ЗначениеЗаполнено(x) Цикл` и отсутствие warning при dereference в теле.

## 5. Tests
- [ ] 5.1 Тест: `cfg` всегда `Some` (включая “только декларации”, пустые тела процедур/функций).
- [ ] 5.2 Тесты на позиционирование: completion на `.` (owner слева), hover на границах токенов, пустые ветки if/loop.
- [ ] 5.3 Тест: детерминизм (повторные вызовы mapping дают одинаковый результат для одинаковых входов).

## 6. Validation
- [ ] 6.1 `openspec validate update-flow-sensitive-cfg-stability --strict --no-interactive`.
- [ ] 6.2 `cargo test --workspace` (после реализации; до архивации change).

