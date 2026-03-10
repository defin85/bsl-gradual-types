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

### 7a. Adapter cutover contract для `LSP`, `Web`, `MCP`, `CLI`

Big-bang cutover относится к production semantic surfaces, а не ко всем вообще syntax/discovery
операциям runtime.

В scope этого cutover входят:
- `LSP`: `completion`, `hover`, `signatureHelp`, `definition`, semantic `diagnostics`, `type-at-position`;
- `Web`: semantic `hover`, enhanced hover, semantic `diagnostics` и эквивалентные semantic endpoints;
- `MCP`: `bsl_diagnostics`, `bsl_type_at_position`, `bsl_members`, `bsl_definition`;
- `CLI`: terminal/batch entry points, которые публикуют production semantic truth или semantic diagnostics через application/runtime layer.

Вне scope этого cutover остаются syntax/discovery surfaces, которые не являются canonical
semantic consumers этого change, например:
- `document_symbol`;
- `rename`;
- `references`;
- `symbol_search`;
- другие parse-result/discovery-first operations, если они не используются как fallback для semantic surfaces.

Нормативное следствие:
- такие операции MAY жить отдельным migration track;
- они MUST NOT становиться rescue-path для semantic surfaces после cutover.

### 7b. Shared runtime ownership после cutover

После merge-state ownership разделяется так:
- shared semantic runtime:
  - владеет revision/deps/settings gating;
  - строит canonical IR;
  - строит/publish-ит derived semantic index;
  - исполняет semantic queries;
  - публикует shared observability/outcome contract.
- adapters:
  - выполняют transport mapping;
  - выполняют syntax/position preparation;
  - выбирают stateful vs ephemeral orchestration;
  - не materialize-ят semantic truth локально.

Adapter-specific end-state:
- `LSP`:
  - остаётся stateful adapter над `prepare_stateful_operation(...)`;
  - отвечает за `wait_for_file_version`, cancellation policy и LSP transport mapping;
  - не собирает request-time semantic query bundle из raw `AnalysisV2` calls вне shared runtime query contract.
- `Web`:
  - остаётся ephemeral adapter над `prepare_ephemeral_operation(...)`;
  - отвечает за HTTP payload parsing и response formatting;
  - не держит отдельный semantic runtime и не подмешивает discovery snapshot в semantic payload.
- `MCP`:
  - остаётся ephemeral/session-bound adapter над тем же shared facade/runtime;
  - отвечает за session/document overlay resolution, job orchestration и MCP DTO mapping;
  - не строит MCP-only semantic helper path.
- `CLI`:
  - остаётся terminal/batch adapter над тем же application/runtime layer;
  - может использовать offline/ephemeral snapshot preparation;
  - не имеет права заводить отдельный semantic engine для terminal-only поведения.

### 7c. Merge-state invariants для big-bang cutover

Merge-state MUST удовлетворять одновременно:
- у всех production semantic adapters один semantic source of truth: canonical IR + derived semantic index;
- semantic behavior одного и того же snapshot/revision эквивалентен между `LSP`, `Web`, `MCP` и terminal/batch CLI surfaces в пределах их API contract;
- adapter-specific orchestration не меняет semantic truth, а только способ доставки/ожидания snapshot;
- exact current-revision artifact является единственным publishable semantic artifact для interactive semantic queries.

Merge-state MUST NOT содержать:
- runtime feature flag или config knob, который переключает production semantic adapter между old и new semantic engine;
- adapter-local rollback path на parse-result/type-index legacy semantics;
- operator-visible режим "temporary degraded semantics";
- долгоживущий dual-read/dual-write между старым и новым semantic core;
- acceptance, где один adapter ещё работает на legacy path, а другой уже на canonical path.

Branch-only допустимо до merge:
- временно держать dual-path scaffolding;
- собирать дополнительную observability для сравнения;
- держать helper adapters/bridges, нужные только для миграции тестов и кода.

Но до merge эти branch-only элементы MUST быть либо удалены, либо превращены в чисто internal,
non-production implementation detail без alternate semantic behavior.

### 7d. Cutover sequencing и deletion boundary

Допустимая последовательность внутри feature branch:
1. Дособрать shared runtime/query contract над canonical IR + derived semantic index.
2. Перевести `LSP`, `Web`, `MCP`, `CLI` semantic entry points на этот contract.
3. Прогнать cross-interface acceptance/parity.
4. В той же merge-state удалить legacy semantic branches.

Deletion boundary merge-state включает как минимум:
- adapter-local `serve_only -> full` semantic rescue behavior;
- adapter-local request-time reconstruction `receiver_type_hint` / `type_at_position_hint`, если она используется как substitute для отсутствующего shared query fact;
- semantic fallback на discovery/search snapshot;
- stale-as-current semantic substitute behavior;
- old observability labels/paths, которые предполагают допустимость degraded semantic answer.

При этом допустимо сохранить:
- syntax extraction helpers;
- parse-result-backed операции вне semantic scope этого change;
- internal cache/build scaffolding, если оно не публикует alternate semantic behavior и не размывает merge-state contract.

### 8. Contract/version impact для `contracts/**` и public DTO boundaries

