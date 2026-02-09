## Context
В текущей реализации implicit symbols в v2 задаются как имена + `type_name: Option<String>`, после чего строки пытаются резолвиться в `TypeResolution`.

Это работает для части случаев, но не даёт стабильного архитектурного контракта для:
- сохранения facet context при seed implicit symbols;
- отдельной семантики form-data (`FormModule.Объект`);
- единообразного member-resolution без string-fallback эффекта.

## Goals / Non-Goals
- Goals:
  - Ввести структурную (descriptor-based) модель контекстных implicit типов.
  - Убрать зависимость implicit binding от string-only типа как источника истины.
  - Зафиксировать детерминированный путь member-resolution для form-data.
  - Сохранить текущий UX-контракт: без legacy alias в diagnostics/hover/completion.
- Non-Goals:
  - Полная эмуляция runtime-движка форм 1С.
  - Изменение грамматики/AST.
  - Рефакторинг всего `TypeResolution` вне области implicit/form-data сценариев.

## Decisions

### 1) Ввести контекстные дескрипторы типов для implicit-symbols
Добавляется отдельная модель дескрипторов (рабочее имя: `ContextualTypeDescriptor`) с минимумом полей:
- module context (`ModuleType`/role),
- owner metadata (`kind`, `name`),
- required facet (если есть),
- form context (`form_name`) для `FormModule`.

Ключевая идея: implicit binding возвращает структурное описание, а не финальную строку типа.

### 2) Разделить descriptor-model и user-facing label
Descriptor используется для внутренней семантики и lookup.
Строковые имена формируются отдельно (formatter/UI boundary), чтобы не смешивать transport/UX с семантическим контрактом.

Нормативное правило для `FormModule.Объект` (зафиксировано по платформенной модели):
- canonical semantic type: form-data descriptor (`ДанныеФормыСтруктура` semantics),
- default user-facing label (compact/standard): owner object facet (`ДокументОбъект.X`, `СправочникОбъект.X`, ...),
- detailed user-facing representation: owner object facet + явная пометка form-data (например, `ДокументОбъект.X (данные формы: ДанныеФормыСтруктура)` или эквивалент).

### 3) Descriptor -> TypeResolution с явным facet контрактом
Конвертация descriptor в `TypeResolution` должна:
- задавать `ConcreteType::Configuration` там, где это configuration facet;
- выставлять `active_facet` детерминированно;
- деградировать в `InferredWeak` только в явно разрешённых местах (например, нет метаданных), без ложных `Unknown`/FP.

### 4) Form-data member-resolution как отдельный provider chain
Для `FormModule.Объект` вводится отдельный descriptor-aware провайдер:
1. form shape (атрибуты/ТЧ формы);
2. guaranteed applied-object members (`Ссылка` и т.п. по правилам платформы);
3. applied facet lookup fallback.

Это снимает зависимость от synthetic string alias как механизма получения members.

### 5) Совместимость
`ДанныеФормыОбъект.*` остаётся только как migration compatibility alias на входе/нормализации. Внутренний pipeline и user-facing output используют descriptor/model без legacy имени.

## Alternatives Considered
- Оставить string-based модель и расширять набор эвристик.
  - Отклонено: растёт связность и вероятность regressions в разных этапах pipeline.
- Полностью перевести все типы v2 на новую модель за один change.
  - Отклонено: слишком широкий scope и высокий риск для поставки.

## Risks / Trade-offs
- Рост сложности модели и числа преобразований.
  - Митигация: узкий scope (только implicit/form-data path), строгие invariants и тест-контракт.
- Возможные регрессии completion/hover из-за смены внутреннего контракта.
  - Митигация: интеграционные golden/regression тесты на текущие кейсы.
- Риск рассинхронизации internal canonical и user-facing labels.
  - Митигация: отдельные инварианты и тесты для semantic layer и formatter layer.

## Migration Plan
1. Добавить descriptor model и конвертер в `TypeResolution`.
2. Перевести `ImplicitBindingResolver` и seed paths (`AST->IR`, `type_inference_v2`).
3. Добавить descriptor-aware form-data provider в `TypeMetadataLookup`.
4. Обновить formatter и тесты/регрессии, зафиксировав:
   - canonical form-data semantics для `FormModule.Объект`,
   - user-facing owner facet label,
   - отсутствие legacy alias в output.
