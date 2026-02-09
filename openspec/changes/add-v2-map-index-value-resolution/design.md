## Context
Типичный паттерн:

```bsl
Map = Новый Соответствие;
Map.Вставить("Идентификатор", Новый ОписаниеПоля(...));
X = Map["Идентификатор"];
Сообщить(X.Имя);
```

Сейчас в v2 `IndexAccess` по сути не формирует устойчивый тип в inference path, из-за чего downstream completion/hover/type-at-position теряют owner-type.

## Goals / Non-Goals
- Goals:
  - Резолвить тип значения `map[key]` в рамках snapshot на основе map-effects.
  - Поддержать литеральные ключи и generic value type `V`.
  - Обеспечить единое поведение completion/hover/type-at-position после index access.
  - Сохранять безопасную деградацию без FP для динамических ключей.
- Non-Goals:
  - Доказательство присутствия ключа во всех runtime путях.
  - Hard-fail диагностика для "ключ не найден" при динамических ключах.

## Architecture Drivers
- Практическая полезность IDE при работе с dictionary payload.
- Контролируемый баланс strictness/false positives.
- Переиспользование существующего v2 snapshot и generic plumbing.

## Options Considered

### Option A: Только generic `V` без key-level tracking
- Плюсы: просто.
- Минусы: теряет точность для literal keys с конкретными типами.

### Option B (Recommended): Snapshot-local map overlay
- Идея:
  - хранить общий `V` (тип значения map),
  - плюс optional literal-key specializations: `key_literal -> value_type`.
- Плюсы: высокий signal для популярных паттернов.
- Минусы: нужна merge policy и аккуратная деградация.

### Option C: Completion-only эвристики
- Плюсы: быстрый локальный эффект.
- Минусы: не закрывает hover/type-at-position/diagnostics единообразно.

## Decisions
- Decision: использовать Option B.
  - Why: даёт best effort точность без полного value tracking.

- Decision: приоритет резолюции для `map[key]`:
  1. literal-key specialization (если есть),
  2. generic value type `V`,
  3. `Произвольный` (fallback).

- Decision: для динамического ключа не поднимать hard-fail "ключ не найден".
  - Why: слишком высокий риск FP в 1С-коде.

- Decision: конфликты типов по одному ключу/веткам:
  - union (или certainty downgrade) по policy,
  - с сохранением стабильного user-facing поведения.

## Implementation Outline
1. `analysis-v2`:
   - добавить map-effects для `Вставить/Установить`;
   - обновить `Expression::IndexAccess` для вычисления значения по policy приоритетов.
2. `bsl-runtime completion`:
   - использовать v2 owner resolution для продолжения цепочки после index access.
3. `semantic-diagnostics`:
   - использовать member access type hints после index access (без отдельного эвристического канала).
4. Тесты:
   - unit и integration на `map["k"]` и цепочки `map["k"].`.

## Test Strategy
- Unit:
  - `Вставить("k", Число)` -> `map["k"]` имеет тип `Число`.
  - неизвестный literal-key при известном generic `V` -> тип `V`.
  - при отсутствии обоих -> `Произвольный`.
- Integration:
  - completion на `map["k"].` предлагает свойства типа значения.
  - hover/type-at-position на `map["k"]` возвращает ожидаемый тип.
- Regression:
  - текущие completion tests для map index access остаются зелёными.

## Risks / Trade-offs
- Риск: рост сложности merge policy на ветвлениях.
  - Mitigation: ограничить первую версию простым deterministic policy.
- Риск: ложная уверенность при частичной информации о ключах.
  - Mitigation: certainty downgrade и явный fallback в `Произвольный`.