`contracts/README` уже задаёт базовое правило merge-state:
- любое breaking изменение machine-readable surface требует новый major directory `vN -> vN+1`;
- breaking изменение обязано сопровождаться migration note в `changelog.md`;
- `scripts/check-contract-compatibility-diff.py` становится обязательным merge gate для всех поверхностей, которые этот change реально затрагивает.

Для этого cutover breaking считаются не только shape changes, но и:
- удаление legacy outcome/reason enum values;
- переименование или удаление observability labels/counters, на которые опираются внешние dashboards/tests;
- semantic reinterpretation существующего поля так, что старый consumer мог бы принять stale/degraded semantics за допустимый current-revision ответ.

### 8a. Surface matrix по versioned contracts

`contracts/lsp-completion-v2/v1` считается breaking-affected surface:
- `v1` закрепляет legacy outcomes `degraded_incomplete` и `fallback_unavailable`;
- merge-state их удаляет как допустимые публичные completion outcomes;
- cutover поэтому требует `contracts/lsp-completion-v2/v2/` с migration note;
- `v2` examples MUST доказывать только два класса semantic поведения: exact current-revision completion или fail-closed current-revision response, без stale/degraded vocabulary.

`contracts/lsp-completion-timeline/v1` считается breaking-affected surface:
- `v1` timeline outcome set и trace interpretation включают legacy `degraded_incomplete`, `fallback_unavailable`, `wait_not_ready`, `missing_ir` как completion-visible end-state;
- после cutover timeline MUST описывать canonical-or-fail-closed execution, а не precompute/fallback lifecycle;
- это требует `contracts/lsp-completion-timeline/v2/` с обновлённым outcome taxonomy и migration note для tooling/dashboard consumers.

`contracts/observability-completion-v2/v1` считается breaking-affected surface:
- `v1` публично закрепляет `fallback_unavailable_counter`, `type_index_reason_counter_prefix`, `type_index_precompute_*` counters/histograms и legacy reason values (`type_index_stale_served`, `type_index_degraded_incomplete`, `type_index_fallback_unavailable`, ...);
- merge-state удаляет stale/degraded/type-index-as-truth semantics, поэтому эти labels/counters больше не могут оставаться публичным compatibility baseline;
- cutover требует `contracts/observability-completion-v2/v2/`, где публичная taxonomy ограничена shared fail-closed reason codes и canonical-path metrics;
- новый contract MUST избегать high-cardinality label dimensions и MUST NOT вводить adapter-specific reason enums для одной и той же runtime причины.

`contracts/observability-diagnostics-v2/v1` в рамках этого slice остаётся совместимым только при жёстком ограничении scope:
- `v1` продолжает описывать publish/cancel pipeline diagnostics (`published`, `superseded_*`, `*_cancel`);
- shared fail-closed reason taxonomy interactive semantic runtime MUST NOT silently подмешиваться в `allowed_reasons` этого `v1`;
- если последующие slices захотят сделать diagnostics-public observability surface carrier для `missing_canonical_ir` / `missing_semantic_index` / `unavailable_by_contract`, это уже отдельный breaking change и требует `contracts/observability-diagnostics-v2/v2/`.

`contracts/intellisense-perf-gate/v1` считается недостаточным authoritative cutover gate, но major bump откладывается на task `2.6`:
- `v1` покрывает только completion-centric metrics/profiles;
- новый merge-state требует representative fixtures и budgets также для `hover`, `definition`, `type-at-position`, `members`;
- поэтому `v1` остаётся историческим baseline, а task `2.6` должен подготовить `contracts/intellisense-perf-gate/v2/` с расширенным набором fixtures/metrics/report fields;
- до появления `v2` perf evidence по одному completion-contract не считается достаточным доказательством cutover acceptance.

`contracts/lsp-index-state/v1` не получает обязательного version impact от этого change:
- cutover меняет semantic source of truth, но не требует менять публичные `states` / `active_operations`;
- bump допускается только если дальнейшая реализация реально меняет этот DTO shape или machine-readable значения.

### 8b. Adapter boundaries и DTO ownership после cutover

Versioned contract существует только там, где surface уже объявлен в `contracts/**`.
На момент этого slice:
- `LSP completion` и completion timeline имеют versioned public contract и обязаны пройти через major bump;
- observability surfaces имеют versioned public contract и обязаны сохранить bounded shared taxonomy;
- для `Web`, `MCP`, `CLI` в `contracts/**` пока нет самостоятельного versioned baseline.

Следовательно граница ownership такая:
- shared semantic runtime владеет semantic truth, fail-closed outcome class и machine-readable reason taxonomy;
- adapter DTO layer владеет только transport mapping: `CompletionList`, JSON response, MCP result envelope, CLI stdout/stderr/report formatting;
- adapters MAY выбирать surface-специфичную форму unavailable (`empty`, `None`, `explicit warning/envelope`), но MUST NOT вводить adapter-local semantic reason enums или stale/degraded substitute fields;
- shared reason codes по умолчанию живут в observability/logging contract, а не в новых public DTO fields;
- если `Web`, `MCP` или `CLI` потребуется новый стабильно поддерживаемый field/enum для fail-closed semantics, merge должен сначала добавить новый versioned surface в `contracts/**`, а не расширять DTO ad hoc.

