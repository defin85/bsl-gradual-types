## 1. Contract
- [x] 1.1 Зафиксировать, что canonical IR является единственным semantic source of truth для v2.
- [x] 1.2 Зафиксировать, что `derived semantic index` является единственным fast query артефактом и строится только из IR snapshot текущей revision.
- [x] 1.3 Зафиксировать fail-closed policy для недоступности canonical semantic артефактов вместо stale/degraded/keyword fallback и запрет на маскировку stale ответа под current-revision semantics.
- [x] 1.4 Зафиксировать удаление applied-owner bare identifier fallback как части semantic contract.
- [x] 1.5 Зафиксировать, что `parse_result` и другие syntax helpers могут использоваться только для syntax/position extraction, но не как самостоятельный semantic source.
- [x] 1.6 Зафиксировать, что semantic `derived semantic index` отделён от discovery/search read-model (`IndexSnapshot` и эквиваленты) и search index MUST NOT быть semantic source для interactive queries.
- [x] 1.7 Зафиксировать, что adapters (`LSP`, `Web`, `MCP`, `CLI`) MUST NOT reconstruct semantic truth локально из `parse_result`, текста документа или adapter-local эвристик.
- [x] 1.8 Зафиксировать, что canonical semantic core сохраняет facet-aware identity configuration types (`active_facet` / `available_facets` или эквивалент) и не допускает flattening до plain type names, меняющего member/property semantics.

## 2. Design
- [x] 2.1 Описать минимальные расширения canonical IR, достаточные для owner/member/type/definition queries без parallel semantic inference path.
- [x] 2.2 Описать состав `derived semantic index` и contract его построения из одного IR snapshot/revision.
- [x] 2.3 Описать big-bang cutover для `LSP`, `Web`, `MCP`, `CLI` без long-lived dual runtime behavior в merge state.
- [x] 2.4 Описать contract/version impact для `contracts/**`, observability reason taxonomy, adapter boundaries, acceptance и perf expectations после удаления degraded paths и stale-as-current substitute behavior.
- [x] 2.5 Описать координацию или supersede-plan для связанных pending MCP/index changes.
- [x] 2.6 Зафиксировать representative latency fixtures и bounded observability taxonomy как обязательные quality gates cutover, а не как post-factum tuning.
- [x] 2.7 Зафиксировать positive canonical contract для module-context bindings `ЭтотОбъект` / `Объект` в `ObjectModule` / `RecordSetModule`, чтобы removal of applied-owner fallback не удалил корректную 1C module semantics.
- [x] 2.8 Зафиксировать facet-preservation contract для `derived semantic index`: какие facet-aware facts должны переживать materialization и как они проверяются в shared runtime.

## 3. Validation Plan
- [x] 3.1 Подготовить execution matrix `Requirement -> Code Area -> Test Class`.
- [x] 3.2 Зафиксировать acceptance набор, который доказывает:
  - [x] exact cross-consumer semantic equivalence
  - [x] отсутствие adapter-local semantic truth
  - [x] отсутствие semantic answers из discovery/search index
  - [x] fail-closed behavior при miss canonical IR/index
  - [x] отсутствие stale semantic ответа, замаскированного под current revision
  - [x] removal of bare-identifier fallback semantics
  - [x] сохранение canonical explicit module-context semantics для `ЭтотОбъект` / `Объект` в `ObjectModule` / `RecordSetModule`
  - [x] сохранение facet-aware member/property semantics без flattening `active_facet` / `available_facets`
- [x] 3.3 Зафиксировать набор quality gates для cutover: tests, contracts, bounded fail-closed reason codes, representative latency budgets и observability/perf checks.
- [x] 3.4 Зафиксировать, что latency regressions не могут закрываться через возврат stale/degraded/search-backed semantic substitute.
- [x] 3.5 Прогнать `openspec validate refactor-ir-canonical-semantic-pipeline --strict --no-interactive`.

## Dependencies / Parallelism
- [x] D1 Пункты 1.1-1.8 блокируют весь design/spec cutover.
- [x] D2 Пункт 2.1 блокирует 2.2 и semantic spec deltas.
- [x] D3 Пункты 2.2-2.4 блокируют validation matrix и contract deltas.
- [x] D3a Пункт 2.6 блокирует финальную формулировку quality gates и acceptance.
- [x] D4 Пункт 2.5 должен быть согласован до apply-stage, чтобы не внедрять конфликтующий MCP runtime path.
