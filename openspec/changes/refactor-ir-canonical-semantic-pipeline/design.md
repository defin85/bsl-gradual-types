# Design: refactor-ir-canonical-semantic-pipeline

## Context

Текущая архитектура уже частично движется к shared semantic contract, но не завершила переход к единственной canonical truth:
- exact completion path использует `SemanticProgram`/IR;
- `type_at_byte_offset`, `serve_only` и ряд interactive owner-hint/type lookup paths используют отдельный `type_index`;
- `type_index` строится напрямую из `parse_result.program`, а не как projection от canonical IR;
- в runtime уже существует отдельный discovery/search read-model (`IndexSnapshot`), который не должен становиться второй semantic truth;
- часть user-facing availability contract завязана на degraded semantics;
- applied-owner bare identifier fallback остаётся отдельной semantic policy вне canonical IR.
Дополнительно часть miss-path поведения допускает возврат semantic substitute, который для пользователя выглядит как ответ про текущую revision, хотя по сути является stale/degraded approximation.

Пользователь выбрал жёсткий end-state:
- big-bang cutover;
- без long-lived dual runtime behavior в merge state;
- без degraded/stale/keyword semantic fallbacks;
- без applied-owner bare identifier fallback.

## Goals

- Сделать canonical IR единственным semantic source of truth в v2.
- Свести быстрые интерактивные запросы к чтению `derived semantic index`, построенного только из canonical IR snapshot.
- Убрать parallel semantic inference paths и hidden adapter-local semantic truth.
- Сделать fail-closed поведение явной частью публичного semantic contract.
- Убрать возможность выдавать stale semantic result под видом exact/current-revision ответа.
- Сохранить bounded interactive latency через derived index, а не через parallel semantic pipelines.

## Non-Goals

- Не вводить второй новый semantic graph рядом с текущим `SemanticProgram`, если можно расширить существующий canonical IR.
- Не сохранять long-lived runtime dual-write/dual-read path после merge.
- Не оставлять operator rollback или temporary fallback как часть merge-state semantics.
- Не переписывать parser или syntax pipeline, кроме случаев, когда это нужно для поддержки canonical IR builder.

## Current State Signals

- `analysis-v2` имеет два значимых semantic artifacts:
  - canonical `ir(...)`
  - independent `type_index(...)`
- `flow_type_at_byte_offset` уже смешивает оба мира: base truth из `type_index`, narrowing из IR/CFG.
- `hover`, `signatureHelp`, `definition`, completion owner-hint и MCP/Web `type-at-position` активно используют `serve_only`/`type_index` fast paths.
- `completion_missing_ir_policy_decision` и связанные adapter paths поддерживают degraded availability contract.
- `infer_applied_owner_member_identifier` сохраняет отдельную module-context semantic policy.
- `SemanticNodeKind::{MemberAccess, FunctionCall, IndexAccess}` уже хранит canonical topology выражения (`object_node`, `object_span`, `arg_nodes`, `arg_spans`), но resolved receiver/member facts дочитываются не из IR, а через `type_index` lookups по span.
- `SymbolTable` сегодня доказывает visibility/declaration span, но `VariableState` не несёт canonical binding type/origin; typed module-context bindings (`ЭтотОбъект`, `Объект`, `ЭтаФорма`, `Параметры`) живут в `implicit_bindings` / seed logic, а не в typed IR contract.
- `definition` сейчас реконструирует target из `type_at_position_hint`, `receiver_type_hint` и repository lookups, потому что canonical IR не публикует binding/member definition anchors.

## Architecture Drivers

- Correctness: одна semantic truth на snapshot/revision.
- Maintainability: исключить parallel inference logic и drift между consumers.
- Determinism: одинаковый IR snapshot должен давать одинаковый derived semantic index и одинаковые ответы consumers.
- Latency: interactive queries должны оставаться fast без прямого полного IR traversal на каждый запрос.
- Traceability: contract, tests и observability должны доказывать один и тот же semantic path.

## Decisions

### 1. Canonical IR становится единственным semantic source of truth

`SemanticProgram` (или его совместимое расширение) становится единственным источником semantic facts.

