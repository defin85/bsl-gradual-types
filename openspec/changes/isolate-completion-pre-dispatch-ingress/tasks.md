## 1. Contract и truthful pre-dispatch attribution

- [x] 1.1 Поднять authoritative completion timeline до response version `19` и contiguous baseline `contracts/lsp-completion-timeline/v16`, добавив bounded поля для раннего adapter ingress и wait окна `adapter read -> dispatch` без переосмысления legacy `transport_received_at_ms`.
- [x] 1.2 Обновить derived extension verdicts и drilldown так, чтобы server pre-dispatch backlog давал отдельный verdict `adapter_before_dispatch_dominant`, а `client_before_transport_dominant` публиковался только для доказанного client-side wait.
- [x] 1.3 Добавить focused extension tests на truthful split между `client_before_transport_dominant`, `adapter_before_dispatch_dominant` и существующими server-side ingress verdicts, включая additive semantics `adapter_read_at_ms` vs legacy `transport_received_at_ms`.

## 2. Pre-dispatch admission isolation

- [x] 2.1 Перестроить transport admission path в mandatory-модель `reader -> single-owner scheduler`, где только scheduler владеет `poll_ready()/call()`, а completion/control requests классифицируются и попадают в очередь до shared readiness blocking.
- [x] 2.2 Зафиксировать и реализовать bounded strict priority lanes `control -> completion -> general`, включая явную queue/backpressure policy: reserved progress для `control`, bounded completion admission без silent drop и `general` как primary backpressure lane; weighted/fair scheduler в этот change не входит.
- [x] 2.3 Сохранить queued cancellation и exactly-once terminal behaviour для completion, отменённых до dispatch: один terminal response c cancellation semantics, bounded outcome `cancelled`, отсутствие fabricated post-dispatch fields и совместимость с existing post-dispatch completion handoff.

## 3. Regression gates и acceptance

- [x] 3.1 Добавить backend tests на то, что `documentSymbol`/general backlog больше не задерживает completion в окне `adapter read -> dispatch`, и что scheduler owner остаётся единственной точкой `poll_ready()/call()`.
- [x] 3.2 Добавить backend tests на queued completion cancellation до dispatch: ровно один terminal response, outcome `cancelled`, отсутствие late publish и отсутствие fabricated post-dispatch timestamps после control-lane cancel.
- [x] 3.3 Расширить representative real-module mixed-load gate и change-specific validation wrapper так, чтобы acceptance fail-ил по `p95/max(adapter_to_dispatch_wait_ms)` budget, явно репортил новый pre-dispatch split и не маскировал backlog как client-side ingress.

## 4. Traceability и runbook

- [x] 4.1 Обновить contracts/changelog, operator docs и readiness traceability для нового pre-dispatch split, additive semantics `adapter_read_at_ms` vs `transport_received_at_ms` и version bump `19/v16`.
- [x] 4.2 Зафиксировать `Requirement -> Code -> Test` evidence для truthful attribution, scheduler ownership, pre-dispatch isolation и queued cancellation terminal outcome на default path.
