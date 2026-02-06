## 1. Proposal / Design
- [ ] 1.1 Зафиксировать алгоритм лексической видимости локальных символов для non-member completion (scope chain, позиция объявления, затенение).
- [ ] 1.2 Зафиксировать правила источников кандидатов: local из v2 snapshot; module/global/meta/keywords из существующих индексов.

## 2. Completion Pipeline (runtime)
- [ ] 2.1 Выделить helper определения текущей позиции в терминах scope/byte offset (переиспользуемый для completion).
- [ ] 2.2 Реализовать сбор локальных кандидатов из IR (`VariableDeclaration`, параметры, loop vars, implicit assignment locals) с фильтрацией по лексической области и позиции.
- [ ] 2.3 Интегрировать сборщик в non-member completion pipeline до стадий ранжирования/дедупликации.
- [ ] 2.4 Убедиться, что member-access ветка completion не меняет поведение.

## 3. Correctness Rules
- [ ] 3.1 Реализовать исключение символов, объявленных после курсора.
- [ ] 3.2 Реализовать корректное затенение имён (nearest scope + latest visible declaration wins).
- [ ] 3.3 Исключить ложные локалы из assignment target, если target не `Identifier` (например, `obj.field = ...`).

## 4. Tests
- [ ] 4.1 Unit-тесты на видимость локалов в блоках `Если/Иначе/Цикл`.
- [ ] 4.2 Тесты на позиционную видимость (до/после объявления).
- [ ] 4.3 Тесты на затенение имён между вложенными блоками.
- [ ] 4.4 Интеграционный/LSP тест: локалы из другой процедуры не попадают в completion.

## 5. Validation
- [ ] 5.1 `openspec validate refactor-v2-lexical-local-completion-scope --strict --no-interactive`.
- [ ] 5.2 Прогон таргетных completion-тестов после реализации.