Это означает:
- owner/member/type truth не может рождаться из `parse_result.program` как отдельного semantic pipeline;
- implicit semantics, которые нужны продукту, должны быть представлены в canonical IR/resolved facts;
- любые adapter surfaces читают только canonical IR или его derived projections.

`parse_result` сохраняется как syntax artifact:
- для неполного кода,
- для позиционирования,
- для syntax diagnostics,
- для syntax-aware extraction.

Но `parse_result` не считается самостоятельным semantic source.

### 1a. Минимальные расширения canonical IR фиксируются как additive semantic facts

Цель `2.1` не в создании второго semantic graph, а в том, чтобы существующий `SemanticProgram`
нёс минимальный набор canonical facts, из которых можно построить все interactive queries без
parallel semantic inference path.

Нормативные принципы:
- сохраняется один `SemanticProgram` с теми же `SemanticNodeKind`, `SymbolTable`, `CFG` и node-index topology;
- минимальные расширения добавляются как canonical semantic facts внутри `SemanticProgram`
  (или семантически эквивалентного IR-owned storage), а не как отдельный parse-result/type-index pipeline;
- existing `TypeResolution` и `TypeDefinitionLocation` переиспользуются как canonical value objects;
- всё, что является только денормализованным lookup по byte/span/revision, остаётся зоной ответственности `derived semantic index`, а не canonical IR.

Минимально обязательные canonical facts:
- `BindingFact` для identifier-producing bindings:
  - stable binding identity;
  - origin (`local_decl`, `param`, `implicit_module_context`, `global_function`, `common_module`, `global_collection`, или эквивалентный bounded enum);
  - owning scope и declaration anchor/span;
  - typed descriptor (`ContextualTypeDescriptor`, `TypeResolution` или семантически эквивалентная canonical форма) для bindings, у которых тип известен на snapshot build.
- `ExpressionTypeFact` для expression-producing nodes:
  - `node_id/span -> base TypeResolution`;
  - exact expression surface, который обслуживает `type-at-position`, `hover`, completion owner hints и diagnostics;
  - обязательное сохранение `active_facet` / `available_facets` без flattening.
- `ReceiverFact` для `MemberAccess`, `FunctionCall`, `IndexAccess`:
  - canonical receiver node/span;
  - optional binding identity receiver-а, если он происходит из identifier binding;
  - base receiver `TypeResolution`, который больше не требуется восстанавливать через request-time `type_index` lookup по owner span.
- `MemberFact` для `MemberAccess` и method-oriented `FunctionCall`:
  - canonical owner `TypeResolution`;
  - access kind (`property`, `method`, `indexer`);
  - resolved member identity/name в форме, достаточной для metadata lookup и validation;
  - optional result-type / callable-signature anchor, если этот факт уже известен на snapshot build.
- `DefinitionAnchorFact` для go-to-definition:
  - local declaration anchors для variable/parameter/function/procedure bindings;
  - configuration/type anchors через `TypeDefinitionLocation`;
  - common-module и config-member anchors там, где repository-backed definition уже определим на canonical path.

Эта минимальная модель должна покрывать следующие классы запросов:
- owner/member queries: `ReceiverFact` + `MemberFact`;
- type-at-position и hover base truth: `ExpressionTypeFact`;
- definition: `BindingFact` + `DefinitionAnchorFact`;
- explicit module-context semantics: `BindingFact(origin=implicit_module_context)` для `ЭтотОбъект` / `Объект` и других supported bindings.

Явные границы минимального расширения:
- canonical IR НЕ обязан хранить per-byte lookup maps, smallest-containing-node indexes или serve-only caches;
- canonical IR НЕ должен дублировать `type_index` в виде второго списка `span -> TypeResolution`, живущего отдельно от node/binding facts;
- `parse_result` MAY помогать syntax extraction для неполного кода, но MUST NOT синтезировать отсутствующие `BindingFact` / `ExpressionTypeFact` / `DefinitionAnchorFact`;
- flow-sensitive narrowing остаётся overlay поверх base `ExpressionTypeFact`, а не альтернативным semantic source.

