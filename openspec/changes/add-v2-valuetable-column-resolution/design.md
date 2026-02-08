## Context
Пользовательский кейс:

```bsl
ТЗ = Новый ТаблицаЗначений;
ТЗ.Колонки.Добавить("Идентификатор", ОписаниеТиповСтрока150);
...
Возврат ТЗ;
```

и дальнейшее использование строк таблицы через member access (например, `Строка.Идентификатор`) сейчас не получает стабильного типового контракта в v2 pipeline.

Причина: inference вычисляет типы выражений по месту, но не хранит schema-effect вызовов `Колонки.Добавить(...)` как часть состояния экземпляра таблицы и не доставляет это состояние в completion/diagnostics/hover.

## Goals / Non-Goals
- Goals:
  - Отслеживать схему колонок `ТаблицаЗначений` на уровне конкретного экземпляра в пределах v2 snapshot.
  - Резолвить тип строки таблицы с учетом схемы колонок.
  - Обеспечить одинаковый результат для completion, hover и semantic diagnostics.
  - Сохранить hard-fail для неизвестных колонок typed-row (без silent fallback).
- Non-Goals:
  - Полный inter-file dataflow для всех таблиц значений.
  - Поддержка динамических имен колонок, неизвестных на этапе анализа.

## Architecture Drivers
- Корректность IDE-фич (не только отсутствие FP, но и полезные completion candidates).
- Единый источник истины (v2 type index + hints), без дублирующей логики по слоям.
- Предсказуемая деградация, если тип колонки определить нельзя.
- Низкий риск регрессий производительности в completion path.

## Options Considered

### Option A: Синтетические типы в репозитории типов
- Идея: материализовывать per-table synthetic `RawTypeData` и подключать их через `TypeRepository`.
- Плюсы: reuse существующих metadata_lookup/validator APIs.
- Минусы: сложная lifecycle-модель (snapshot invalidation, GC synthetic типов), высокий риск утечек состояния между файлами.

### Option B (Recommended): Snapshot-local schema overlay + typed row model
- Идея: хранить schema-effect в v2 inference состоянии (snapshot-local), выдавать для строк таблицы специализированный resolution, который metadata_lookup умеет раскрывать в свойства.
- Плюсы: изоляция по snapshot, минимальная мутация глобального репозитория, детерминированный dataflow.
- Минусы: нужно расширить контракты type hints/completion для доставки overlay.

### Option C: Completion-only эвристика
- Идея: парсить `Колонки.Добавить` только в completion_service.
- Плюсы: быстрый локальный фикс.
- Минусы: ломает единый source of truth, не закрывает diagnostics/hover/type-at-position, высокий риск расхождений.

## Decisions
- Decision: Принять Option B.
  - Why: архитектурно согласуется с v2 snapshot model и покрывает все IDE consumers.

- Decision: Ввести snapshot-local `ValueTableSchema` (имя колонки + тип значения) как effect inference.
  - Why: side-effect `Колонки.Добавить` должен моделироваться как изменение состояния экземпляра таблицы, а не только как return type вызова.

- Decision: Для типа строки таблицы использовать специализированный typed-row resolution, из которого `metadata_lookup.get_properties(...)` возвращает список колонок.
  - Why: это позволяет reuse существующей валидации свойства и completion через стандартный pipeline.

- Decision: Тип колонки извлекать из `ОписаниеТипов` best-effort:
  - string literal type name (например, `"Строка"`),
  - простые идентификаторы, указывающие на ранее вычисленный `ОписаниеТипов`,
  - известные квалификаторы `*.StringType` => `Строка`.
  - При невозможности извлечения типа: `Произвольный`.
  - Why: в реальном коде часто используются промежуточные переменные `ОписаниеТипов*`.

- Decision: Для typed-row неизвестная колонка -> hard-fail diagnostics.
  - Why: это сигнал реальной ошибки в коде/опечатки и соответствует пользовательскому требованию строгого режима.

## Implementation Outline
1. `analysis-v2`:
   - Добавить сбор и применение schema-effects для паттерна `ТЗ.Колонки.Добавить(...)`.
   - Пропагировать schema при простых alias-присваиваниях и возвратах локальных функций (в пределах snapshot).
   - На `ТЗ.Добавить()` / `Для каждого Стр Из ТЗ` выдавать typed-row resolution со схемой колонок.
2. `shared`:
   - Расширить `TypeMetadataLookup.get_properties(...)` поддержкой typed-row schema.
3. `semantic-diagnostics`:
   - Использовать тот же typed-row resolution через `member_access_object_type_by_span` (без отдельного эвристического пути).
4. `completion_service`:
   - Приоритетно использовать v2 типовую информацию snapshot (включая typed-row) для owner resolution.
5. Тесты:
   - inference, completion, hover, semantic diagnostics на сценариях с `Колонки.Добавить`.

## Test Strategy
- Unit (analysis-v2):
  - `ТЗ.Колонки.Добавить("Идентификатор", ...)` регистрирует колонку.
  - `ТЗ.Добавить().Идентификатор` имеет корректный тип.
- Integration (backend/bsl-runtime):
  - completion на `Стр.` предлагает все добавленные колонки.
  - hover/type-at-position на `Стр.Идентификатор` возвращает ожидаемый тип.
  - diagnostics не ругается на известную колонку и ругается на несуществующую.
- Regression:
  - существующие сценарии `ТаблицаЗначений.Колонки` и `Колонки.Добавить().Имя` остаются рабочими.

## Risks / Trade-offs
- Риск: ложные колонка-схемы при переиспользовании переменных в разных ветках.
  - Mitigation: scope-aware merge policy (пересечение/union с certainty downgrade).
- Риск: неполный разбор `ОписаниеТипов` в сложных выражениях.
  - Mitigation: безопасный fallback к `Произвольный`, но имя колонки сохраняется.
- Риск: completion path использует эвристики без полного type_index.
  - Mitigation: добавить передачу/использование v2 type snapshot в `CompletionAnalysisContext`.

## External References
- 1C KB: Values and types (TypeDescription, ValueTable columns):
  - https://kb.1ci.com/1C_Enterprise_Platform/FAQ/Development/Standards/Values_and_types/
- 1C DN: Procedure/function descriptions (описание колонок `ValueTable` в контракте):
  - https://1c-dn.com/library/procedure_and_function_descriptions/
- TypeScript narrowing/control-flow (референс для flow-sensitive модели):
  - https://www.typescriptlang.org/docs/handbook/2/narrowing.html
  - https://www.typescriptlang.org/play/4-4/new-js-features/control-flow-improvements.ts.html

## Context7 Notes (1C)
- `/websites/1c-dn-library` не резолвится в текущем Context7 окружении.
- `/shootnick-tm/ssl_3_1` доступен, но релевантных сниппетов именно по `ТаблицаЗначений.Колонки.Добавить` не предоставляет.
