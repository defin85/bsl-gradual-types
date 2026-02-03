# Дизайн: Консолидация CFG и flow-sensitive анализа в v2 pipeline

## Проблема
Канонический CFG уже определён в IR-слое: `shared/src/ir/cfg.rs`.

При этом flow-sensitive логика живёт в Domain-слое и должна использовать CFG из v2 pipeline:
- `shared/src/domain/flow_analysis.rs` реэкспортирует IR CFG типы (`pub use crate::ir::...`) и содержит доменные структуры (например, `FlowAnalysisContext`).
- `analysis-v2/src/ast_to_ir/converter.rs` строит CFG и пишет его в `SemanticProgram.cfg` (поле `cfg: Option<ControlFlowGraph>`).

Дополнительно:
- Flow-sensitive «экспериментальные» анализаторы существуют в `bsl-runtime/src/domain/flow_analyzer*.rs`; нужно зафиксировать их статус относительно канонического CFG/v2 pipeline.

### Текущее использование (инвентаризация)

 - **Flow-sensitive логика использует CFG активно**:
  - `shared/src/analysis/narrowing_engine.rs` строит/обходит CFG по `CfgNodeKind`/`EdgeKind`.
  - `shared/src/domain/null_safety.rs` выполняет анализ null-safety по CFG.
  - интеграционные тесты backend строят CFG вручную: `backend/tests/type_narrowing_integration_test.rs`, `backend/tests/flow_sensitive_analysis_test.rs`.
- **CFG присутствует как поле в IR и строится в v2 pipeline**:
  - `shared/src/ir/program.rs` содержит `cfg: Option<ControlFlowGraph>`.
  - `analysis-v2/src/ast_to_ir/converter.rs` вызывает `build_cfg()` и возвращает `cfg: Some(...)` на исполняемых конструкциях (assignment/if/loop и т.п.).

Следствие: основная техническая цель этого change — не “устранить раздвоение типов” (оно уже устранено через реэкспорт), а закрепить CFG как first-class часть v2 snapshot и довести потребителей/документацию/тесты до консистентного состояния.

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

Текущее состояние на момент старта реализации:
- (1) выполнено: Domain слой реэкспортирует IR CFG, отдельного Domain `ControlFlowGraph` нет.
- (2) выполнено в базовом виде: CFG строится в `bsl-analysis-v2`; требуется закрепить контракт тестами и убедиться, что CFG доступен из v2 snapshot для flow-sensitive логики.
- (3) частично выполнено: потребители используют `crate::domain::flow_analysis::ControlFlowGraph`, который является IR CFG типом; требуется убрать оставшиеся “ручные” пути построения CFG из тестов/кода, если они обходят v2.
- (4) требует решения: зафиксировать, какие `bsl-runtime` flow-analyzer модули остаются, и какие удаляются/переносятся после консолидации.

## Критерии успеха (DoD)

- В workspace **нет двух разных `ControlFlowGraph`** как публичных типов (CFG определяется в одном месте).
- `AstToIrConverter::convert_with_resolver(...)` возвращает `SemanticProgram` с `cfg: Some(...)` для файлов, содержащих исполняемые конструкции (assignment/if/loop и т.п.).
- Все потребители (минимум: `shared` narrowing/null-safety и связанные тесты) компилируются и используют канонический CFG.
