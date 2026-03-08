# Design: update-gradual-core-production-readiness

## Context
Сейчас проект сильнее в архитектурном понимании правильной gradual-системы, чем в полной доставке этого понимания в shared runtime contract.

Подтверждённые симптомы:
- `Certainty` / `UncertaintyReason` и graceful degradation уже формализованы;
- часть acceptance/parity тестов реально ловит drift;
- в design/spec уже закреплена цель единого resolved path;
- при этом snapshot-local structural knowledge ещё не стало first-class shared truth;
- completion сохраняет локальные semantic branches;
- отчётность change может расходиться с фактическим критическим backlog.

## Goals
- Сохранить архитектурный вывод как формальный future contract, а не только как текст ревью.
- Зафиксировать критерии, при которых ядро gradual typing можно будет честно считать production-grade.
- Связать архитектурную готовность и delivery honesty одним change.

## Non-Goals
- Не реализовывать этот контракт в рамках данного change.
- Не дублировать текущий `add-v2-universal-collection-schema-resolution` и его follow-up epic task-by-task.
- Не предписывать немедленный выбор конкретной структуры данных, если соблюдён shared-contract result.

## Current Code Signals
- `bsl-types/src/types/certainty.rs` уже хранит snapshot-local structural members внутри `ResolutionMetadata.structural_members`.
- `bsl-types/src/types/structural_members.rs` уже задаёт shared carrier для `canonical_name`, `member_type`, `source_span`, `certainty`.
- `analysis-v2/src/type_inference_v2/tests.rs` уже проверяет materialization typed `Структура` и typed-row, а также сохранение `source_span`.
- `backend/src/bin/lsp_server/server/core/tests.rs` уже содержит exact cross-consumer acceptance для typed `Структура` и typed-row.
- `backend/src/bin/lsp_server/handlers/completion.rs` и `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` всё ещё содержат bootstrap/local owner-resolution logic, которую future remediation должна либо удалить, либо свести к thin adapter.

## Decisions

### 1. Shared structural knowledge MUST стать first-class contract
Typed `Структура` и typed-row не могут считаться fully shared semantics, пока их member knowledge не живёт в общем контракте как first-class данные.

Минимальный required payload для structural member:
- canonical member name;
- stable identity;
- member type;
- certainty;
- source span / source location.

Representation вида `Структура<vec<ConcreteType>>` или generic lookup только по `base_type` недостаточна для роли shared truth.

#### Preferred shape
Предпочтительная форма shared contract для этого change: расширенный `TypeResolution`, где snapshot-local structural members остаются частью одного owner resolution через `ResolutionMetadata.structural_members`.

Причины выбора:
- это уже совпадает с текущим ядром типов и не требует второго параллельного carrier для одной и той же semantic truth;
- hover / type-at-position / diagnostics уже умеют читать `TypeResolution` как общий resolved result;
- equality, cloning, serialization и snapshot lifecycle уже привязаны к `TypeResolution`, поэтому риск drift между owner result и отдельным sidecar ниже;
- существующие typed `Структура` / typed-row tests уже формулируются в терминах `find_structural_member(...)`, а не через второй join-step.

Эквивалентный explicit sidecar contract допустим только как controlled alternative, если одновременно соблюдены все условия:
- sidecar versioned и snapshot-scoped вместе с owner `TypeResolution`;
- consumer получает его через тот же resolved API boundary, без отдельного локального join/inference шага;
- member identity и source location стабильно сериализуются и сравниваются между consumers;
- sidecar не создаёт второй semantic source of truth относительно owner resolution.

#### Stable identity interpretation
`stable identity` в рамках этого future contract означает consumer-independent member key внутри одного snapshot/revision.

Предпочтительный end-state:
- явный `member_id` или эквивалентный stable token в shared contract.

Допустимый transitional equivalent:
- нормализованное canonical member name вместе с owner identity и source location, если эта композиция выдаётся всем consumers как один и тот же stable contract key.

### 2. Semantic consumers MUST использовать один resolved path
`completion`, `hover`, `type-at-position`, `semantic diagnostics`, а также adapter surfaces (`LSP`, `MCP`, Web helpers) должны читать owner/type из одного semantic contract.

Допустимы только thin adapters:
- меняют формат ответа;
- не вводят отдельную schema/effect truth;
- не выполняют consumer-local inference как новый источник смысла.

Если временные исключения остаются, они должны быть:
- явно перечислены;
- покрыты migration plan;
- removed-by-default целью, а не бессрочной особенностью.

#### Migration strategy for consumer-local branches
Migration strategy делится на четыре шага:

1. Inventory
- перечислить все consumer-local semantic branches, которые влияют на owner/member truth;
- отдельно пометить bootstrap paths, adapter-only paths и forbidden end-state branches.

2. Converge on shared owner/member contract
- structural owner/member result должен материализоваться в одном shared resolved path до consumer-specific formatting;
- completion, hover, type-at-position и diagnostics должны читать один и тот же owner/member contract для одного snapshot/revision.

3. Shrink temporary exceptions
- временно допустимы только bootstrap paths, которые помогают дойти до того же shared resolved result и не меняют semantic truth;
- такие paths обязаны иметь exit criterion и removal target, а не статус постоянной совместимости.

4. Remove hidden semantic reconstruction
- после появления shared structural contract consumer-local owner/member reconstruction удаляется;
- в end-state у consumer остаются только ranking, formatting, snippet generation, protocol mapping и другие thin adapter concerns.