Это даёт fail-closed merge-state без скрытого расширения публичного API:
- semantic truth не зависит от transport DTO;
- DTO не маскирует stale/degraded answer под success;
- одна и та же runtime причина даёт один и тот же bounded reason code во всех adapters, даже если transport envelope различается.

### 8c. Acceptance и perf expectations, привязанные к новому merge-state

Для affected versioned surfaces acceptance MUST включать:
- compatibility-diff against `master` и явную классификацию breaking vs non-breaking;
- наличие нового `vN` directory и migration note для `lsp-completion-v2`, `lsp-completion-timeline`, `observability-completion-v2`;
- отсутствие legacy enums/labels `degraded_incomplete`, `fallback_unavailable`, `type_index_stale_served`, `type_index_degraded_incomplete` в новых authoritative contract versions;
- cross-adapter evidence, что `LSP`, `Web`, `MCP`, `CLI` мапят один shared runtime outcome в свои transport envelopes без adapter-local semantic rescue;
- evidence, что ни один публичный DTO не маркирует stale result как current-revision success.

Для observability acceptance merge-state MUST дополнительно доказывать:
- фиксированный low-cardinality reason set (`missing_canonical_ir`, `missing_semantic_index`, `superseded_revision`, `cancelled`, `unavailable_by_contract`);
- отсутствие свободных labels по revision/file/request id в публичном contract;
- одинаковую семантику reason codes во всех interactive adapters и отсутствие MCP/Web-only aliases для той же причины.

Perf expectations связываются с тем же merge-state:
- latency/perf regressions являются quality-gate failure, а не поводом вернуть degraded/stale/search-backed behavior;
- authoritative perf report для cutover обязан ссылаться на canonical-path runs и fail-closed reason taxonomy, а не на legacy fallback outcomes;
- до task `2.6` любые perf numbers считаются предварительными, если они не покрывают representative fixtures за пределами completion;
- после task `2.6` contract-level perf evidence MUST блокировать merge, если pass достигается только за счёт скрытого semantic substitute вместо оптимизации canonical IR/derived semantic index path.

### 9. Coordination / supersede-plan для pending MCP/index changes

Этот change становится architectural source of truth для всех активных change-треков, которые затрагивают:
- MCP semantic tools;
- runtime index vocabulary;
- versioned observability/perf contracts;
- adapter/runtime boundaries между search/discovery и semantic truth.

Цель coordination не в том, чтобы автоматически закрыть все соседние change-id, а в том, чтобы до apply-stage исключить merge, где разные pending changes заново вводят неоднозначную границу между:
- `derived semantic index` как canonical semantic fast path;
- `IndexSnapshot` и search indexes как discovery/read-model path;
- adapter transport shaping и semantic runtime truth.

### 9a. `refactor-bsl-agent-index-backed-search` не supersede-ится целиком, но получает жёсткое narrowing

`refactor-bsl-agent-index-backed-search` остаётся допустимым active change только в discovery/search scope:
- `bsl_types_search_start`;
- `bsl_symbol_search_start`;
- `bsl_references_start`;
- parity search endpoints и shared candidate-retrieval facade поверх `IndexSnapshot`.

Для него обязательны следующие coordination rules:
- `IndexSnapshot` и связанные search indexes MAY быть primary path только для discovery/search queries;
- они MUST NOT становиться semantic source для `bsl_type_at_position`, `bsl_members`, `bsl_definition`, `bsl_diagnostics`, `hover`, `completion`, `definition`, `type-at-position` и других semantic surfaces этого change;
- search-specific fallback/rollback semantics MAY существовать только для search tools и MUST NOT переиспользоваться semantic adapters;
- operator override вроде `BSL_AGENT_INDEX_SEARCH=0` допустим только для search/discovery contract и MUST NOT переключать semantic runtime обратно на legacy path;
- search observability (`search_path`, `fallback_reason`, `legacy_forced`) MUST жить в отдельной taxonomy и MUST NOT маскироваться под shared semantic fail-closed reason codes;
- wording `index-backed` в этом change MUST трактоваться как `IndexSnapshot`/discovery path, а не как generic license использовать любой index как semantic truth.

Partial supersede rule:
- если `refactor-bsl-agent-index-backed-search` в будущих правках попытается распространить `IndexSnapshot`, fallback path или rollout override на semantic MCP tools, эти части считаются superseded данным change и не подлежат merge;
- при таком конфликте корректный путь не rollback, а выделение follow-up change с новой явной spec/delta поверх уже принятого semantic boundary.

### 9b. `update-bsl-agent-mcp-ergonomics` остаётся совместимым только как transport/help layer

`update-bsl-agent-mcp-ergonomics` не supersede-ится, если остаётся в своём scope:
- `mcp_help`;
- `build_info`;
- operator-facing error wording;
- convenience wrappers над уже существующими canonical runtime paths.