Следствие для cutover:
- текущие `SemanticTypeHints` и `type_index`-backed owner/type hints становятся derived projection от этих canonical facts;
- typed module-context bindings перестают быть seed-only detail type inference и становятся частью shared IR contract;
- `definition` больше не требует request-time semantic reconstruction из `receiver_type_hint` / `type_at_position_hint`, кроме чтения уже построенных canonical anchors.

### 2. `derived semantic index` является read-model projection от IR

Новый `derived semantic index` является единственным fast query слоем для интерактивных операций.

Он строится только из canonical IR snapshot текущей revision и материализует
denormalized lookup-структуры над canonical facts, описанными выше. Он может содержать,
например:
- `byte/span -> TypeResolution`;
- receiver/member lookup;
- member identity lookup;
- definition anchors / symbol ownership;
- быстрые owner hints для member access;
- данные, нужные для interactive `type-at-position`, `hover`, `signatureHelp`, `definition`, `members`.

Нормативные свойства:
- индекс не выполняет отдельный semantic inference;
- индекс не читает `parse_result.program` как источник semantic truth;
- индекс не реанимирует legacy fallback semantics.
- индекс сохраняет facet-aware semantic identity конфигурационных типов (`active_facet`, `available_facets` или семантически эквивалентное представление), достаточную для owner/member/property lookup;
- индекс MAY денормализовать facet lookup для fast queries, но MUST NOT сплющивать facet-aware truth до plain metadata/platform type name, если это меняет member/property semantics.

`derived semantic index` нормативно отделён от discovery/search индексов:
- search/discovery read-model (`IndexSnapshot` и эквиваленты) может сосуществовать в runtime;
- search/discovery read-model не является semantic source для `completion`, `hover`, `signatureHelp`, `definition`, `type-at-position`, `members`, `diagnostics`;
- недоступность semantic fast index не даёт права backfill-ить semantic surfaces через discovery/search index;
- координация с pending search changes должна сохранять это разделение явно.

### 2a. Логический состав `derived semantic index`

`derived semantic index` состоит из одного payload-а и optional operational envelope.
Payload является semantic read-model; envelope хранит только operational metadata.

Обязательные логические разделы payload:
- `position surface index`:
  - отображает byte offset / span на минимальную semantic surface (`node_id`, `binding_id`, `member access`, `call surface`);
  - служит общей входной точкой для `type-at-position`, `hover`, `definition`, `signatureHelp`, `members`;
  - заменяет разрозненные request-time probe-эвристики по owner span.
- `expression type table`:
  - materialize-ит `ExpressionTypeFact` для expression-producing nodes и queryable spans;
  - хранит base `TypeResolution` в форме, пригодной для exact lookup без повторного inference;
  - сохраняет facet-aware identity в той же canonical форме, что и IR.
- `binding lookup table`:
  - materialize-ит `BindingFact` для declaration/use sites;
  - связывает source occurrence с binding identity, declaration anchor и binding origin;
  - покрывает local symbols, params, explicit module-context bindings, common modules и другие canonical bindings.
- `receiver/member query table`:
  - materialize-ит `ReceiverFact` и `MemberFact` для `MemberAccess`, `FunctionCall`, `IndexAccess`;
  - хранит canonical owner type, access kind и resolved member identity;
  - даёт fast lookup для completion owner hints, hover property/method formatting, signatureHelp и semantic validation.
- `call query table`:
  - хранит receiver type, argument types и callable anchor для call surfaces;
  - заменяет текущие `call_receiver_type_by_span` / `call_arg_types_by_span` как primary semantic source;
  - допускает thin projections для signature and overload selection, но не новый inference step.
- `definition anchor table`:
  - materialize-ит `DefinitionAnchorFact` для binding/member/type navigation;
  - покрывает local declarations, local functions/procedures, type anchors и repository-backed member definitions, уже выраженные на canonical path.
- `diagnostic hint views`:
  - могут публиковаться как thin denormalized views поверх тех же payload tables;
  - включают текущие категории наподобие `assignment_value_type`, `call_receiver_type`, `call_arg_types`, `member_access_object_type`;
  - не образуют отдельный semantic artifact и не вычисляются независимо от общего index payload.

