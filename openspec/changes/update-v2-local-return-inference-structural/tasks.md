## 1. Proposal / Specs
- [ ] 1.1 Добавить delta-спеку для `bsl-intellisense-v2`: структурное распространение return-типа по локальным вызовам + рекурсия.
- [ ] 1.2 `openspec validate update-v2-local-return-inference-structural --strict --no-interactive`.

## 2. Refactor: структурный return solver (analysis-v2)
- [ ] 2.1 Убрать хранение return-типа как `String` в local solver; перейти на структурное представление (`TypeResolution` / варианты union как `ConcreteType`).
- [ ] 2.2 Ввести явную `join`/merge policy для return-типа: flatten union, дедуп, детерминированный порядок, soundness для implicit `Неопределено`.
- [ ] 2.3 Убедиться, что `A(){ return B(); }` переносит тип `B` без деградации union в platform-строку.

## 3. Tests (структурные)
- [ ] 3.1 Юнит‑тест: транзитивное распространение union (`A` возвращает `B`, `B` возвращает разные типы) проверяет структуру `ResolutionResult::Union`, а не только `type_name()`.
- [ ] 3.2 Юнит‑тест: mutual recursion `A()<->B()` проверяет детерминизм результата и завершение solver’а (без лимитов).
- [ ] 3.3 При необходимости — интеграционный тест на `examples/conf_big` (не обязателен, если юнит‑тесты закрывают риск).

## 4. Quality gates
- [ ] 4.1 `cargo test -p bsl-analysis-v2`
- [ ] 4.2 `cargo test -p bsl-runtime` (smoke)