Для него обязательны rules:
- `bsl_diagnostics_file_start(...)` MUST оставаться thin wrapper над тем же canonical diagnostics path, что и `bsl_diagnostics_start`;
- convenience wrappers MUST NOT добавлять MCP-only semantic cache, MCP-only fail-open behavior или отдельный runtime branch;
- help/README/build_info MAY объяснять fail-closed semantics, но MUST NOT описывать legacy semantic rescue как поддерживаемый operator workflow.

### 9c. `add-bsl-agent-compact-diagnostics-mode` остаётся shape-only change

`add-bsl-agent-compact-diagnostics-mode` не supersede-ится, если его scope ограничен post-query shaping:
- compact/grouped serialization;
- summary fields;
- severity filtering;
- omission of repeated/null transport fields.

Для него обязательны rules:
- shaping MUST применяться только к уже полученному canonical diagnostics result;
- compact mode MUST NOT вводить второй diagnostics pipeline, MCP-only semantic hints или alternate semantic source;
- compact payload MUST NOT становиться carrier для rollout/debug/provenance полей, которые обходят versioned observability contract.

### 9d. `rewrite-v2-observability-perf-pipeline` остаётся downstream rewrite, а не blocker текущего cutover

`rewrite-v2-observability-perf-pipeline` не supersede-ит текущий change и не может откладывать его contract cleanup.

Для него обязательны rules:
- пока rewrite не реализован, bounded taxonomy и version-impact решения из section 8 остаются authoritative v2 baseline;
- dual-write/canary MAY существовать только внутри observability materialization pipeline и MUST NOT означать dual semantic runtime behavior;
- rewrite MUST NOT возвращать legacy degraded/stale/type-index reason labels в качестве временной production-совместимости для semantic surfaces;
- если rewrite захочет изменить versioned surfaces, он обязан делать это отдельным explicit major-bump migration поверх уже зафиксированного cutover contract, а не заменять его задним числом.

### 9e. Apply-stage precedence и merge rules

До `openspec apply` действует следующая precedence:
1. Этот change владеет semantic boundary между canonical IR/derived semantic index и search/discovery indexes.
2. Pending MCP/search/observability changes обязаны rebase-иться на этот boundary, а не переопределять его.
3. При конфликте между convenience/search/telemetry change и canonical semantic contract побеждает fail-closed semantic boundary этого change.

Практические merge rules:
- изменения в `analysis-v2`, `bsl-runtime` facade/query contract и `bsl-agent` semantic managers считаются semantic-boundary-sensitive и не должны принимать generic `index-backed`/`fallback` wording без явного уточнения `search-only` vs `semantic`;
- transport/help/shaping changes MAY идти параллельно, если их acceptance явно доказывает отсутствие нового semantic path;
- если pending change не удаётся cleanly narrow/rebase без размывания semantic boundary, его надо пометить как superseded целиком или разбить на follow-up change до merge, а не тащить ambiguity в apply-stage.

### 10. Representative latency fixtures и bounded taxonomy как обязательный cutover gate

Для этого change quality gates не являются post-factum tuning activity.
Они входят в merge contract и блокируют apply-stage так же жёстко, как spec deltas и compatibility-diff.

Cutover считается недоказанным, пока одновременно не выполнены все четыре класса gate:
- contract gate: versioned contracts и compatibility-diff фиксируют новый authoritative baseline;
- perf gate: authoritative report показывает representative latency/resource evidence для canonical path;
- observability gate: bounded low-cardinality taxonomy доказана contract/tests/runtime evidence;
- acceptance gate: cross-consumer сценарии подтверждают, что fail-closed semantics и latency budget достигаются без alternate semantic path.

Исторические completion-only helpers MAY использоваться как вспомогательная автоматизация, но MUST NOT считаться достаточным cutover evidence сами по себе.

### 10a. Representative fixture families

Representative latency suite MUST покрывать не только scale profiles (`small`, `large`, `churn`), но и semantic fixture families.
Scale profile без semantic fixture не считается representative evidence.

Обязательные fixture families:
- `steady_member_chain`:
  - warm current-revision запрос по цепочке member access в обычном semantic path;
  - обязателен для `completion`, `hover`, `definition`, `type-at-position`, `members`.
- `post_didChange_current_revision`:
  - тот же класс запросов сразу после `didChange`/overlay update;
  - обязан мерить exact-current-revision orchestration (`wait_for_file_version` / snapshot / query) или fail-closed без stale substitute;
  - обязателен для `completion`, `hover`, `definition`, `type-at-position`, `members`.
- `object_module_explicit_context`:
  - explicit access через `ЭтотОбъект` / `Объект` в `ObjectModule`;
  - обязателен для `hover`, `definition`, `type-at-position`, `members`;
  - completion обязателен, если fixture содержит explicit dotted access.
- `recordset_module_explicit_context`:
  - explicit access через `ЭтотОбъект` / `Объект` в `RecordSetModule`;
  - обязателен для `hover`, `definition`, `type-at-position`, `members`;
  - completion обязателен, если fixture содержит explicit dotted access.
- `incomplete_syntax_member_access`:
  - syntactically incomplete код, где syntax extraction ещё допустим, а semantic truth обязана оставаться canonical;
  - completion обязателен всегда;
  - остальные operations MAY измеряться дополнительно только если их публичный contract реально поддерживает такой incomplete input.

