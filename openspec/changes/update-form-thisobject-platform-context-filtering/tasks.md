## 1. Specification
- [ ] 1.1 Обновить requirement для `FormModule` implicit symbols: зафиксировать `ЭтотОбъект/ЭтаФорма/Форма` как runtime form context (`ФормаКлиентскогоПриложения` + extension + shape).
- [ ] 1.2 Обновить requirement для member completion implicit symbols: добавить обязательную контекстную фильтрацию доступности.
- [ ] 1.3 Добавить новое requirement про context/facet-aware доступность members для `completion/hover/diagnostics/type-at-position`.

## 2. Implementation
- [ ] 2.1 Расширить metadata/lookup слой для хранения и чтения availability не только у методов, но и у свойств.
- [ ] 2.2 Реализовать provider-chain для `FormModule.ЭтотОбъект/ЭтаФорма/Форма` с обязательным platform base + extension + shape.
- [ ] 2.3 Внедрить единый context-aware member filtering API в `TypeMetadataLookup` и убрать локальные divergent-фильтры в consumer-ах.
- [ ] 2.4 Обновить `completion`, `hover`, `diagnostics`, `type-at-position` на единый API с Unknown-context fallback.

## 3. Validation
- [ ] 3.1 Добавить/обновить тесты на проброс members `ФормаКлиентскогоПриложения` в `ЭтотОбъект`.
- [ ] 3.2 Добавить/обновить тесты матрицы доступности по директивам (`&НаКлиенте`, `&НаСервере`, `&НаСервереБезКонтекста`, `&НаКлиентеНаСервереБезКонтекста`).
- [ ] 3.3 Добавить/обновить тесты изоляции `FormModule.Объект` (без протечки form-runtime members).
- [ ] 3.4 Добавить/обновить cross-consumer parity тесты (`completion/hover/diagnostics/type-at-position`) для одного snapshot.
- [ ] 3.5 Прогнать quality gates (`cargo test`, профильные тестовые наборы) и зафиксировать отсутствие регрессий.

