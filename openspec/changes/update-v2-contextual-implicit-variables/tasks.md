## 1. Specification
- [ ] 1.1 Добавить spec delta в `bsl-intellisense-v2` для context-aware implicit-переменных модулей.
- [ ] 1.2 Зафиксировать матрицу биндингов для `FormModule`, `ManagerModule`, `ObjectModule`, `RecordSetModule`.
- [ ] 1.3 Зафиксировать policy для `Параметры` в `FormModule`: тип `Структура`.
- [ ] 1.4 Зафиксировать поведение `*БезКонтекста`: context-bound переменные формы недоступны.
- [ ] 1.5 Зафиксировать запрет на FP-диагностики `Необъявленная переменная` для корректных implicit-переменных в валидном контексте.

## 2. Design
- [ ] 2.1 Описать единый источник правил implicit-биндингов, переиспользуемый AST→IR и type inference.
- [ ] 2.2 Описать fallback-политику при неполной загрузке metadata (детерминированно, без ложных `undeclared`).
- [ ] 2.3 Описать влияние на diagnostics/hover/completion и требования к согласованности snapshot.

## 3. Validation
- [ ] 3.1 `openspec validate update-v2-contextual-implicit-variables --strict --no-interactive`.
- [ ] 3.2 Review change с владельцами IntelliSense v2 (диагностики + фасеты + контексты).