Нормативное следствие:
- `small|large|churn` остаются scale axis;
- fixture families остаются semantic axis;
- authoritative gate обязан явно показывать покрытие обеих осей или эквивалентную machine-readable matrix.

### 10b. Required perf evidence shape

Task `2.6` требует подготовить `contracts/intellisense-perf-gate/v2/` как authoritative perf baseline для этого cutover.
До появления этого `v2` apply-stage MUST считать perf evidence предварительным.

`intellisense-perf-gate/v2` MUST требовать:
- machine-readable fixture identifier;
- machine-readable operation identifier;
- scale profile (`small`, `large`, `churn` или совместимый bounded enum);
- обязательный provenance envelope (`change_id`, `generated_at`, `profile`, `schema_version`, `contract_version`);
- bootstrap/sample policy не слабее текущего `sample_size_min>=5` и `aggregation_rule=median`.

Authoritative perf evidence MUST включать по крайней мере:
- total duration metric per operation;
- `wait_for_file_version` latency, если operation проходит через stateful freshness gate;
- snapshot-preparation latency;
- canonical semantic query latency (`ir_query` или семантически эквивалентный metric family);
- resource metrics, достаточные для обнаружения allocator/lock regressions на canonical path.

Для `completion`, `hover`, `definition`, `type-at-position`, `members` contract MUST иметь explicit metric families.
Опора на generic `other` bucket или completion-only metrics как substitute для остальных operations является gate failure.

Существующий completion harness MAY быть переиспользован как один из runners, но только после расширения:
- с completion-only профилей на representative operation coverage;
- с completion-only thresholds на fixture-aware thresholds;
- с completion-specific fallback counters на canonical fail-closed evidence.

### 10c. Bounded observability taxonomy как contract gate

Authoritative public fail-closed taxonomy для interactive semantic surfaces MUST быть конечной и фиксированной.
Для cutover acceptance допустим ровно следующий reason set:
- `missing_canonical_ir`;
- `missing_semantic_index`;
- `superseded_revision`;
- `cancelled`;
- `unavailable_by_contract`.

Допустимые bounded companion dimensions:
- `origin`: `lsp`, `web`, `agent`, `runtime`;
- `operation`: `completion`, `hover`, `definition`, `type-at-position`, `members` и только те дополнительные operations, которые явно включены в versioned contract;
- при необходимости outcome class уровня `ok` / `fail_closed` / `cancelled`, если он остаётся bounded и одинаково трактуется всеми adapters.

Недопустимо для authoritative cutover artifacts:
- legacy/public reasons `missing_ir`, `wait_not_ready`, `fallback_unavailable`, `degraded_incomplete`;
- `type_index_*` reason taxonomy как public semantic gate vocabulary;
- adapter-specific aliases для одной и той же runtime причины;
- high-cardinality labels по `revision`, `uri`, `path`, `request_id`, `symbol_id` в публичном contract.

Normalization sink `other` MAY существовать во внутреннем runtime instrumentation, но authoritative cutover tests/reports MUST доказывать нулевое использование `other` для interactive fail-closed taxonomy на representative fixtures.

### 10d. Merge-blocking failure conditions

Cutover MUST NOT быть признан готовым, если выполняется хотя бы одно из условий:
- отсутствует хотя бы одна обязательная fixture family;
- отсутствует explicit operation coverage для `completion`, `hover`, `definition`, `type-at-position`, `members`;
- perf contract/report остаётся completion-only и не доказывает representative semantic coverage;
- observability artifacts содержат legacy reasons или используют `other` как часть authoritative fail-closed evidence;
- acceptance/perf pass достигается только потому, что runner или adapter вернул stale/degraded/search-backed semantic substitute;
- quality gate numbers зафиксированы только в prose без machine-readable contract/report;
- compatibility-diff или provenance gate не пройдены.

### 10e. Relation to existing helpers and legacy artifacts

Текущие artifacts:
- `contracts/intellisense-perf-gate/v1`;
- `scripts/run-intellisense-perf.sh`;
- `scripts/validate-v2-completion-gates.sh`;
- `contracts/observability-completion-v2/v1`.

Они остаются полезным transitional baseline, но для этого cutover считаются insufficient authority, потому что:
- сосредоточены на completion-centric metrics;
- несут legacy vocabulary (`missing_ir`, `fallback_unavailable`, `type_index_*`);
- не доказывают full representative coverage по `definition`, `type-at-position`, `members` и module-context fixtures.

Следовательно merge-state обязан либо расширить эти helpers/contracts до нового authoritative baseline, либо заменить их эквивалентным machine-readable gate bundle до `openspec apply`.

### 11. Positive canonical contract для `ЭтотОбъект` / `Объект` в `ObjectModule` / `RecordSetModule`

Удаление applied-owner bare identifier fallback MUST NOT удалять корректную explicit module-context semantics, которую ожидает платформа 1C в object и record set modules.
Для этого change `ЭтотОбъект` и `Объект` в `ObjectModule` / `RecordSetModule` считаются не compatibility workaround, а частью canonical semantic contract.