Запрещённый состав payload:
- отдельный `parse_result`-derived semantic table;
- отдельный stale/degraded payload для serve-only;
- discovery/search entities, нужные только для text/symbol search;
- дублирующий full graph semantic model рядом с canonical IR.

### 2b. Artifact envelope и identity contract

`derived semantic index` публикуется как revision-bound artifact с двумя уровнями идентичности:
- semantic payload identity:
  - один canonical IR snapshot;
  - один semantic deps snapshot;
  - один settings snapshot;
  - один file revision.
- operational cache key:
  - `file_id`;
  - `file_version`;
  - `deps_id`;
  - `settings_id`;
  - или семантически эквивалентный exact key без скрытых ambient inputs.

Envelope MAY содержать только bounded operational metadata:
- `produced_at`;
- build profile / latency counters;
- parse/incremental provenance, если это нужно для observability;
- cache retention / invalidation bookkeeping.

Envelope MUST NOT:
- менять semantic payload shape между exact и serve-only mode;
- разрешать публикацию partially-built semantic payload;
- быть вторым semantic source of truth.

Физическое retention-window хранение старых artifacts допустимо только как cache policy.
Это не меняет semantic contract:
- interactive surfaces читают только exact artifact текущего key/revision;
- stale entries MAY существовать в кеше, но MUST NOT обслуживать semantic queries;
- invalidation по `deps_id` / `settings_id` / revision должна удалять или делать unreadable exact artifacts, чей key больше не совпадает с current snapshot.

### 2c. Contract построения и публикации из одного IR snapshot

Build contract для `derived semantic index`:
- входом является один canonical IR snapshot, уже связанный с exact revision/deps/settings;
- все payload tables строятся в одном build transaction;
- публикация в cache/runtime является atomic publish одного complete artifact;
- при неудаче build не публикуется degraded или partial semantic artifact.

Build MUST:
- читать semantic truth только из canonical IR snapshot и embedded canonical facts;
- быть deterministic для одинакового `(IR snapshot, deps_id, settings_id, file_version)` input set;
- использовать stable IR/node/binding identities, чтобы все tables ссылались на один и тот же semantic payload;
- порождать одинаково согласованные `position -> binding/type/member/definition` результаты для всех consumers.

Build MUST NOT:
- выполнять новый semantic inference из `parse_result.program`, raw syntax tree или document text;
- запрашивать discovery/search read-model как источник semantic data;
- recompute-ить owner/member truth отдельно для каждой table;
- публиковать special fallback artifact для неполного кода, superseded revision или blocked incremental parse scenario.

Неполный код допускается только в пределах уже построенного canonical IR snapshot:
- syntax helpers могут помочь получить current parse snapshot для IR build;
- после того как IR snapshot построен, `derived semantic index` работает только с этим IR snapshot;
- если canonical IR snapshot отсутствует или признан недоступным, semantic index не публикуется.

### 2d. Query contract и переход от текущих artifacts

`derived semantic index` должен обслуживать query surfaces без повторной materialization semantic truth:
- `type-at-position`:
  - exact base type приходит из `position surface index` + `expression type table`;
  - flow-sensitive overlay затем работает поверх этого base result.
- `hover`:
  - symbol/type lookup идёт через `position surface index`, `binding lookup table`, `expression type table`, `receiver/member query table`;
  - property/method hover не должен повторно вычислять owner type через span probes.
- `signatureHelp`:
  - receiver и argument semantics читаются из `call query table`.
- `definition`:
  - binding/member/type navigation читает `binding lookup table` + `definition anchor table`, а не request-time reconstruction через `receiver_type_hint`.
- `semantic diagnostics`:
  - используют `diagnostic hint views` как projection от того же artifact, а не отдельные per-visitor semantic hints.

Следствия для текущей реализации:
- нынешний `TypeIndexArtifact` является transitional названием и должен эволюционировать в artifact,
  который содержит не только `type_at_byte_offset`, но полный `derived semantic index` payload;
- `SemanticTypeHints` должны стать thin view над `derived semantic index`, а не отдельным hand-populated layer;
- on-demand `type_index(...)` compute path допустим только как temporary implementation scaffold;
- merge-state contract остаётся exact-precomputed/read-only для semantic queries текущей revision.

### 3. Interactive queries используют только canonical IR или derived semantic index

