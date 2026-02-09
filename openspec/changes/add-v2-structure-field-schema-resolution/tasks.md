## 1. Specification
- [ ] 1.1 Добавить spec delta в `bsl-intellisense-v2` для flow-sensitive field schema `Структура`.
- [ ] 1.2 Зафиксировать контракт schema-effect для `Новый Структура(...)` и `Вставить("ИмяПоля", Значение)`.
- [ ] 1.3 Зафиксировать user-facing поведение для `completion/hover/type-at-position` на `s.<поле>`.
- [ ] 1.4 Зафиксировать hard-fail диагностику для неизвестного поля typed-structure.

## 2. Design
- [ ] 2.1 Описать snapshot-local модель `StructureSchema` и точки интеграции в v2 inference.
- [ ] 2.2 Описать стратегию вывода типа поля из выражения значения (best-effort + fallback `Произвольный`).
- [ ] 2.3 Описать унификацию completion/hover/diagnostics через единый owner resolution.

## 3. Validation
- [ ] 3.1 `openspec validate add-v2-structure-field-schema-resolution --strict`
- [ ] 3.2 Review change с владельцами analysis-v2 и diagnostics/completion.
