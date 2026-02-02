# Дизайн: flow-sensitive v2 wiring (IDE/Web API/MCP) с явным включением

## Цели дизайна
1) **v2-only source of truth**: flow-sensitive результаты вычисляются из v2 snapshot (salsa queries), без legacy/fallback inference путей.
2) **Gating**: вычисления flow-sensitive выполняются только при явном включении флага/настройки (default: OFF).
3) **Привязка к позиции**: результаты должны быть адресуемы для `type-at-position` (byte offsets → flow-sensitive тип).
4) **Единый контракт для интерфейсов**: IDE/Web API/MCP используют один и тот же механизм получения flow-sensitive результата.

## Текущее состояние (наблюдения)
- CFG присутствует в IR (`SemanticProgram.cfg`) и строится в v2 конвертере.
- LSP hover сейчас опирается на `analysis.type_at_byte_offset(...)` (типовой индекс v2) и не использует CFG (`backend/src/bin/lsp_server/handlers/hover.rs`, `analysis-v2/src/lib.rs`).
- Web API принимает `include_flow_sensitive` для semantic tree, но на уровне DTO он пока не влияет (`shared/src/ir/dto.rs`).
- В MCP спецификации уже есть `bsl_type_at_position_start` и `bsl_members_start`, но они не фиксируют flow-sensitive контракт/параметры (см. `openspec/specs/mcp-bsl-agent/spec.md`).

## Архитектурное решение

### 1) Flow-sensitive как v2 queries
Добавить набор v2 queries, вычисляющих flow-sensitive артефакты по требованию:
- `flow_graph(...)` / `flow_cfg(...)`: получить CFG, пригодный для анализа (включая привязку узлов к IR/позициям).
- `flow_type_at_byte_offset(...)`: вернуть уточнённый `TypeResolution` для позиции.
- `flow_diagnostics(...)`: вернуть дополнительные diagnostics (в первую очередь null-safety), которые могут быть добавлены к `semantic_diagnostics(...)`.

Salsa обеспечивает demand-driven вычисления: если интерфейс не запрашивает flow-sensitive результаты, вычисления не выполняются.

### 2) Контракт привязки CFG к IR
Требование “flow-sensitive type-at-position” невозможно реализовать корректно без привязки:
- от “позиции курсора” (byte offset) к IR узлу/выражению,
- от IR узла/выражения к “точке” CFG, на которой валиден flow-sensitive контекст.

В рамках реализации требуется зафиксировать одну из моделей:
- **CFG-per-body (предпочтительно)**: отдельный CFG для каждого тела функции/процедуры (и опционально для root-level statements). Это упрощает:
  - выбор entrypoint анализа,
  - производительность (анализируем только body, содержащий позицию),
  - корректность (нет многократных Entry/Exit без контракта).

Для этого потребуется:
- стабильный идентификатор “владельца CFG” (например, `ScopeId` или индекс узла декларации в IR),
- отображение “позиция → владелец CFG” через IR (scope hierarchy),
- отображение “IR узел/выражение → CFG узел” (через сохранённые связи при построении CFG).

#### Выбранный минимальный контракт (v2 pipeline, без legacy)
Цель: обеспечить корректный `type-at-position` без предположения “первый Entry в файле”.

1) **CFG как набор компонент**: v2 CFG может содержать несколько независимых компонент (root-scope и каждое тело процедуры/функции).
   - Компонента определяется по узлу `CfgNodeKind::Entry` и достижимому из него подграфу.
   - Алгоритмы flow-sensitive MUST инициализировать контексты для *каждого* `Entry`, а не только для первого.

2) **position → CFG node (byte offset)**:
   - базовый выбор: самый “узкий” CFG-узел, span которого содержит позицию (минимальная длина span);
   - UX-хак для границ токена: при промахе разрешается искать назад по небольшому окну (например, до 32 байт).

3) **Уточнение ветки для if/loop header**:
   - если выбранный CFG-узел — `Conditional`, позиция внутри then/else должна попадать в соответствующую ветку;
   - ветка выбирается через привязку `cfg.node_ir_node_index(node_id)` к IR `IfStatement` и сравнение позиции с началом `else_branch` (если есть);
   - для `LoopHeader` используется `ConditionalTrue` как “внутрь тела”.

4) **owner id**:
   - минимально: owner/entrypoint компоненты = `CfgNodeId` entry узла;
   - дополнительно (опционально): связь owner → IR декларации (индекс узла `FunctionDeclaration/ProcedureDeclaration`), если нужна стабильность для внешних контрактов.

### 3) Gating: как включаем flow-sensitive
Flow-sensitive вычисления включаются только при явном флаге/настройке:
- **LSP (IDE)**: настройка сервера/клиента (например, `enable_flow_sensitive`) с default `false`.
  - Сервер MUST иметь быстрый путь без дополнительных запросов/алокаций при `false`.
- **Web API**: параметр запроса `include_flow_sensitive` (default `false`), влияющий на:
  - включение flow-sensitive полей в ответ,
  - запуск соответствующих v2 queries.
- **MCP**: параметры tools `include_flow_sensitive` (default `false`) и/или session-level настройка.

### 4) Какие user-visible эффекты считаются “встроено”
Когда flow-sensitive включён:
- **Hover** MUST использовать flow-sensitive type-at-position (если доступен) и показывать уточнённый тип.
- **Completion** MUST учитывать уточнение типа receiver’а в текущей ветке (narrowing) при подборе members.
- **Diagnostics** MUST включать null-safety diagnostics (и любые другие flow-sensitive диагностические правила, перечисленные в tasks/spec) на основе CFG.
- **SignatureHelp / Definition** MUST использовать flow-sensitive тип в спорных случаях (например, когда тип зависит от ветки).
- **Web API** MUST предоставлять те же улучшения/поля, что IDE, при включённом флаге.
- **MCP tools** MUST возвращать согласованные результаты с IDE/Web API и явно указывать, включён ли flow-sensitive режим в конкретном ответе.

## Точки интеграции (конкретика)
1) **LSP**:
   - добавить серверную настройку `enable_flow_sensitive` (default `false`);
   - wiring: hover/completion/diagnostics/signatureHelp/definition выбирают базовый vs flow-sensitive путь по этому флагу.

2) **Web API**:
   - все параметры `include_flow_sensitive` должны иметь default `false`;
   - minimum endpoints: semantic tree и запросы, использующие `type-at-position` / diagnostics.

3) **MCP (bsl-agent)**:
   - добавить `include_flow_sensitive` (default `false`) в `bsl_type_at_position_start`, `bsl_members_start`, `bsl_diagnostics_start`;
   - ответы должны явно отражать, был ли включён режим (например, поле `flow_sensitive: bool` или аналог).

## Риски и компромиссы
- **Производительность**: flow-sensitive может быть дорогим; gating обязателен, а реализация должна быть инкрементальной и по возможности локальной (per-body).
- **Точность**: на первом этапе важнее корректность контракта и отсутствие “ложной уверенности”, чем полный охват конструкций языка.
- **Совместимость**: изменения DTO/MCP output должны быть версионируемыми/совместимыми (или чётко помеченными).

## DoD (Definition of Done)
- Flow-sensitive отключён по умолчанию во всех интерфейсах.
- При включении:
  - hover/completion/diagnostics/signatureHelp/definition используют flow-sensitive результаты из v2 queries,
  - Web API и MCP возвращают flow-sensitive поля и они совпадают по смыслу с IDE,
  - есть тесты на ON/OFF режимы и на базовые сценарии narrowing/null-safety.
- Repo policy включает CI job для проверки ссылок на пути в документации (используя `scripts/check-doc-paths.py`).