Целевой read path:
- `completion`: syntax extraction допустима, semantic candidate truth только из canonical IR + derived semantic index;
- `hover`: type/member truth только из derived semantic index и canonical IR node facts;
- `signatureHelp`: receiver truth только из derived semantic index;
- `definition`: receiver/type hints только из derived semantic index;
- `type-at-position`: base type из derived semantic index, flow-sensitive overlay из canonical IR/CFG;
- `semantic diagnostics`: canonical IR + derived semantic index;
- `MCP` / `Web`: thin adapters над тем же shared runtime contract.

Adapters (`LSP`, `Web`, `MCP`, `CLI`) не создают semantic truth локально:
- допустимы syntax/position extraction и transport mapping;
- недопустимы adapter-local owner/member/type reconstruction из `parse_result`, текста документа или локальных fallback-эвристик;
- недопустимы adapter-local caches, которые переживают revision switch и затем маскируются под current-revision semantic truth;
- при miss canonical artifacts adapter обязан оставаться fail-closed, а не materialize-ить substitute.

### 4. Flow-sensitive анализ остаётся IR-based overlay, но опирается на тот же base contract

Flow-sensitive режим не создаёт отдельную semantic truth.

Базовый unresolved/resolved type для позиции обязан приходить из derived semantic index текущего IR snapshot.
Flow-sensitive logic добавляет narrowing/null-safety поверх того же base contract через canonical IR/CFG.

### 5. Degraded/stale/keyword semantic fallback paths удаляются

После cutover merge-state:
- отсутствуют semantic stale fallbacks;
- отсутствует keyword fallback как substitute для semantic completion truth;
- отсутствует `serve_only -> full semantic fallback`, который меняет semantic truth;
- отсутствуют hidden local owner/member recovery branches.

При недоступности canonical IR/index система работает fail-closed:
- explicit unavailable;
- empty response;
- `None`;
- или иной bounded empty contract, совместимый с конкретным API surface.

Система MUST NOT:
- возвращать stale semantic payload под видом exact/current-revision ответа;
- маскировать substitute-ответ как эквивалент canonical truth текущей revision.

Observability обязана отражать bounded причину fail-closed, но не включать альтернативный semantic path.

Для cutover acceptance reason taxonomy должна быть low-cardinality и различать как минимум:
- `missing_canonical_ir`;
- `missing_semantic_index`;
- `superseded_revision`;
- `cancelled`;
- `unavailable_by_contract`.

Observability контракт дополнительно требует:
- фиксированный bounded набор reason codes без свободных/high-cardinality labels;
- одинаковую интерпретацию reason codes во всех adapters;
- отсутствие reason code, который подразумевает допустимость stale/substitute semantic payload.

### 6. Applied-owner bare identifier fallback удаляется

`ObjectModule` / `RecordSetModule` больше не получают отдельную semantic ветку, которая резолвит bare identifier через implicit owner property lookup вне canonical IR semantics.

Это удаление НЕ означает потерю canonical module-context semantics для explicit контекстных идентификаторов.
Для `ObjectModule` / `RecordSetModule` в canonical IR/binding model должны остаться явно выраженные binding'и как минимум для:
- `ЭтотОбъект`;
- `Объект`.

Эти binding'и:
- принадлежат canonical binding model, а не fallback-ветке;
- типизируются owner object facet / recordset object facet для текущего модуля;
- используются всеми consumers одинаково через shared semantic path;
- позволяют canonical member access вида `ЭтотОбъект.Свойство` / `Объект.Свойство` без adapter-local или type-index-only эвристик.

Если продукту нужна такая языковая семантика, она должна быть:
- выражена в canonical IR/binding model;
- доступна всем consumers одинаково;
- доказана тестами как часть shared truth.

В рамках этого change временное сохранение legacy applied-owner fallback не допускается.

### 7. Big-bang cutover без long-lived dual runtime behavior

Допускается временная scaffolding-работа в feature branch для разработки и тестов.

Но merge-state обязан удовлетворять:
- только canonical IR + derived semantic index path;
- отсутствие production runtime dual-path;
- отсутствие operator-visible rollback semantics, сохраняющих старый parse-result-based semantic core.

