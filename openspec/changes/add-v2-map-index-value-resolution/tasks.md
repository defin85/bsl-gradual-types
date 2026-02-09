## 1. Specification
- [ ] 1.1 Добавить spec delta в `bsl-intellisense-v2` для flow-sensitive index access `Соответствие`.
- [ ] 1.2 Зафиксировать контракт map-effect для `Вставить/Установить`.
- [ ] 1.3 Зафиксировать поведение `map[key]` с приоритетом literal-key type -> generic `V` -> `Произвольный`.
- [ ] 1.4 Зафиксировать user-facing поведение для completion/hover/type-at-position после index access.

## 2. Design
- [ ] 2.1 Описать snapshot-local модель map value resolution (общий `V` + literal-key specializations).
- [ ] 2.2 Описать merge policy при конфликтах типов и ветвлениях.
- [ ] 2.3 Описать интеграцию с `Expression::IndexAccess` в v2 inference и с completion owner resolution.

## 3. Validation
- [ ] 3.1 `openspec validate add-v2-map-index-value-resolution --strict`
- [ ] 3.2 Review change с владельцами analysis-v2 и completion/diagnostics.
