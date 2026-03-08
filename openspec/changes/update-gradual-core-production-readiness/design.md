# Design: update-gradual-core-production-readiness

## Context
Change больше не является purely future-facing.

После remediation work shared structural contract, exact acceptance и readiness governance доставлены в код и change-local artefacts:
- `ResolutionMetadata.structural_members` остаётся canonical carrier для snapshot-local member knowledge;
- structural member entry теперь несёт explicit `member_id`;
- completion ranking и adapter payloads больше не теряют structural identity;
- exact acceptance закрывает same-identity, hidden-hint fail-closed и revision-switch leakage;
- governance gate фиксирует honest `declared_status` относительно Beads backlog.

## Goals
- Сохранить архитектурный вывод как delivered contract, а не как устаревший review note.
- Зафиксировать прямую связь `Requirement -> Code -> Test` для shared structural semantics и delivery honesty.
- Убрать future-facing wording там, где change уже backed by code/tests/governance evidence.

## Non-Goals
- Не переписывать runtime на новый carrier поверх уже доставленного `TypeResolution`-centric contract.
- Не устранять bootstrap-only implicit module-context path, пока он остаётся bounded exception и не вводит отдельную structural truth.
- Не дублировать remediation epic task-by-task; delivered evidence уже отражено в change-local artefacts.

## Current Code Signals
- `bsl-types/src/types/certainty.rs` уже хранит snapshot-local structural members внутри `ResolutionMetadata.structural_members`.
- `bsl-types/src/types/structural_members.rs` задаёт shared carrier для `member_id`, `canonical_name`, `member_type`, `source_span`, `certainty`.
- `analysis-v2/src/type_inference_v2/tests.rs` покрывает alias/update/merge lifecycle для typed `Структура` и typed-row.
- `backend/src/bin/lsp_server/server/core/tests.rs` содержит exact cross-consumer acceptance и revision-switch regressions.
- `backend/tests/universal_collection_cross_consumer_consistency_test.rs` фиксирует fail-closed behaviour без shared owner hint.
- `backend/src/bin/lsp_server/handlers/completion.rs` и `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` всё ещё содержат bootstrap-only owner path для implicit module-context symbols; он остаётся явным bounded exception, а не second semantic truth.

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
`stable identity` в рамках этого delivered contract означает consumer-independent member key внутри одного snapshot/revision.

Delivered state:
- явный `member_id` в shared contract;
- identity сохраняется через materialization, replace/update, serialization, ranking/dedup и adapter payloads.

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
- parse-result owner hint как bootstrap input в `backend/src/bin/lsp_server/handlers/completion.rs`, если он лишь помогает прийти к тому же owner resolution, который уже доступен через shared path для reviewed structural scenarios;
- implicit module-context descriptor resolution в `completion_service/member_resolution.rs`, если итогом остаётся обычный `TypeResolution`, читаемый теми же downstream consumers и не создающий structural truth;
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

Delivered evidence:
- lifecycle / identity coverage in `analysis-v2/src/type_inference_v2/tests.rs`;
- contract / serde / replace coverage in `bsl-types/src/types/tests/structural_members_tests.rs`;
- exact `LSP` / `MCP` / runtime / Web assertions in `backend/src/bin/lsp_server/server/core/tests.rs`;
- direct-handler fail-closed checks in `backend/tests/universal_collection_cross_consumer_consistency_test.rs`.

### 4. Delivery readiness MUST быть честной относительно MUST backlog
Если review выявил, что MUST-требования change фактически не доставлены, и для этого создан критический follow-up backlog, исходный change не должен продолжать жить в статусе “complete” только на основании закрытых checklist items.

Нужен readiness gate, который сверяет:
- OpenSpec status / checklist;
- traceability matrix;
- review-gate verdict;
- критический Beads backlog, созданный для закрытия тех же MUST-требований.

#### Governance path
OpenSpec -> Beads governance для этого delivered contract работает так:

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
- пока этот backlog открыт, `readiness_status.json` не позволяет честно объявить change `complete`.

5. Archive truth
- archive допустим только после согласования checklist, strict validation, traceability, review verdict и состояния critical Beads backlog;
- approved superseding delivery path должен явно ссылаться на заменяющий epic/change, иначе он не снимает блокировку `complete`.

### 5. Этот change больше не future-facing
Follow-up remediation epic использовался как delivery path и теперь исчерпан.

Change считается delivered только потому, что:
- code-path gaps закрыты задачами `6mx.1`, `6mx.2`, `6mx.3`, `6mx.7`, `6mx.8`;
- residual risk matrix закрыта задачей `6mx.5`;
- readiness gate и closure evidence закрыты задачами `6mx.4` и `6mx.6`.

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
- В кодовой базе остаётся bootstrap-only implicit module-context path.
  - Mitigation: он явно перечислен как bounded exception, не несёт second structural truth и покрыт honest delivery wording.

## Migration Plan
1. Применить change и архивировать его только после закрытия OpenSpec workflow.
2. Сохранять `TypeResolution`-centric structural contract как canonical path для следующих gradual-typing changes.
3. Удалять bounded bootstrap exceptions отдельными follow-up changes только при наличии replacement path и automated evidence.

## Open Questions
- Нет. Для этого change readiness gate уже автоматизирован tooling-скриптом.
