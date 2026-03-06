## Context
Паттерн со структурами в 1С:

```bsl
S = Новый Структура;
S.Вставить("Идентификатор", "A-01");
S.Вставить("Количество", 10);
Возврат S;
```

и далее:

```bsl
X = ПолучитьДанные();
Сообщить(X.Идентификатор);
```

Сейчас v2 не фиксирует поле-схему экземпляра `S` как flow-sensitive состояние, поэтому member access `X.Идентификатор` часто не получает устойчивый контракт.

## Goals / Non-Goals
- Goals:
  - Отслеживать схему полей конкретного экземпляра `Структура` в рамках v2 snapshot.
  - Резолвить `s.<поле>` через единый pipeline для completion/hover/diagnostics.
  - В strict режиме диагностировать неизвестные поля typed-structure.
  - Сохранять предсказуемую деградацию по типу поля (`Произвольный`).
- Non-Goals:
  - Полная интерпроцедурная передача схемы между произвольными файлами.
  - Полное покрытие динамических имён полей и runtime reflection.

## Architecture Drivers
- Единый source of truth в v2 type snapshot.
- Минимизация расхождений между completion и diagnostics.
- Низкий риск регрессий и контролируемая деградация при неполной статике.

## Options Considered

### Option A: Synthetic platform types для каждой структуры
- Плюсы: reuse `metadata_lookup` без расширений.
- Минусы: сложный lifecycle synthetic-типа, риск утечек между snapshot.

### Option B (Recommended): Snapshot-local `StructureSchema` overlay
- Плюсы: изоляция по snapshot, ясный dataflow, проще контролировать merge policy.
- Минусы: нужно расширить часть contracts для owner resolution.

### Option C: Completion-only эвристики по AST
- Плюсы: быстрый локальный эффект.
- Минусы: нет единообразия с hover/diagnostics.

## Decisions
- Decision: использовать Option B.
  - Why: согласуется с текущим направлением v2 flow-sensitive inference.

- Decision: ввести snapshot-local модель `StructureSchema`:
  - `field_name` (case-insensitive lookup + canonical label),
  - `field_type` (`TypeResolution` или concrete type string),
  - `source_span`.

- Decision: источники schema-effect:
  - `Новый Структура(...)` в поддерживаемых статических паттернах,
  - `s.Вставить("Имя", Значение)`.

- Decision: unknown field у typed-structure -> hard-fail diagnostics.
  - Why: для структуры это почти всегда опечатка/ошибка контракта.

- Decision: если тип значения поля не вычислен -> `Произвольный`, но поле сохраняется.
  - Why: сохраняем полезность completion и минимизируем FP.

## Implementation Outline
1. `analysis-v2`:
   - добавить сбор structure schema-effects;
   - поддержать базовые merge/alias сценарии внутри snapshot;
   - при `s.<поле>` выдавать typed structure resolution.
2. `shared`:
   - расширить `TypeMetadataLookup.get_properties(...)` для typed-structure полей.
3. `semantic-diagnostics`:
   - использовать `member_access_object_type_by_span` и общий property validation path.
4. `completion_service`:
   - резолв owner из v2 snapshot и показывать поля typed-structure.
5. Тесты:
   - unit + integration для completion/hover/diagnostics.

## Test Strategy
- Unit:
  - `Вставить("Идентификатор", "X")` регистрирует поле и тип `Строка`.
  - `Вставить("Количество", 10)` регистрирует тип `Число`.
- Integration:
  - completion на `S.` показывает поля схемы.
  - hover/type-at-position на `S.Идентификатор` возвращает `Строка`.
  - diagnostics не ругается на известное поле и ругается на неизвестное.
- Regression:
  - существующие кейсы без структуры остаются без изменений.

## Risks / Trade-offs
- Риск: конфликт типов поля в разных ветках.
  - Mitigation: merge policy (union или certainty downgrade).
- Риск: неполная поддержка всех форм `Новый Структура(...)`.
  - Mitigation: вводить покрытие поэтапно, fallback `Произвольный`.
- Риск: расхождения между owner resolution и diagnostics.
  - Mitigation: единый source через type hints v2.
