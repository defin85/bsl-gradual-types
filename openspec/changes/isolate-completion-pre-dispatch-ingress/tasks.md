## 1. Contract и truthful pre-dispatch attribution

- [x] 1.1 Поднять authoritative completion timeline до response version `19` и contiguous baseline `contracts/lsp-completion-timeline/v16`, добавив bounded поля для раннего adapter ingress и wait окна `adapter read -> dispatch` без переосмысления legacy `transport_received_at_ms`.
- [x] 1.2 Обновить derived extension verdicts и drilldown так, чтобы server pre-dispatch backlog давал отдельный verdict `adapter_before_dispatch_dominant`, а `client_before_transport_dominant` публиковался только для доказанного client-side wait.
- [x] 1.3 Добавить focused extension tests на truthful split между `client_before_transport_dominant`, `adapter_before_dispatch_dominant` и существующими server-side ingress verdicts, включая additive semantics `adapter_read_at_ms` vs legacy `transport_received_at_ms`.

## 2. Pre-dispatch admission isolation

- [x] 2.1 Перестроить transport admission path в mandatory-модель `reader -> single-owner scheduler`, где только scheduler владеет `poll_ready()/call()`, а completion/control requests и completion-supporting document-sync notifications классифицируются и попадают в очередь до shared readiness blocking.
- [x] 2.2 Зафиксировать и реализовать bounded strict priority lanes `control -> completion -> general`, включая явную queue/backpressure policy: late control classification не должна теряться и после exhausted spillover на completion path, потому что transport fail-closed вытесняет older queued completion через pre-dispatch `queue_rejected`, completion admission остаётся bounded и без silent drop, current-revision handoff для `didOpen/didChange/didSave/didClose` сохраняется, а `general` остаётся primary backpressure lane для unrelated traffic; weighted/fair scheduler в этот change не входит.
- [x] 2.3 Сохранить queued cancellation и exactly-once terminal behaviour для completion, отменённых до dispatch: один terminal response c cancellation semantics, bounded outcome `cancelled`, отсутствие fabricated post-dispatch fields и совместимость с existing post-dispatch completion handoff.

## 3. Regression gates и acceptance

- [x] 3.1 Добавить backend tests на то, что `documentSymbol`/general backlog больше не задерживает completion в окне `adapter read -> dispatch`, не ломает current-revision handoff для `didChange` на default path, что scheduler owner остаётся единственной точкой `poll_ready()/call()`, и что saturated completion lane / completion-supporting barrier не оставляют late `$/cancelRequest` застрявшим до dispatch даже после bounded spillover overflow и pre-dispatch `queue_rejected`.
- [x] 3.2 Добавить backend tests на queued completion cancellation до dispatch: ровно один terminal response, outcome `cancelled`, отсутствие late publish и отсутствие fabricated post-dispatch timestamps после control-lane cancel.
- [x] 3.3 Расширить representative real-module mixed-load gate и change-specific validation wrapper так, чтобы acceptance fail-ил по `p95/max(adapter_to_dispatch_wait_ms)` budget, явно репортил новый pre-dispatch split, не маскировал backlog как client-side ingress и сохранял зелёной blocking representative-matrix perf gate для shipped interactive runtime policy (`completion`, `hover`, `definition`, `members`, `type_at_position`).

## 4. Traceability и runbook

- [x] 4.1 Обновить contracts/changelog, operator docs и readiness traceability для нового pre-dispatch split, additive semantics `adapter_read_at_ms` vs `transport_received_at_ms` и version bump `19/v16`.
- [x] 4.2 Зафиксировать `Requirement -> Code -> Test` evidence для truthful attribution, scheduler ownership, pre-dispatch isolation, queued cancellation terminal outcome на default path и explicit saturation proofs, которые опровергают late-cancel regressions на completion lane / completion-supporting barrier path, включая fail-closed `queue_rejected` для completion, вытеснённых pre-dispatch overflow policy.

## 5. Governance dependencies

- [x] D1 Зафиксировать change-local governance package и acceptance matrix как обязательный fail-closed слой перед merge.
- [x] D2 Связать `test_first` и ownership sign-off с отдельными validation markdown refs внутри change-root.
- [x] D3 Отразить protected assets и traceability evidence в machine-readable dependency checks.
