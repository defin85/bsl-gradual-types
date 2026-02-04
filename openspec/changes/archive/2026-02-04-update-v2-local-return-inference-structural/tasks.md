## 1. Proposal / Specs
- [x] 1.1 Добавить delta-спеку для `bsl-intellisense-v2`: структурное распространение return-типа по локальным вызовам + рекурсия.
- [x] 1.2 `openspec validate update-v2-local-return-inference-structural --strict --no-interactive`.

## 2. Refactor: структурный return solver (analysis-v2)
- [x] 2.1 Убрать хранение return-типа как `String` в local solver; перейти на структурное представление (`TypeResolution` / варианты union как `ConcreteType`).
- [x] 2.2 Ввести явную `join`/merge policy для return-типа: flatten union, дедуп, детерминированный порядок, soundness для implicit `Неопределено`.
- [x] 2.3 Убедиться, что `A(){ return B(); }` переносит тип `B` без деградации union в platform-строку.

## 3. Tests (структурные)
- [x] 3.1 Юнит‑тест: транзитивное распространение union (`A` возвращает `B`, `B` возвращает разные типы) проверяет структуру `ResolutionResult::Union`, а не только `type_name()`.
- [x] 3.2 Юнит‑тест: mutual recursion `A()<->B()` проверяет детерминизм результата и завершение solver’а (без лимитов).
- [x] 3.3 Интеграционный тест на `examples/conf_big` не добавлялся: риск закрыт детерминированными юнит‑тестами (3.1/3.2).

## 4. Quality gates
- [x] 4.1 `cargo test -p bsl-analysis-v2`
- [x] 4.2 `cargo test -p bsl-runtime` (smoke)