## Alternatives Considered

### 1. Оставить `type_index` как независимый semantic pipeline и лишь усилить тесты

Rejected.
Это не устраняет главный архитектурный риск: две semantic truths с разными lifecycle и miss modes.

### 2. Убрать индекс и читать всё напрямую из IR

Rejected.
Для interactive workloads это слишком рискованно по latency. Dynamic-language tooling обычно использует canonical semantic model + derived query indexes, а не прямой полный graph traversal на каждый запрос.

### 3. Делать phased dual-run migration с длительным runtime coexistence

Rejected.
Пользователь явно запросил big-bang end-state. Длительное coexistence увеличивает стоимость поддержки и создаёт ещё один слой drift-risk.

## Risks / Trade-offs

- Поведение станет строже для пользователей при miss canonical artifacts.
  - Mitigation: explicit fail-closed contract, bounded observability reasons, acceptance updates.
- Пользователи чаще увидят empty/unavailable вместо "хоть какого-то" результата.
  - Mitigation: считать это осознанным контрактным изменением; stale/substitute ответ больше не считается допустимым, если он маскируется под current-revision semantics.
- Рефактор затронет hot paths во многих модулях.
  - Mitigation: derived semantic index сохраняет fast query path и позволяет вынести expensive work в snapshot build.
- Возможны perf regressions после удаления stale/degraded shortcuts.
  - Mitigation: проектировать derived index как read-optimized projection и обновить perf contracts/gates.
- Есть риск сохранить canonical bindings, но потерять facet identity при materialization derived semantic index.
  - Mitigation: зафиксировать отдельный facet-preservation contract и acceptance для `active_facet` / `available_facets`-эквивалентной semantic identity.
- Есть риск перепутать удаление applied-owner bare identifier fallback с удалением корректного module-context для `ЭтотОбъект` / `Объект`.
  - Mitigation: явно зафиксировать positive contract для canonical module-context bindings в `ObjectModule` / `RecordSetModule` и acceptance на explicit member access через эти binding'и.
- Pending changes в `mcp-bsl-agent` могут предполагать старое понимание index path.
  - Mitigation: в apply-stage согласовать dependency/supersede policy до начала реализации.
- Big-bang cutover повышает интеграционный риск.
  - Mitigation: заранее зафиксировать execution matrix и cross-consumer acceptance до кодирования.

## Quality Gates

- Representative latency budgets фиксируются для интерактивных semantic queries (`completion`, `hover`, `definition`, `type-at-position`, `members`) на типовых fixtures:
  - member access chain;
  - immediately-after-`didChange` запрос;
  - `ObjectModule` / `RecordSetModule`;
  - неполный код с syntax extraction.
- Cutover acceptance не допускает perf-rescue через alternate semantic path: любые latency проблемы устраняются оптимизацией canonical IR / derived semantic index, а не возвратом stale/degraded semantics.
- Cross-consumer observability должна показывать одинаковый bounded reason code для одинаковой причины fail-closed в `LSP`, `Web`, `MCP`, `CLI`.

## Migration Plan

1. Специфицировать canonical IR contract и derived semantic index contract.
2. Зафиксировать границу между semantic fast index и discovery/search read-model, чтобы pending search changes не ввели вторую semantic truth.
3. Расширить canonical IR так, чтобы он содержал все semantic facts, нужные interactive consumers.
4. Построить derived semantic index как projection от canonical IR snapshot.
5. Перевести `type-at-position`, owner hints, `hover`, `signatureHelp`, `definition`, `completion`, `members`, `diagnostics`, `MCP`/`Web`/`CLI` adapters на новый shared path без local semantic reconstruction.
6. В той же merge-state удалить:
   - parse-result-based semantic index truth,
   - degraded/stale/keyword semantic fallback paths,
   - stale-as-current substitute behavior,
   - applied-owner bare identifier fallback.
7. Перебазировать contracts, acceptance и perf-gates на fail-closed canonical behavior, включая bounded reason codes и representative latency budgets.

## Open Questions

- Нет. Пользователь подтвердил big-bang cutover, удаление degraded/fallback semantic paths и удаление applied-owner bare identifier fallback.
