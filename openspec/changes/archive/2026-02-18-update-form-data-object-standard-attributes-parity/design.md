## Context
После перехода на strict form-data semantics для `FormModule.Объект` в выдаче остался gap parity:
- canonical type корректный (`ДанныеФормыСтруктура`),
- но часть expected applied-object members отсутствует (в первую очередь standard attributes документа),
- при этом инвариант no form-shape leakage должен быть сохранён.

Текущий pipeline покрывает:
- applied attributes и tabular sections из metadata parser,
- form-shape attributes из `Form.xml` для form context (`ЭтотОбъект`),
- ограниченный intrinsic supplement для form-data (`Ссылка`, `ПометкаУдаления`).

Этого недостаточно для стабильного parity с runtime/debugger по документам.

## Goals / Non-Goals
- Goals:
  - Зафиксировать архитектурно корректный источник истины для parity через metadata pipeline.
  - Обеспечить минимум `Дата`, `Номер`, `Проведен` в `FormModule.Объект` для документов.
  - Добавить projection табличных частей applied-object в form-data members.
  - Сохранить strict инварианты: `ДанныеФормыСтруктура` и отсутствие form-only leakage.
- Non-Goals:
  - Полная эмуляция всего runtime-поведения платформы 1С для всех типов.
  - Пересмотр user-facing label policy за пределами данного контракта parity.
  - Возврат к applied object facet fallback в form-data chain.

## Decision
Выбран **вариант 2**:
- source of truth = `parser -> converter -> repository -> metadata_lookup`;
- parity достигается расширением metadata ingestion и form-data aggregation;
- hardcoded intrinsic слой остаётся минимальным safety-net и не является основным источником parity.

### Alternatives considered
1. Только intrinsic hardcoded members (`Дата/Номер/Проведен`):
   - быстрее локально,
   - но высокие риски drift/ложных positives и слабая масштабируемость.
2. Подмешивание form-shape (`Form.xml`) в `Объект`:
   - просто технически,
   - но нарушает strict контракт и вносит form-only leakage.

## Target Architecture
### 1) Parser stage
- Расширить parser model на извлечение standard attributes applied-object (для document: `Posted`, `Ref`, `DeletionMark`, `Date`, `Number`).
- Нормализовать имена и сохранить данные, достаточные для типизации (например, `NumberType`).

### 2) Converter stage
- При построении `RawTypeData` applied-object включать standard attributes как metadata-derived properties.
- Применять dedup policy:
  - repository-derived members имеют приоритет над intrinsic fallback,
  - no duplicate names (case-insensitive).

### 3) Lookup stage (form-data chain)
- Сохранить provider-chain:
  - `IntrinsicGuaranteed`
  - `RawTypeFallback`
- Обновить `RawTypeFallback`, чтобы он включал:
  - расширенные applied-object properties (включая standard attributes),
  - projection табличных частей applied-object в form-data members.
- Не добавлять form-shape providers в `FormModule.Объект`.

### 4) Context invariants
- `FormModule.Объект` остаётся `ДанныеФормыСтруктура`.
- `ЭтотОбъект` остаётся form-context type и содержит `Объект: ДанныеФормыСтруктура`.
- form-only attributes доступны через form context (`ЭтотОбъект`), но не через `Объект`.

## Mapping Policy
- `Ref` -> `Ссылка` (reference type конкретного документа).
- `DeletionMark` -> `ПометкаУдаления` (`Булево`).
- `Date` -> `Дата` (`Дата`).
- `Number` -> `Номер` (тип зависит от `NumberType`, минимум `String`/`Number`).
- `Posted` -> `Проведен` (`Булево`) только для posting-capable документов.

## Test Strategy
### Unit
- parser/converter tests:
  - извлечение standard attributes,
  - корректная типизация `Number`,
  - dedup/precedence.
- metadata_lookup tests:
  - provider-chain order,
  - projection табличных частей,
  - no form-shape leakage.

### Integration
- `conf_big/Documents/РеализацияТоваровУслуг`:
  - required/minimum positive-set: `Дата`, `Номер`, `Проведен`,
  - tabular members (`Товары`, `Услуги`, ...),
  - negative-set form-only (`ПоказыватьБаннер`, `СсылкаДляПереходаНаКарту`, `Надпись*`),
  - `ЭтотОбъект` form-context invariants.

### Quality gate behavior
- Критичные parity-asserts не должны silently pass при неполном окружении.
- Если окружение неполное, тест обязан явно репортить missing prerequisite.

## Risks / Mitigations
- Risk: неверная типизация `Номер` из-за неполного metadata контекста.
  - Mitigation: явный mapping policy + unit tests для `NumberType` вариантов.
- Risk: рост latency в hover/completion из-за расширенной агрегации members.
  - Mitigation: bounded aggregation, dedup, targeted perf regression check.
- Risk: повторная утечка form-only members.
  - Mitigation: отдельные negative regression tests и invariant checks в provider-chain.

## Migration / Rollback
- Migration:
  - ввод parser/converter поля для standard attributes,
  - включение в lookup chain без изменения canonical form-data type.
- Rollback:
  - отключение расширенного raw projection при сохранении существующего intrinsic minimum (`Ссылка`, `ПометкаУдаления`),
  - сохранение strict контракта `FormModule.Объект` без fallback к object facet.

## Definition of Ready
- Согласован mapping policy standard attributes.
- Согласован required positive-set и negative-set для `РеализацияТоваровУслуг`.
- Определены критерии поведения тестов при отсутствии prereq данных.

## Definition of Done
- `FormModule.Объект` показывает минимум `Дата`, `Номер`, `Проведен` + tabular projection.
- `FormModule.Объект` не содержит form-only attributes.
- `ЭтотОбъект` сохраняет form-context + `Объект: ДанныеФормыСтруктура`.
- Unit/integration quality gates стабильно проходят.