Нормативная опора:
- в 1C module context object/record set module предоставляет explicit current-context object через `ThisObject`;
- record set является самостоятельным applied object surface со своим `ThisObject`-driven context, а не суррогатом manager/reference semantics.

Следовательно removal of bare-identifier fallback меняет только неявную applied-owner эвристику.
Он не имеет права затрагивать explicit identifiers `ЭтотОбъект` / `Объект`, которые уже принадлежат canonical binding model.

### 11a. Binding invariants

Shared runtime MUST обеспечивать следующие инварианты:
- `ObjectModule` MUST публиковать `ЭтотОбъект` и `Объект` как explicit module-context bindings корневого lexical scope текущей revision.
- `RecordSetModule` MUST публиковать `ЭтотОбъект` и `Объект` как explicit module-context bindings корневого lexical scope текущей revision.
- оба имени MUST резолвиться через один canonical binding family, а не через разные adapter-local rules; различается только lexical name, semantic owner fact остаётся одним и тем же.
- для `ObjectModule` canonical descriptor MUST сохранять owner `FacetKind::Object` applied object, то есть `ДокументОбъект.*`, `СправочникОбъект.*` и совместимые facet-aware configuration types.
- для `RecordSetModule` canonical descriptor MUST сохранять recordset object facet, то есть `РегистрСведенийНаборЗаписей.*`, `РегистрНакопленияНаборЗаписей.*` и совместимые recordset object types, а не manager/reference/list substitute.
- `derived semantic index` MUST materialize эти bindings как часть shared semantic runtime state, чтобы `LSP`, `Web`, `MCP`, `CLI` читали один и тот же fact вместо локальной реконструкции.

Практическое следствие:
- `hover`, `definition`, `type-at-position`, `members` и dotted `completion` через `ЭтотОбъект` и `Объект` MUST видеть один и тот же canonical owner/type surface;
- diagnostics MUST считать эти identifiers объявленными в `ObjectModule` / `RecordSetModule`;
- facet-aware identity (`active_facet` / `available_facets` или эквивалент) MUST сохраняться и для binding fact, и для materialized query result.

### 11b. Negative boundary

Этот contract намеренно узкий и fail-closed:
- он НЕ восстанавливает applied-owner bare identifier fallback для `ДоговорКонтрагента`, `Реквизит`, `ТабличнаяЧасть` и других owner members без explicit receiver;
- он НЕ разрешает adapter-local injection aliases, keyword heuristics или late reconstruction из текста документа;
- он НЕ разрешает flattening `ObjectModule` / `RecordSetModule` context до plain owner type name без facet identity;
- он НЕ разрешает получать эти bindings из discovery/search index или из completion-specific fallback path;
- он НЕ расширяет scope на другие module types только потому, что у них тоже может существовать `ЭтотОбъект`; для них остаются собственные contracts.

### 11c. Acceptance consequences

Acceptance bundle для этого change MUST трактовать следующий набор как обязательное доказательство positive contract:
- `type-at-position` на `ЭтотОбъект` и `Объект` в `ObjectModule` возвращает один и тот же object facet owner type;
- `type-at-position` на `ЭтотОбъект` и `Объект` в `RecordSetModule` возвращает один и тот же recordset facet owner type;
- explicit dotted member access через `ЭтотОбъект.<member>` и `Объект.<member>` остаётся canonical для `hover`, `definition`, `members` и, где применимо, `completion`;
- undeclared-variable diagnostics не репортят `ЭтотОбъект` / `Объект` как missing symbols в `ObjectModule` / `RecordSetModule`;
- bare owner member access без explicit receiver остаётся undeclared после removal of fallback, даже если explicit module-context bindings продолжают работать.

Если хотя бы один consumer удерживает этот contract только через private aliasing, stale substitute или operation-specific workaround, cutover считается неуспешным.

### 12. Facet-preservation contract для `derived semantic index`

Facet-aware identity из task `1.8` считается сохранённой только тогда, когда она переживает не только IR construction, но и materialization в `derived semantic index`, shared runtime transport и public DTO surfaces.

Для configuration-backed semantic facts plain type name сам по себе не является достаточной semantic identity.
1C platform различает manager/object/reference/record set surfaces как разные object families с разными members, properties и operations.
Следовательно materialized semantic payload MUST хранить facet envelope как authoritative truth, а строковое имя типа может существовать только как presentation projection поверх него.

### 12a. Authoritative materialized fact set

Для любого materialized fact, который может стать receiver/owner/type truth для interactive semantic operation (`type-at-position`, `hover`, `definition`, `members`, `completion`, `diagnostics`), `derived semantic index` MUST сохранять как минимум:
- configuration owner identity (`metadata kind` + `metadata name`) или семантически эквивалентный canonical key;
- `active_facet`, если semantic fact относится к конкретной facet surface;
- `available_facets`, если они известны для данного configuration type;
- связь между materialized fact и canonical binding/member/definition identity той же revision;
- достаточное представление для восстановления user-facing type label без отдельного facet guess step.

