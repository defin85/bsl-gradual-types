## 1. Specification
- [ ] 1.1 Добавить spec delta в `bsl-intellisense-v2` для резолва колонок `ТаблицаЗначений`.
- [ ] 1.2 Зафиксировать контракт schema-effect для `ТЗ.Колонки.Добавить("Имя", ТипКолонки?)`.
- [ ] 1.3 Зафиксировать поведение typed-row (`ТЗ.Добавить()`, `Для каждого Стр Из ТЗ`) и доступа `Стр.<Колонка>`.
- [ ] 1.4 Зафиксировать hard-fail диагностику для неизвестной колонки typed-row.

## 2. Design
- [ ] 2.1 Описать snapshot-local модель хранения `ValueTableSchema` и точки интеграции в v2 inference.
- [ ] 2.2 Описать стратегию извлечения типа из `ОписаниеТипов` (best-effort + fallback `Произвольный`).
- [ ] 2.3 Описать унификацию completion/hover/diagnostics через единый typed-row resolution.

## 3. Validation
- [ ] 3.1 `openspec validate add-v2-valuetable-column-resolution --strict`
- [ ] 3.2 Review change с владельцами IntelliSense v2 (analysis + diagnostics + completion).
