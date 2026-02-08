## Context
В v2 есть фасетная модель для конфигурационных типов (`Manager/Object/Reference/...`), но `FormModule.Объект` сейчас частично опирается на внутренний synthetic алиас `ДанныеФормыОбъект.*`. Это создаёт разрыв между платформенной моделью form data и фактической проверкой members в diagnostics/inference.

## Goals / Non-Goals
- Goals:
  - Устранить legacy `ДанныеФормыОбъект.*` из v2 пути и user-facing выдачи.
  - Ввести единый контекстный резолвер implicit symbols для всех `ModuleType`.
  - Сохранить корректную фасетную семантику в `Object/Manager/RecordSet` модулях.
  - Гарантировать корректный member-resolution для form-object сценариев (минимум `Объект.Ссылка`).
- Non-Goals:
  - Полное моделирование всех platform form-data вариаций за один change.
  - Изменение грамматики BSL/AST.
  - Ретрофит legacy inference pipelines вне v2.

## Architecture
### 1) Contextual implicit binding layer
Добавляется единый слой, который резолвит символ по контексту:
`(ModuleType, owner metadata, compiler directive, symbol) -> semantic type descriptor`.

Ключевые правила:
- `FormModule`:
  - `ЭтотОбъект/ЭтаФорма/Форма` -> тип формы.
  - `Объект` -> form-data descriptor (платформенная модель данных формы), не `ДанныеФормыОбъект.*`.
  - `Элементы` -> контейнер элементов формы.
  - `Параметры` -> `Структура`.
- `ManagerModule`: `ЭтотОбъект/Объект` -> manager facet.
- `ObjectModule`/`RecordSetModule`: `ЭтотОбъект/Объект` -> object/recordset facet.

### 2) Form-data member resolution pipeline
Для `FormModule.Объект` member-resolution выполняется слоями:
1. platform form-data members;
2. гарантированные projected members applied object (например, `Ссылка` для документа);
3. реквизиты формы;
4. привязанные табличные части;
5. controlled fallback (`InferredWeak`) без ложных `NonExistentProperty`.

### 3) Legacy cleanup
- Удаляется генерация/использование `ДанныеФормыОбъект.*` в v2 core path.
- На время миграции допускается адаптер чтения старых данных, но без попадания legacy-имен в diagnostics/hover/completion/type-at-position.

## Data and API considerations
- Внутренний descriptor должен содержать минимум: owner metadata kind/name, module context, form name, source certainty.
- `TypeMetadataLookup` получает расширение для form-data descriptor (отдельный provider), чтобы не смешивать его с чисто фасетным lookup.
- Формат user-facing type labels должен быть нормализован к платформенным наименованиям.

## Rollout plan
1. Добавить descriptor + resolver (без удаления старого кода).
2. Переключить `analysis-v2` seed implicit context на новый resolver.
3. Переключить metadata lookup/member validation для form-data.
4. Удалить legacy alias path + зачистить тесты/доки.

## Validation strategy
- Интеграционные сценарии по модулям: form/manager/object/recordset.
- Негативные сценарии для `*БезКонтекста`.
- Snapshot/golden проверки, что `ДанныеФормыОбъект.*` отсутствует в user-facing outputs.
- Regression: `Объект.Ссылка` в документной форме не даёт `NonExistentProperty`.