Нормативное следствие:
- `TypeResolution`-подобный envelope является authoritative semantic payload;
- user-facing строка вида `ДокументОбъект.Док1` или `Документы.Док1` MAY кэшироваться в индексе, но MUST NOT становиться единственным semantic carrier;
- отсутствие `active_facet` допустимо только там, где canonical truth действительно facet-neutral; оно не может возникать как артефакт flattening при materialization.

### 12b. Forbidden materialization shortcuts

`derived semantic index` и shared runtime MUST NOT:
- заменять configuration identity одной строкой type name и потом восстанавливать facet эвристикой по operation kind или member name;
- терять `available_facets` при переносе из IR/resolution в queryable runtime payload;
- нормализовать recordset object facet к manager/reference surface только потому, что у них совпадает metadata owner name;
- смешивать transport shaping и semantic normalization, когда adapter/DTO слой "дорисовывает" отсутствующий facet после runtime query;
- считать сериализацию/десериализацию safe, если после round-trip пропадает facet envelope, даже при сохранении `name`.

Если materialized payload не может сохранить facet envelope без потери точности, runtime MUST fail-closed для соответствующего semantic ответа, а не публиковать плоский substitute.

### 12c. Shared runtime verification points

Facet-preservation contract MUST быть проверяемым в трёх точках shared runtime:
- producer point:
  - resolver/type inference при построении canonical facts обязан заполнять `active_facet` и `available_facets` для configuration descriptors;
  - materializer не имеет права обнулять эти поля без явной facet-neutral причины.
- runtime projection point:
  - shared semantic managers обязаны прокидывать `active_facet` / `available_facets` в surface DTO без дополнительной semantic normalization;
  - `TypeInfoDto` и эквивалентные transport shapes считаются projection canonical runtime result, а не вторичным semantic model.
- consumer semantics point:
  - metadata/member lookup обязан зависеть от сохранённого active facet;
  - manager/object/reference/recordset surfaces MUST продолжать возвращать разные member/property sets после materialization и round-trip через runtime responses.

### 12d. Concrete preservation obligations

Cutover acceptance MUST считать facet-preservation недоказанным, если не выполняется хотя бы один из пунктов:
- object-module binding после shared-runtime query не возвращает `active_facet=Object` вместе с полным `available_facets` owner type;
- serialization round-trip для public semantic DTO теряет `available_facets` или меняет их порядок/состав без canonical причины;
- information-register object facet перестаёт expose-ить recordset methods/properties (`Записать`, `ОбменДанными`, `ДополнительныеСвойства`) после materialization;
- manager-only predefined markers начинают протекать в object facet, или object members исчезают из-за flattening в manager/reference substitute;
- любой consumer использует `name` как единственный input для member/property lookup, игнорируя facet envelope.

Практически это означает:
- module-context bindings из section 11 и facet-preservation из section 12 являются одним контрактом, а не двумя независимыми best-effort правилами;
- acceptance/tests должны доказывать не только наличие facet полей в ответе, но и их поведенческую значимость для member/property semantics;
- future `contracts/**` и perf/observability artifacts MUST ссылаться на этот facet envelope как на часть canonical semantic truth, если операция возвращает configuration-backed type info.

### 13. Execution matrix `Requirement -> Code Area -> Test Class`

Эта matrix является рабочим source of truth для tasks `3.2` и `3.3`.
Она фиксирует, где именно живёт обязательное поведение и какие automated assets уже существуют или должны стать authoritative evidence для apply-stage.

