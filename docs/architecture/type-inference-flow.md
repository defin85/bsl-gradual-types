# Type Inference Flow (аудит R6)

Цель документа — зафиксировать **реальный** поток вывода/резолвинга типов и точки, где мы «теряем» информацию, чтобы можно было безопасно менять ключи типов (TypeId) и логику резолвинга без регрессий.

Связанный документ с более широкой схемой контрольных точек: `docs/architecture/type_resolution_workflow.md`.

## Область и термины

- **Type inference** — вывод `TypeResolution` для AST-выражений при конвертации `AST → IR`.
- **Type resolution** — резолв строки типов (включая фасеты/дженерики/union) в доменном `TypeResolver`.
- **Documentation lookup** — получение методов/свойств/табличных частей из `TypeMetadataLookup` по `TypeResolution`.

## Высокоуровневый поток

```mermaid
flowchart TD
  AST[AST Expression] -->|AstToIrConverter| TI[infer_type_resolution()]
  TI -->|SymbolTable / эвристики| TRS[TypeResolution]

  TRS -->|при необходимости| TR[TypeResolver.resolve_expression_sync()]
  TR -->|TypeRepository.find_type()| Repo[InMemoryTypeRepository + TypeId]
  TR -->|MemberResolver/Strategies| TRS

  TRS -->|hover/diagnostics| TML[TypeMetadataLookup.get_methods/get_properties]
  TML -->|SignatureIndex| SI[SignatureIndex (TypeId keys)]
  TML -->|RawTypeData fallback| Repo
```

## Детализация: AST → TypeResolution (Application слой)

Ключевая точка: `backend/src/application/ast_to_ir/type_inference.rs`.

Основные ветки в `infer_type_resolution()`:

1. **Литералы** (`Number/String/Boolean/Date`) → `TypeResolution::primitive(...)`.
2. **Identifier**:
   - спец-случаи `Неопределено/Null`;
   - глобальные коллекции (`Справочники`, `Документы`, …) → `TypeResolution::inferred(name)` (нужно для корректного `PropertyAccess`);
   - иначе lookup в `SymbolTable`, fallback → `TypeResolution::undeclared_variable(name)`.
3. **New** → `TypeResolution::explicit(type_name)` (позже может быть обогащён через `TypeResolver`).
4. **PropertyAccess**:
   - сначала тип базового выражения;
   - конвертация глобальных коллекций в Manager facet (например `Документы.ЗаказНаряды` → `ДокументМенеджер.ЗаказНаряды`);
   - резолв через `TypeResolver` (если доступен) для получения `active_facet` и корректного результата.
5. **Call**:
   - сначала тип объекта/функции;
   - затем поиск сигнатуры/return type через `SignatureIndex`/`TypeResolver` (цепочки вида `Ссылка.ТабличнаяЧасть.Метод()` критичны).

## Детализация: резолв строк типов (Domain слой)

Ключевая точка: `shared/src/domain/resolver/type_resolver.rs`.

`TypeResolver.resolve_expression_sync()` фиксирует порядок резолвинга:

1. **Direct lookup** → `repository.find_type(expression)`
2. **Member access** → `MemberResolver::parse_member_access()` + `resolve_member_access()`
3. **Composite types** → `Union/Intersection/Generic/Nullable` (стратегии)
4. **Primitives fallback** → если репозиторий пуст

Ключевые гарантии, которые должны удерживать тесты:

- `resolve_variable_with_context()` делегирует в `resolve_expression_sync()` для не-generic значений (`shared/src/domain/resolver/context_resolution.rs`).
- `TypeRepository.find_type()` использует `TypeId` и O(1) индекс для регистронезависимых/space-insensitive вариантов имён (`shared/src/domain/repository.rs`).

## Детализация: методы/свойства по TypeResolution

Ключевая точка: `shared/src/domain/metadata_lookup/core.rs`.

`TypeMetadataLookup.get_methods()` имеет приоритеты:

1. Generic result (`ResolutionResult::Generic`) → `get_methods_for_generic()`
2. Generic, распарсенный из `type_name()` строки (когда `result` не Generic, но строка содержит `<...>`)
3. Lazy facet lookup через `active_facet` (для конфигурационных типов)
4. SignatureIndex по нормализованному имени (через `TypeId`)
5. Fallback на `RawTypeData.methods` и fallback по базовому фасетному типу

## Известные ограничения (на текущий момент)

1. **TypeId нормализация**: сейчас нормализация — это в основном `lowercase + remove spaces` (с сохранением `.` и `<...>`). Это удобно для ключей, но намеренно не пытается «угадывать» все варианты написания.
2. **camel_to_spaced() — эвристика**: последовательности заглавных (аббревиатуры) не разбиваются пробелами (пример: `HTTPКлиент` остаётся как есть).
3. **SignatureIndex не хранит english_name**: `TypeMetadataLookup` подтягивает `english_name` из `RawTypeData` только как дополнительное обогащение (если оно есть в репозитории).
4. **Generic parsing из строки**: поддержка «generic в строке» зависит от синтаксического парсера (`GenericStrategy::parse_syntax`) и может быть ограничена для сложных вложенных кейсов.

## Набор контрольных цепочек для регрессионных тестов

См. модуль тестов `shared/src/domain/resolver/resolver_inference_flow_tests.rs`:

1. `SymbolTable → resolve_variable_with_context → resolve_expression_sync → repository.find_type(TypeId)` (CamelCase/space варианты имени дают один результат).
2. `TypeResolver → TypeMetadataLookup.get_methods → SignatureIndex(TypeId)` (встроенные методы `ТабличнаяЧасть.*` доступны даже когда репозиторий хранит тип как `"Табличная часть"`).

