## 1. Contract
- [ ] 1.1 Зафиксировать, что canonical IR является единственным semantic source of truth для v2.
- [ ] 1.2 Зафиксировать, что `derived semantic index` является единственным fast query артефактом и строится только из IR snapshot текущей revision.
- [ ] 1.3 Зафиксировать fail-closed policy для недоступности canonical semantic артефактов вместо stale/degraded/keyword fallback и запрет на маскировку stale ответа под current-revision semantics.
- [ ] 1.4 Зафиксировать удаление applied-owner bare identifier fallback как части semantic contract.
- [ ] 1.5 Зафиксировать, что `parse_result` и другие syntax helpers могут использоваться только для syntax/position extraction, но не как самостоятельный semantic source.

## 2. Design
- [ ] 2.1 Описать минимальные расширения canonical IR, достаточные для owner/member/type/definition queries без parallel semantic inference path.
- [ ] 2.2 Описать состав `derived semantic index` и contract его построения из одного IR snapshot/revision.
- [ ] 2.3 Описать big-bang cutover для `LSP`, `Web`, `MCP`, `CLI` без long-lived dual runtime behavior в merge state.
- [ ] 2.4 Описать contract/version impact для `contracts/**`, observability, acceptance и perf expectations после удаления degraded paths и stale-as-current substitute behavior.
- [ ] 2.5 Описать координацию или supersede-plan для связанных pending MCP/index changes.

## 3. Validation Plan
- [ ] 3.1 Подготовить execution matrix `Requirement -> Code Area -> Test Class`.
- [ ] 3.2 Зафиксировать acceptance набор, который доказывает:
  - [ ] exact cross-consumer semantic equivalence
  - [ ] отсутствие adapter-local semantic truth
  - [ ] fail-closed behavior при miss canonical IR/index
  - [ ] отсутствие stale semantic ответа, замаскированного под current revision
  - [ ] removal of bare-identifier fallback semantics
- [ ] 3.3 Зафиксировать набор quality gates для cutover: tests, contracts, observability/perf checks.
- [ ] 3.4 Прогнать `openspec validate refactor-ir-canonical-semantic-pipeline --strict --no-interactive`.

## Dependencies / Parallelism
- [ ] D1 Пункты 1.1-1.5 блокируют весь design/spec cutover.
- [ ] D2 Пункт 2.1 блокирует 2.2 и semantic spec deltas.
- [ ] D3 Пункты 2.2-2.4 блокируют validation matrix и contract deltas.
- [ ] D4 Пункт 2.5 должен быть согласован до apply-stage, чтобы не внедрять конфликтующий MCP runtime path.
