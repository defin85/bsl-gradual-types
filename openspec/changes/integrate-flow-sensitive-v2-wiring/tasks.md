## 1. Proposal / Design
- [ ] 1.1 Согласовать контракт “позиция → flow-sensitive тип” и выбрать модель CFG (CFG-per-body) + идентификатор владельца.
- [ ] 1.2 Определить список интерфейсов и точек интеграции (LSP/Web API/MCP) и форматы/DTO, которые будут расширены.

## 2. v2 Core (analysis-v2)
- [ ] 2.1 Добавить v2 queries/структуры для flow-sensitive анализа (CFG привязка, flow type-at-position, flow diagnostics).
- [ ] 2.2 Реализовать CFG-per-body (или эквивалентный контракт entrypoints) так, чтобы анализ не опирался на “первый Entry в файле”.
- [ ] 2.3 Реализовать flow-sensitive `type_at_position` (narrowing) на базе CFG и v2 snapshot.
- [ ] 2.4 Реализовать null-safety diagnostics на базе CFG и v2 snapshot.

## 3. IDE (LSP)
- [ ] 3.1 Добавить явную настройку/флаг для включения flow-sensitive (default OFF) и гарантировать, что при OFF нет дополнительных запросов.
- [ ] 3.2 Hover: при включении использовать flow-sensitive тип; добавить тесты ON/OFF.
- [ ] 3.3 Completion: учитывать flow-sensitive тип receiver’а; добавить тесты ON/OFF на базовых кейсах.
- [ ] 3.4 Diagnostics: при включении добавлять null-safety diagnostics; добавить тесты ON/OFF.
- [ ] 3.5 SignatureHelp/Definition: использовать flow-sensitive тип там, где это влияет на результат; добавить минимальные тесты.

## 4. Web API
- [ ] 4.1 Уточнить/добавить параметр `include_flow_sensitive` (default false) для релевантных endpoints и задокументировать поведение.
- [ ] 4.2 Реализовать выдачу flow-sensitive полей/улучшений при включении (в т.ч. semantic tree DTO, если применимо).
- [ ] 4.3 Добавить тесты/смоук проверки для Web API (ON/OFF).

## 5. MCP (bsl-agent)
- [ ] 5.1 Добавить параметр `include_flow_sensitive` (default false) в `bsl_type_at_position_start`, `bsl_members_start`, `bsl_diagnostics_start` и согласовать output.
- [ ] 5.2 Реализовать поведение инструментов при включении: flow-sensitive типы/diagnostics и соответствие IDE/Web API.
- [ ] 5.3 Добавить тесты/контрактные проверки для MCP output (ON/OFF).

## 6. Repo Policy / Docs
- [ ] 6.1 Добавить CI job в repo policy, запускающий `python3 scripts/check-doc-paths.py --targets scripts/doc-path-check-targets.txt`.
- [ ] 6.2 Обновить документацию о составе CI, чтобы она соответствовала фактическим гейтам (при необходимости).

## 7. Validation
- [ ] 7.1 `openspec validate integrate-flow-sensitive-v2-wiring --strict --no-interactive`.
- [ ] 7.2 `cargo test --workspace` (после реализации; до архивации change).

