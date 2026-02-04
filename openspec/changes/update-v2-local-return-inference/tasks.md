## 1. Proposal / Specs
- [x] 1.1 Добавить delta-спеку для `bsl-intellisense-v2` (локальный return inference: soundness + union + SCC).
- [x] 1.2 `openspec validate update-v2-local-return-inference --strict --no-interactive`.

## 2. Local summaries (analysis-v2)
- [x] 2.1 Выделить отдельную структуру “local function summaries” (return_type + fallthrough) и сделать её доступной для type inference.
- [x] 2.2 Реализовать определение `may_fallthrough` (CFG-based либо sound AST-аппроксимация) и покрыть тестами.

## 3. Call graph + SCC solver
- [x] 3.1 Построить локальный call graph по AST (только `FunctionDecl`, только короткие вызовы `F()`).
- [x] 3.2 Реализовать SCC + worklist solver без магических лимитов и задокументировать policy для рекурсии/Unknown.

## 4. Union return types
- [x] 4.1 Реализовать union типов возвратов по нескольким `Возврат` (включая `Возврат;` → `Неопределено`).
- [x] 4.2 Гарантировать, что `implicit return` добавляет `Неопределено`, если возможен выход без `Возврат`.

## 5. Integration + Regression tests
- [x] 5.1 Интегрировать приоритет local summaries для `Call(Identifier)` в type inference v2.
- [x] 5.2 Добавить регрессионные тесты на примере common module:
  - `КакаяТоСтрока = ФункцияКотораяВозвращаетСтроку();` → `Строка`
  - fallback на “необъявленную” функцию остаётся `Unknown`/diagnostic как раньше
- [x] 5.3 Добавить тесты на union/fallthrough/рекурсию.

## 6. Quality gates
- [x] 6.1 `cargo test -p bsl-analysis-v2`
- [x] 6.2 `cargo test -p bsl-runtime` (smoke: hover/type-at-position не регрессировал на type index)
