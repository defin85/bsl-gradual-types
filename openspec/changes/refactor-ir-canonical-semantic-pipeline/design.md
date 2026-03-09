# Design: refactor-ir-canonical-semantic-pipeline

## Context

Текущая архитектура уже частично движется к shared semantic contract, но не завершила переход к единственной canonical truth:
- exact completion path использует `SemanticProgram`/IR;
- `type_at_byte_offset`, `serve_only` и ряд interactive owner-hint/type lookup paths используют отдельный `type_index`;
- `type_index` строится напрямую из `parse_result.program`, а не как projection от canonical IR;
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

### 2. `derived semantic index` является read-model projection от IR

Новый `derived semantic index` является единственным fast query слоем для интерактивных операций.

Он строится только из canonical IR snapshot текущей revision и может содержать денормализованные lookup-структуры, например:
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

### 3. Interactive queries используют только canonical IR или derived semantic index

Целевой read path:
- `completion`: syntax extraction допустима, semantic candidate truth только из canonical IR + derived semantic index;
- `hover`: type/member truth только из derived semantic index и canonical IR node facts;
- `signatureHelp`: receiver truth только из derived semantic index;
- `definition`: receiver/type hints только из derived semantic index;
- `type-at-position`: base type из derived semantic index, flow-sensitive overlay из canonical IR/CFG;
- `semantic diagnostics`: canonical IR + derived semantic index;
- `MCP` / `Web`: thin adapters над тем же shared runtime contract.

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

### 6. Applied-owner bare identifier fallback удаляется

`ObjectModule` / `RecordSetModule` больше не получают отдельную semantic ветку, которая резолвит bare identifier через implicit owner property lookup вне canonical IR semantics.

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
- Pending changes в `mcp-bsl-agent` могут предполагать старое понимание index path.
  - Mitigation: в apply-stage согласовать dependency/supersede policy до начала реализации.
- Big-bang cutover повышает интеграционный риск.
  - Mitigation: заранее зафиксировать execution matrix и cross-consumer acceptance до кодирования.

## Migration Plan

1. Специфицировать canonical IR contract и derived semantic index contract.
2. Расширить canonical IR так, чтобы он содержал все semantic facts, нужные interactive consumers.
3. Построить derived semantic index как projection от canonical IR snapshot.
4. Перевести `type-at-position`, owner hints, `hover`, `signatureHelp`, `definition`, `completion`, `members`, `diagnostics`, `MCP`/`Web` adapters на новый shared path.
5. В той же merge-state удалить:
   - parse-result-based semantic index truth,
   - degraded/stale/keyword semantic fallback paths,
   - stale-as-current substitute behavior,
   - applied-owner bare identifier fallback.
6. Перебазировать contracts, acceptance и perf-gates на fail-closed canonical behavior.

## Open Questions

- Нет. Пользователь подтвердил big-bang cutover, удаление degraded/fallback semantic paths и удаление applied-owner bare identifier fallback.
