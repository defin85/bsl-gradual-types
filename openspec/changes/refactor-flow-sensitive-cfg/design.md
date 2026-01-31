# Дизайн: Консолидация CFG и flow-sensitive анализа в v2 pipeline

## Проблема
В репозитории одновременно присутствуют:
- IR CFG: `shared/src/ir/cfg.rs` (CFG как часть IR/semantic program).
- Domain CFG: `shared/src/domain/flow_analysis.rs` (CFG как часть доменной модели + контексты/edge kinds).

Дополнительно:
- `analysis-v2/src/ast_to_ir/converter.rs` содержит `build_cfg()`, но сейчас это заглушка (возвращает `None`).
- Flow-sensitive «экспериментальные» анализаторы существуют в `bsl-runtime/src/domain/flow_analyzer*.rs`, но один из них не подключён в `bsl-runtime/src/domain/mod.rs`.

### Текущее использование (инвентаризация)

- **Domain CFG используется активно**:
  - `shared/src/analysis/narrowing_engine.rs` строит/обходит CFG по `CfgNodeKind`/`EdgeKind`.
  - `shared/src/domain/null_safety.rs` выполняет анализ null-safety по CFG.
  - интеграционные тесты backend строят CFG вручную: `backend/tests/type_narrowing_integration_test.rs`, `backend/tests/flow_sensitive_analysis_test.rs`.
- **IR CFG присутствует как поле в IR, но фактически не строится**:
  - `shared/src/ir/program.rs` содержит `cfg: Option<ControlFlowGraph>`.
  - `analysis-v2/src/ast_to_ir/converter.rs` вызывает `build_cfg()`, но она возвращает `None`.

Следствие: существуют **две разные структуры с одинаковым именем** `ControlFlowGraph`, при этом «рабочая» — в Domain, а «проваленная интеграция в v2» — в IR.

## Цели
- Ввести один «канонический» CFG формат/тип для использования в flow-sensitive анализе.
- Сделать CFG доступным из v2 snapshot (AST → IR → CFG) без альтернативных путей построения.
- Минимизировать путаницу слоёв и конфликт имён.

## Варианты

### Вариант A: IR CFG — канонический, Domain CFG мигрируется поверх IR
Идея: CFG является частью IR (описывает граф выполнения), а flow-sensitive анализ хранит состояния/контексты отдельно (в Domain), привязываясь к узлам IR CFG.

Плюсы:
- Естественно ложится в v2 pipeline: CFG строится там же, где строится IR.
- Упрощает контракт: v2 snapshot содержит IR + CFG, а доменная логика работает поверх них.
- Убирает необходимость держать два CFG типа.

Минусы/риски:
- Domain-логике может не хватить семантики edge kinds и node kinds из Domain CFG; потребуется либо расширить IR CFG, либо адаптировать анализ под текущие IR узлы.

### Вариант B: Domain CFG — канонический, IR CFG удаляется/заменяется
Идея: flow-sensitive CFG является доменной моделью, а IR слой перестаёт иметь собственный CFG.

Плюсы:
- Минимум изменений в существующих доменных анализаторах/tests, которые уже завязаны на Domain CFG.

Минусы/риски:
- Смешение Domain и IR: доменная модель «протекает» в IR представление.
- В v2 pipeline всё равно надо будет строить Domain CFG (а не IR CFG), что усложнит границы слоёв.

## Предпочтение
Рекомендуется Вариант A (IR CFG канонический), так как он лучше соответствует архитектурному принципу: граф выполнения — часть IR/программы, а анализ — доменная логика поверх IR.

## План миграции (в общих чертах)
1) Развести/устранить конфликт имён/типов `ControlFlowGraph`:
   - в `shared/src/ir/cfg.rs` оставить единственный канонический CFG;
   - в `shared/src/domain/flow_analysis.rs` убрать собственные определения CFG и работать с IR CFG (через `pub use`).
2) Реализовать построение CFG в v2 при AST → IR и хранить его в `SemanticProgram.cfg` (не `None`).
3) Адаптировать доменные анализаторы (narrowing/null-safety) под канонический CFG (без дублирования типов).
4) Определить судьбу экспериментальных `bsl-runtime` flow analyzer модулей после появления канонического CFG в v2.

## Критерии успеха (DoD)

- В workspace **нет двух разных `ControlFlowGraph`** как публичных типов (CFG определяется в одном месте).
- `AstToIrConverter::convert_with_resolver(...)` возвращает `SemanticProgram` с `cfg: Some(...)` для файлов, содержащих исполняемые конструкции (assignment/if/loop и т.п.).
- Все потребители (минимум: `shared` narrowing/null-safety и связанные тесты) компилируются и используют канонический CFG.
