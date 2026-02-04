# Design: Структурный local return inference в v2 (без stringly-типов)

## Problem Statement
Текущий local return inference в `analysis-v2/src/type_inference_v2.rs` хранит промежуточные варианты return-типа как `String` (`TypeResolution::type_name()`), а затем восстанавливает `TypeResolution` через `string_to_concrete`.

Проблема: `type_name()` — это форматирование для UI, а не сериализация типовой структуры.
В частности:
- если локальная функция `B()` уже имеет `ResolutionResult::Union`, её `type_name()` содержит `|`;
- при `A() { return B(); }` текущий solver добавляет строку `"Строка | Число"` в множество типов `A`;
- затем эта строка конвертируется в `ConcreteType::Platform { name: "Строка | Число" }`,
  т.е. **union теряет структуру**, хотя внешне (в строке) может выглядеть “нормально”.

## Goals
- Возвращаемый тип локальных функций должен быть представлен структурно (`TypeResolution`), без кодирования union/generic в строку.
- Транзитивные вызовы (`A` возвращает `B`) должны сохранять структуру return-типа `B`.
- Алгоритм остаётся sound и детерминированным при SCC/рекурсии.
- Тесты проверяют структуру `TypeResolution`, а не только `type_name()`.

## Proposed Approach
### 1) Заменить stringly состояние на структурное
Внутреннее состояние solver’а хранит не `BTreeSet<String>`, а структурную форму, например:
- `Vec<TypeResolution>` или
- специализированную “решётку” return-типа (join-semilattice), которая умеет:
  - flatten union,
  - дедуплицировать варианты,
  - обеспечивать детерминированный порядок,
  - возвращать `TypeResolution` итогом.

Минимально: хранить `BTreeMap<StableKey, ConcreteType>` для вариантов union, где `StableKey` — детерминированный ключ
(например, `TypeResolution::known(ct).type_name()`), а для “сложных” типов (generic/nullable/intersection) иметь явную policy.

### 2) Join/merge policy (детерминированно)
Определить `join_return(a, b) -> TypeResolution`:
- если `a` или `b` — `Unknown/Dynamic` (по правилам v2) → результат `Unknown/Dynamic` (sound, “верх решётки”);
- если `a` или `b` — union → flatten: объединить варианты union (без превращения union в строку);
- если `a` и `b` — разные конкретные типы → сформировать union (структурно) с детерминированной нормализацией;
- для implicit return добавлять `Неопределено` как отдельный вариант (согласно текущей типовой модели проекта).

### 3) Solver остаётся SCC + fixed-point
Сохранить:
- построение call graph по AST только для локальных `F()` вызовов;
- Tarjan SCC;
- fixed-point внутри SCC без “магических” лимитов.

Замена касается только представления типов и операции merge/join, чтобы SCC/fixed-point работали на структурных значениях.

## Test Strategy
Добавить/усилить тесты:
- Юнит‑тест: `B()` возвращает union, `A()` возвращает `B()` → тип вызова `A()` структурно union.
- Юнит‑тест: mutual recursion `A()<->B()` → детерминированный результат по policy (например `Unknown`) и без зависаний.
- (Опционально) интеграционный тест на реальном `examples/conf_big`, если это даёт дополнительную защиту от регрессий.

## Out of Scope / Deferred
- Изменение `bsl-types` для разведения “Dynamic” и “Неопределено” как значения в union-структуре (если потребуется — отдельный change).