| Requirement | Primary code areas | Automated evidence / target test class | Notes for `3.2` / `3.3` |
| --- | --- | --- | --- |
| `IntelliSense v2 обеспечивает IDE-grade completion по выражениям` | `bsl-runtime/src/application/intellisense_v2/`; `backend/src/bin/lsp_server/handlers/completion.rs`; `backend/src/bin/lsp_server/server/core.rs`; `bsl-runtime/src/application/type_system/services/completion_service*` | `backend/tests/intellisense_golden_completion_test.rs`; `backend/tests/m8_completion_matrix_golden_v2_test.rs`; `backend/tests/lsp_incremental_completion_test.rs` | `3.2` должен добавить explicit cross-consumer acceptance beyond completion-only golden coverage. |
| `Инкрементальность и корректность позиций в v2 pipeline` | `backend/src/bin/lsp_server/server/core.rs`; `bsl-runtime/src/application/intellisense_v2/facade.rs`; `analysis-v2/src/lib/analysis_api.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`; `backend/tests/lsp_incremental_completion_test.rs` | Authoritative acceptance обязан явно покрыть immediate-after-`didChange` fail-closed vs exact-current-revision semantics. |
| `v2 pipeline является единственным источником истины для вывода типов` | `analysis-v2/src/lib/snapshots.rs`; `analysis-v2/src/lib/analysis_api.rs`; `bsl-runtime/src/application/intellisense_v2/mod.rs`; `bsl-agent/src/session/manager_semantic_core.rs` | `bsl-agent/src/session/tests.rs::semantic_helpers_fail_closed_without_precomputed_type_index`; `bsl-runtime/src/application/intellisense_v2/facade/tests.rs` | `3.2` должен зафиксировать parity для `hover`, `definition`, `type-at-position`, `members`, а не только completion/type-at-position. |
| `Canonical IR и derived semantic index образуют единый semantic core v2` | `analysis-v2/src/type_inference_v2.rs`; `analysis-v2/src/lib/snapshots.rs`; `bsl-runtime/src/application/intellisense_v2/facade.rs` | `analysis-v2/src/type_inference_v2/tests.rs`; `bsl-runtime/src/system/intellisense_index/tests.rs` | Для apply-stage нужен machine-readable trace, что materialized query path не строит alternate inference branch. |
| `Facet-aware semantic identity сохраняется в canonical pipeline` | `analysis-v2/src/type_inference_v2.rs`; `analysis-v2/src/implicit_bindings.rs`; `shared/src/domain/metadata_lookup.rs`; `bsl-agent/src/session/manager_semantic_core.rs`; `bsl-agent/src/types/mod.rs` | `analysis-v2/src/implicit_bindings/tests.rs`; `analysis-v2/src/type_inference_v2/tests.rs`; `bsl-agent/src/session/tests.rs::collect_type_at_position_preserves_available_facets_for_object_module_binding`; `bsl-api-dtos/src/semantic_dtos/tests.rs`; `shared/src/domain/metadata_lookup/tests.rs`; `backend/tests/undeclared_variable_test.rs` | `3.2` должен объединить module-context and facet-preservation acceptance в один canonical bundle. |
| `Semantic fast index отделён от discovery/search read-model` | `bsl-runtime/src/system/intellisense_index.rs`; `bsl-runtime/src/application/intellisense_v2/mod.rs`; `bsl-agent/src/session/*search*`; `backend/src/presentation/web/handlers.rs` | `bsl-runtime/src/system/intellisense_index/tests.rs`; `bsl-agent/src/session/tests.rs::symbol_search_and_references_work_via_bounded_blocking_workers`; `bsl-agent/src/session/tests.rs::semantic_helpers_fail_closed_without_precomputed_type_index` | Negative proof "search index never rescues semantic query" остаётся обязательным acceptance asset для `3.2`. |
| `Adapter surfaces не реконструируют semantic truth локально` | `bsl-runtime/src/application/intellisense_v2/mod.rs`; `backend/src/bin/lsp_server/server/core.rs`; `backend/src/presentation/web/handlers.rs`; `bsl-agent/src/session/manager_semantic_core.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`; `bsl-agent/src/session/tests.rs` | Web/MCP/LSP parity и отсутствие adapter-local semantic substitute должны быть выражены отдельным acceptance набором в `3.2`. |
| `Canonical semantic queries fail-closed при недоступности артефактов` | `bsl-runtime/src/application/intellisense_v2/facade.rs`; `backend/src/bin/lsp_server/server/core.rs`; `bsl-agent/src/session/manager_semantic_core.rs` | `bsl-runtime/src/application/intellisense_v2/facade/tests.rs`; `backend/src/bin/lsp_server/server/core/tests.rs`; `bsl-agent/src/session/tests.rs::semantic_helpers_fail_closed_without_precomputed_type_index` | `3.2` должен зафиксировать единый fail-closed acceptance across `completion`, `hover`, `definition`, `type-at-position`, `members`. |
| `Fail-closed observability использует bounded reason codes` | `bsl-runtime/src/system/basic_observability.rs`; `contracts/observability-completion-v2/v1/`; `contracts/observability-diagnostics-v2/v1/`; `scripts/check-versioned-contracts.py` | `bsl-runtime/src/system/basic_observability/tests.rs`; contract policy check `scripts/check-versioned-contracts.py` | `3.3` должен заменить legacy vocabulary на authoritative bounded taxonomy и versioned contract baseline. |
| `Interactive latency budget защищается canonical fast path, а не fallback semantics` | `backend/src/bin/intellisense_perf.rs`; `backend/src/perf_gate_evaluator.rs`; `scripts/run-intellisense-perf.sh`; `contracts/intellisense-perf-gate/v1/` | `backend/src/bin/intellisense_perf/tests.rs`; `scripts/test-perf-gate-architecture.py`; perf reports under `backend/tests/perf/` | `3.3` должен оформить operation-aware authoritative gate bundle и `intellisense-perf-gate/v2`. |
| `Applied-owner bare identifier fallback удалён из v2 semantics` | `analysis-v2/src/implicit_bindings.rs`; `analysis-v2/src/type_inference_v2.rs`; `shared/src/domain/metadata_lookup.rs`; `backend/tests/undeclared_variable_test.rs` | `analysis-v2/src/implicit_bindings/tests.rs`; `analysis-v2/src/type_inference_v2/tests.rs`; `backend/tests/undeclared_variable_test.rs`; `bsl-agent/src/session/tests.rs` | `3.2` должен держать explicit module-context positive contract и negative bare-identifier cases в одном acceptance пакете. |

Матрица трактуется fail-closed:
- если requirement не имеет явного code area или automated evidence class, он считается неготовым к apply-stage;
- если evidence существует только в одном consumer, requirement не считается доказанным для shared runtime;
- если evidence живёт только в prose, без test/contract/gate asset, requirement остаётся открытым независимо от степени архитектурной очевидности.

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
