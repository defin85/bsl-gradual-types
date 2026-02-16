## 1. Specification
- [ ] 1.1 Обновить требования `bsl-intellisense-v2` под strict runtime semantics для `FormModule.Объект` и `ЭтотОбъект`.
- [ ] 1.2 Зафиксировать, что change является breaking и не допускает safe migration режима.
- [ ] 1.3 Удалить из delta-контракта допущения dual-layer (`shape + intrinsic + facet`) для `FormModule.Объект`.

## 2. Design
- [ ] 2.1 Описать целевую модель резолва `FormModule.ЭтотОбъект/ЭтаФорма/Форма` через `ClientApplicationForm + extension(main attribute) + form attributes`.
- [ ] 2.2 Описать целевую модель резолва `FormModule.Объект` как strict `ДанныеФормыСтруктура` без implicit facet fallback.
- [ ] 2.3 Зафиксировать единый source-of-truth между hover/completion/type-at-position/diagnostics.
- [ ] 2.4 Зафиксировать исключение feature flags и compatibility fallback из архитектуры.

## 3. Implementation (follow-up)
- [ ] 3.1 Переписать owner-resolution и member-resolution для `FormModule.Объект` под strict form-data semantics.
- [ ] 3.2 Убрать в `TypeMetadataLookup` provider chain, который подмешивает applied object facet для `FormModule.Объект`.
- [ ] 3.3 Обновить formatter/type-label policy для user-facing вывода (`hover`, diagnostics messages).
- [ ] 3.4 Удалить/переписать тесты dual-layer контракта и добавить runtime-совместимые regression tests по `Объект.txt` и `ЭтотОбъект.txt`.
- [ ] 3.5 Привести completion/hover сортировку и состав members к новой модели без скрытых fallback.
- [ ] 3.6 Добавить guard-тест: applied-module owner fallback (из `update-applied-module-self-scope-and-predefined-members`) не активируется для `FormModule`.

## 4. Validation
- [ ] 4.1 `openspec validate remove-form-module-dual-layer-semantics --strict --no-interactive`
- [ ] 4.2 Прогнать contract/e2e тесты для form module implicit symbols.
- [ ] 4.3 Подтвердить отсутствие compatibility switch и fallback-path в новой реализации.
- [ ] 4.4 Подтвердить совместимость с `update-applied-module-self-scope-and-predefined-members`: `FormModule.Объект` остаётся strict form-data.
