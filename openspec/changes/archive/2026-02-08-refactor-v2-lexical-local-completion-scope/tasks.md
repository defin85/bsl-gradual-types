## 1. Proposal / Design
- [x] 1.1 Зафиксировать алгоритм лексической видимости локальных символов для non-member completion (scope chain, позиция объявления, затенение).
- [x] 1.2 Зафиксировать правила источников кандидатов: local из v2 snapshot; module/global/meta/keywords из существующих индексов.

## 2. Completion Pipeline (runtime)
- [x] 2.1 Выделить helper определения текущей позиции в терминах scope/byte offset (переиспользуемый для completion).
- [x] 2.2 Реализовать сбор локальных кандидатов из IR (`VariableDeclaration`, параметры, loop vars, implicit assignment locals) с фильтрацией по лексической области и позиции.
- [x] 2.3 Интегрировать сборщик в non-member completion pipeline до стадий ранжирования/дедупликации.
- [x] 2.4 Убедиться, что member-access ветка completion не меняет поведение.

## 3. Correctness Rules
- [x] 3.1 Реализовать исключение символов, объявленных после курсора.
- [x] 3.2 Реализовать корректное затенение имён (nearest scope + latest visible declaration wins).
- [x] 3.3 Исключить ложные локалы из assignment target, если target не `Identifier` (например, `obj.field = ...`).

## 4. Tests
- [x] 4.1 Unit-тесты на видимость локалов в блоках `Если/Иначе/Цикл`.
- [x] 4.2 Тесты на позиционную видимость (до/после объявления).
- [x] 4.3 Тесты на затенение имён между вложенными блоками.
- [x] 4.4 Интеграционный/LSP тест: локалы из другой процедуры не попадают в completion.

## 5. Validation
- [x] 5.1 `openspec validate refactor-v2-lexical-local-completion-scope --strict --no-interactive`.
- [x] 5.2 Прогон таргетных completion-тестов после реализации.