#### Temporary exceptions catalog
Для текущей кодовой базы временно допустимы только такие completion-specific исключения:
- parse-result owner hint как bootstrap input в `backend/src/bin/lsp_server/handlers/completion.rs`, если он лишь помогает прийти к тому же owner resolution, который позже станет доступен через shared path;
- implicit module-context descriptor resolution в `completion_service/member_resolution.rs`, если итогом остаётся обычный `TypeResolution`, читаемый теми же downstream consumers;
- LSP/Web/MCP response shaping, snippet support и candidate ranking как adapter-only поведение.

Недопустимый end-state:
- completion-only schema/effect truth;
- hidden local owner reconstruction, без которого hover/type-at-position/diagnostics не могут доказать тот же результат;
- acceptance, зависящий от consumer-specific hints, не представимых другим consumers.

### 3. Acceptance MUST доказывать shared semantics, а не только отсутствие явного drift
Smoke/parity проверки полезны, но недостаточны как единственное доказательство общей модели знания.

Production-grade acceptance должна уметь проверять как минимум:
- один и тот же owner resolution результат;
- одну и ту же member identity;
- отсутствие hidden consumer-only hints как условия корректного результата;
- одинаковую policy реакцию на known/unknown member.

#### Exact acceptance matrix
Минимальная acceptance matrix для production-grade shared semantics:

| Scenario | Shared truth under test | Consumers / surfaces | Exact assertion |
| --- | --- | --- | --- |
| Typed `Структура` field materialized by `Вставить` | owner carries first-class member entry | completion, hover, type-at-position, diagnostics | same canonical member, same member type, no unknown-member drift for known field |
| Typed-row column materialized by `Колонки.Добавить` or `Добавить()` row | row owner carries first-class column entry | completion, hover, type-at-position, diagnostics | same column identity, same value type, same source location semantics |
| Known vs unknown member policy | access policy remains shared, not consumer-specific | diagnostics, hover, completion filtering | typo emits unknown-member policy everywhere; known member never regresses to unknown |
| Hidden-hint independence | semantic correctness does not depend on one consumer-only hint | LSP, MCP, Web, direct shared runtime API | removing consumer-local hint either keeps same result or fails acceptance as drift |
| Adapter parity | adapters do not fork semantic truth | LSP, MCP, Web | payload shape may differ, but owner/member contract and policy verdict remain equivalent |

Required evidence classes:
- `analysis-v2` unit tests for typed `Структура` / typed-row materialization and snapshot isolation;
- backend exact cross-consumer acceptance tests for completion + hover + type-at-position + diagnostics;
- cross-interface parity tests for LSP / MCP / Web;
- negative tests that intentionally remove hidden hint paths and expect acceptance failure when semantic truth is not shared.

### 4. Delivery readiness MUST быть честной относительно MUST backlog
Если review выявил, что MUST-требования change фактически не доставлены, и для этого создан критический follow-up backlog, исходный change не должен продолжать жить в статусе “complete” только на основании закрытых checklist items.

Нужен readiness gate, который сверяет:
- OpenSpec status / checklist;
- traceability matrix;
- review-gate verdict;
- критический Beads backlog, созданный для закрытия тех же MUST-требований.

#### Governance path
OpenSpec -> Beads governance для этого future contract должен работать так:

1. Contract truth
- OpenSpec requirement и design задают MUST truth и acceptance expectations.

2. Traceability truth
- traceability artifact связывает каждый MUST с future code area и required test class;
- если traceability показывает `gap`, checklist не может переопределить это в `covered`.

3. Review truth
- review-gate verdict обязан фиксировать `pass`, `partial` или `gap` по каждому MUST;
- optimistic `complete` запрещён, если review artifact расходится с traceability или открытым critical backlog.

4. Execution truth
- каждый critical MUST gap получает связанный Beads epic/task graph;
- пока этот backlog открыт, change остаётся `partial`, `not ready` или эквивалентно незавершённым.

5. Archive truth
- archive допустим только после согласования checklist, strict validation, traceability, review verdict и состояния critical Beads backlog;
- approved superseding delivery path должен явно ссылаться на заменяющий epic/change, иначе он не снимает блокировку `complete`.

### 5. Этот change future-facing и зависит от более узких remediation changes
Текущий change не заменяет remediation-level change/epic. Он фиксирует более широкий стандарт готовности, к которому должны прийти follow-up работы.

## Alternatives Considered

### Оставить анализ только в review-комментарии
Rejected.
Такой вывод быстро теряется и не становится частью change governance.

### Ограничиться только product-spec без dev-workflow части
Rejected.
Тогда теряется ключевой вывод про расхождение между declared completion и реальной readiness.

### Вынести только process-гейт без архитектурного контракта
Rejected.
Это решает honesty вопрос, но не сохраняет самую важную technical target state.

## Risks / Trade-offs
- Change объединяет архитектурную и процессную тему.
  - Mitigation: scope ограничен readiness contract и не уходит в implementation details.
- Возможен overlap с текущими active changes.
  - Mitigation: этот change явно future-facing и не заменяет remediation work, а задаёт следующий критерий зрелости.

## Migration Plan
1. Утвердить future readiness contract.
2. Использовать preferred `TypeResolution`-centric structural contract как default target для follow-up changes в `bsl-intellisense-v2`.
3. Свести completion-specific semantic branches к bootstrap-only исключениям с явным removal plan.
4. Доставить acceptance matrix и cross-interface exact assertions как обязательный evidence layer.
5. После реализации remediation work добавить governance gate, который связывает OpenSpec completion с реальным MUST backlog.

## Open Questions
- Должен ли readiness gate быть автоматизирован через tooling, или на первом этапе достаточно обязательного review artifact?
