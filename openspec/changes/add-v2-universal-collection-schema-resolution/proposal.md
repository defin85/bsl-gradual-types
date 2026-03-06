# Change: Unified schema resolution for universal value collections in IntelliSense v2

## Why
Три отдельных change (`add-v2-map-index-value-resolution`, `add-v2-structure-field-schema-resolution`, `add-v2-valuetable-column-resolution`) описывают один и тот же архитектурный шов:
- snapshot-local эффекты над экземплярами universal value collections;
- единый source of truth для `completion`, `hover`, `type-at-position`, `semantic diagnostics`;
- согласованную strict/safe-degrade политику для member/index access.

Если оставить их как три независимых change, высок риск получить конфликтующие overlay-модели, разный contract по merge/alias и расхождение между IDE consumers.

Нужен один change с единым архитектурным контрактом.

## What Changes
- Добавить единый контракт snapshot-local schema/effect resolution для universal value collections в `bsl-intellisense-v2`.
- Зафиксировать flow-sensitive index value resolution для `Соответствие`:
  - источники effect: `Новый Соответствие`, `Вставить`, `Установить`;
  - policy: literal-key specialization -> generic value type `V` -> `Произвольный`.
- Зафиксировать flow-sensitive field schema resolution для `Структура`:
  - источники effect: поддерживаемые паттерны `Новый Структура(...)`, `Вставить("Имя", Значение)`;
  - known field -> property access;
  - unknown field typed-structure -> hard-fail diagnostics.
- Зафиксировать flow-sensitive column schema resolution для `ТаблицаЗначений`:
  - источник effect: `ТЗ.Колонки.Добавить("Имя", ОписаниеТипов?)`;
  - typed-row для `ТЗ.Добавить()` и `Для каждого Стр Из ТЗ`;
  - unknown column typed-row -> hard-fail diagnostics.
- Зафиксировать единый consumer contract:
  - `completion`, `hover`, `type-at-position`, `semantic diagnostics` используют один и тот же resolved owner/type contract в рамках одного snapshot.
- Зафиксировать архитектурный запрет на synthetic global repository types для этой задачи:
  - state MUST оставаться snapshot-local;
  - глобальный `TypeRepository` MUST NOT мутироваться ради per-instance schema.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2/src/type_inference_v2.rs`
  - `analysis-v2/src/lib.rs`
  - `analysis-v2/src/lib/snapshots.rs`
  - `semantic-diagnostics/src/type_hints.rs`
  - `semantic-diagnostics/src/visitor.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`
  - `bsl-runtime/src/application/type_system/services/hover_service.rs`
  - `shared/src/domain/metadata_lookup/*`
  - `shared/src/domain/validators/type_validator.rs`
  - `bsl-types/src/types/*`

## Supersedes
- `add-v2-map-index-value-resolution`
- `add-v2-structure-field-schema-resolution`
- `add-v2-valuetable-column-resolution`

С этого момента три перечисленных change считаются superseded и не должны использоваться как отдельные источники требований.

## Non-Goals
- Полная интерпроцедурная и межмодульная передача schema/effect для всех коллекций.
- Поддержка динамических имён ключей/полей/колонок без статических опор.
- Жёсткая диагностика отсутствия ключа для динамических `Соответствие[Expr]`.
- Временный compatibility path через synthetic types в глобальном repository.
