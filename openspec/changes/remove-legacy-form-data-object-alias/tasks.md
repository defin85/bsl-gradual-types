## 1. Design
- [ ] 1.1 Спроектировать единый `ImplicitBindingResolver` для всех модульных контекстов (`FormModule`/`ManagerModule`/`ObjectModule`/`RecordSetModule`).
- [ ] 1.2 Зафиксировать матрицу `ModuleType x Symbol` для `Объект`, `ЭтотОбъект`, `ЭтаФорма`, `Форма`, `Элементы`, `Параметры`.
- [ ] 1.3 Зафиксировать модель form-data для `FormModule.Объект` без legacy алиаса `ДанныеФормыОбъект.*`.

## 2. Implementation
- [ ] 2.1 Перевести seed implicit-symbols в v2 на контекстную модель (без string-based legacy алиасов для form-object).
- [ ] 2.2 Добавить/обновить member-resolution для form-data контекста с гарантированными applied-object членами (`Ссылка` и др. по правилам платформы).
- [ ] 2.3 Удалить генерацию и использование legacy `ДанныеФормыОбъект.*` в v2 пути (runtime loader + inference + lookup).
- [ ] 2.4 Обновить formatter outputs (diagnostics/hover/completion/type-at-position), чтобы legacy-имя не попадало в пользовательскую выдачу.

## 3. Validation
- [ ] 3.1 Добавить интеграционные тесты на `Объект` в `FormModule`/`ManagerModule`/`ObjectModule`/`RecordSetModule`.
- [ ] 3.2 Добавить регрессии на `Объект.Ссылка` в формах документов (без `NonExistentProperty`).
- [ ] 3.3 Добавить тест, гарантирующий отсутствие `ДанныеФормыОбъект.*` в diagnostics/hover/completion.
- [ ] 3.4 `openspec validate remove-legacy-form-data-object-alias --strict --no-interactive`.
