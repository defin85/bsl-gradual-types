## RENAMED Requirements
- FROM: `### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)`
- TO: `### Requirement: didSave heavy follow-up избегает apply-lag и generic background backlog как primary gate (MUST)`

## MODIFIED Requirements

### Requirement: didSave heavy follow-up избегает apply-lag и generic background backlog как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy follow-up того же `save_cycle_sequence` без unbounded зависимости ни от writer/apply lag, ни от generic background runtime backlog как primary gate.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- использовать одну explicit didSave-follow-up lane/policy для writer-owned applied state, same-version fast paths и canonical fallback, а не выводить isolation только из generic diagnostics operation;
- оформлять эту lane как first-class named admission contract (например, `AdmissionLane::DidSaveFollowup` или семантически эквивалентный type-level contract) с canonical additive telemetry/raw-label value `did_save_followup`, отдельный от бинарного `CpuWorkClass` и от `SemanticOperation::Diagnostics`;
- трактовать didSave-follow-up lane identity как first-class admission contract, отдельный от `SemanticOperation::Diagnostics`, и протаскивать его end-to-end через writer/runtime preparation и blocking CPU admission;
- иметь ровно одного owner outer admission arbiter над applied-state / shadow-state / ready-artifacts / fallback branch fan-out; branch-specific code и facade/runtime helpers MUST потреблять выданный opaque lane admission contract вместо собственного branch-local queueing policy;
- не считать change выполненным, если lane identity лишь помечает work, который по-прежнему входит в те же generic `Background` writer FIFO / CPU permit wait paths без отдельного outer admission boundary до scarce resources;
- сохранять бинарную taxonomy `CpuWorkClass` (`Interactive` / `Background`) и реализовывать didSave-follow-up lane как orthogonal admission concern поверх existing non-interactive/background CPU accounting, а не как третье значение work class;
- реализовывать outer admission boundary как explicit latest-wins arbiter/queue перед scarce writer/runtime resources и удерживать один end-to-end follow-up slot от outer admission через writer/runtime preparation, blocking CPU execution и final pre-publish supersession/quota/disposition decision, освобождая scarce slot до outbound publish/output wait, вместо split writer-vs-CPU quotas или raw CPU-only permit semantics;
- применять эту bounded non-interactive follow-up policy к writer-owned applied-state path, если exact same-version applied state уже известен;
- не позволять writer-owned applied-state path обходить новый lane contract через direct snapshot/query helpers вне lane-aware prepare/admission hooks;
- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`, когда это возможно;
- использовать bounded non-interactive follow-up policy, отличную от generic background lane, для same-version fast paths;
- распространять ту же bounded follow-up policy на didSave fallback path через writer/runtime queue, когда fast-path artifacts недоступны;
- требовать explicit lane/supersession admission envelope до входа в scarce writer/runtime resources, а не полагаться только на generic routing из `SemanticOperation::Diagnostics`;
- вырезать didSave-follow-up lane из existing bounded runtime/CPU budget, а не добавлять net-new total process-wide parallelism;
- трактовать operator-visible follow-up lane quota как global process-wide count end-to-end heavy-follow-up slots, охватывающих outer admission boundary, writer/runtime preparation и blocking CPU execution одного follow-up, и MUST NOT раскалывать этот contract на separately configurable writer-vs-CPU quotas или per-file multiplicative capacity;
- при queued contention хранить не более одного queued candidate на файл и ротировать admission fairly между distinct files, чтобы same-file save storm не создавал raw FIFO blocker для другого файла;
- при effective follow-up lane quota `0` не переводить heavy follow-up молча в generic background lane, а отключать новые `didSave + idle_heavy` admissions без влияния на `save_fastlane`;
- при effective follow-up lane quota `0` re-check-ить effective value на admission boundary до захвата scarce writer/runtime resources для queued-but-not-started work и завершать heavy branch canonical non-cancellation outcome `disabled_by_config`, а не silent absence и не generic cancellation surrogate;
- применять runtime quota changes на outer admission boundary для future admissions; already admitted work MAY finish under already acquired slot и MUST NOT быть retroactively reclassified как `disabled_by_config` mid-flight;
- не повышать heavy follow-up до interactive-class semantics;
- не публиковать older-version diagnostics;
- отсекать stale queued follow-up work до захвата scarce follow-up-lane capacity и повторно проверять supersession перед publish, чтобы older same-file save cycle не становился default blocker для newer cycle;
- сохранять latest-wins / supersession semantics для newer save cycles;
- экспортировать dedicated follow-up-lane telemetry additively через bounded first-class canonical `lane` surface, либо semantically equivalent dedicated runtime-lane family, где stable value `did_save_followup` видна отдельно и queue/exec/saturation signals MUST NOT схлопываться в `interactive/background` или legacy `work_class` как единственную видимую форму;
- представлять `disabled_by_config` canonically не только в save trace, но и в terminal diagnostics outcome/disposition reporting через общий outcome/disposition contract как dedicated non-cancellation disposition/outcome, а не как trace-only string;
- оставлять residual blocker explicit в request-centric trace, если contention всё же происходит.

#### Scenario: delayed apply не держит heavy follow-up hostage при наличии ready save artifacts
- **GIVEN** `didSave` already materialized same-version ready artifacts
- **AND** writer apply path всё ещё отстаёт
- **WHEN** heavy follow-up пытается построить richer diagnostics
- **THEN** система не использует unbounded apply-lag как primary gating step
- **AND** либо публикует richer follow-up, либо truthful trace attribution показывает residual blocker

#### Scenario: writer-owned applied state still uses the isolated follow-up lane
- **GIVEN** writer уже зарегистрировал exact same-version applied state для saved revision
- **AND** richer didSave follow-up всё ещё требует `snapshot_with_deps` и semantic work
- **WHEN** сервер запускает post-fastlane `idle_heavy` follow-up через applied-state branch
- **THEN** эта ветка использует ту же explicit didSave-follow-up lane policy
- **AND** не обходит lane-aware prepare/admission hooks через direct snapshot/query execution
- **AND** не наследует generic background runtime queue backlog как default primary gate

#### Scenario: one shared outer arbiter owns admission before branch fan-out
- **GIVEN** applied-state, shadow-state, ready-artifacts и fallback follow-up branches all remain reachable
- **WHEN** сервер принимает решение о запуске post-fastlane `idle_heavy` follow-up
- **THEN** outer admission, supersession re-check и slot issuance выполняются ровно один раз до branch fan-out
- **AND** branch-specific code consumes shared lane admission facts instead of implementing an independent queue policy

#### Scenario: unrelated background backlog does not dominate post-fastlane follow-up
- **GIVEN** `didSave` уже дал bounded same-version `save_fastlane` first publish
- **AND** generic background runtime lane насыщена unrelated auxiliary/background work
- **WHEN** сервер запускает richer `idle_heavy` follow-up для того же `save_cycle_sequence`
- **THEN** heavy follow-up не наследует generic background backlog как default admission gate
- **AND** request-centric trace не показывает seconds-scale wait только из-за shared background lane

#### Scenario: fallback path stays isolated from generic background writer/runtime backlog
- **GIVEN** same-version fast-path artifacts для saved revision недоступны
- **AND** didSave follow-up вынужден идти через canonical fallback path с `wait_for_file_version` / `snapshot_with_deps`
- **WHEN** generic background writer/runtime queue насыщена другой background work
- **THEN** didSave follow-up fallback не наследует эту очередь как default primary gate
- **AND** более новый same-file save cycle не застревает за stale follow-up fallback work

#### Scenario: zero lane quota disables new heavy follow-up without silent generic-background fallback
- **GIVEN** effective didSave follow-up lane quota equals `0`
- **WHEN** same-version `save_fastlane` first publish уже завершён и сервер рассматривает новый `idle_heavy` didSave follow-up
- **THEN** сервер не reroute-ит follow-up молча в generic background lane
- **AND** `save_fastlane` semantics для этого save cycle остаются неизменными
- **AND** save trace завершает heavy branch explicit non-cancellation outcome `disabled_by_config`

#### Scenario: queued follow-up re-checks zero quota before scarce-lane admission
- **GIVEN** older same-file `didSave` cycle already queued heavy follow-up before the operator changed the dedicated lane quota
- **AND** effective didSave follow-up lane quota becomes `0` before that queued work acquires scarce lane capacity
- **WHEN** admission is re-evaluated at the lane boundary
- **THEN** queued-but-not-started follow-up does not run on stale pre-disable assumptions
- **AND** the heavy branch finishes canonical non-cancellation outcome `disabled_by_config`

#### Scenario: already admitted follow-up is not retroactively disabled by later quota change
- **GIVEN** same-file `didSave` heavy follow-up already crossed the dedicated outer admission boundary and owns an end-to-end follow-up slot
- **AND** the operator lowers the lane quota after that admission, including the case `quota=0`
- **WHEN** the already admitted heavy branch continues toward terminal disposition
- **THEN** it is not reclassified mid-flight as `disabled_by_config`
- **AND** the updated quota governs only subsequent outer-admission decisions

#### Scenario: stale queued follow-up yields to a newer save cycle before monopolizing the isolated lane
- **GIVEN** older same-file `didSave` cycle уже поставил heavy follow-up в dedicated lane
- **AND** более новый same-file `didSave` cycle supersedes older cycle before older follow-up acquires or meaningfully consumes lane capacity
- **WHEN** scheduler/admission policy re-evaluates queued older follow-up
- **THEN** obsolete work is shed before becoming the default blocker for the newer cycle
- **AND** newer cycle keeps latest-wins semantics for both first publish and heavy follow-up

#### Scenario: global single-slot quota does not let one file build a raw FIFO wall for another file
- **GIVEN** effective didSave follow-up lane quota equals `1`
- **AND** file A produces repeated same-file `didSave` cycles while file B also has queued heavy follow-up
- **WHEN** the outer arbiter chooses the next queued admission
- **THEN** the queue retains only the latest queued candidate per file
- **AND** file B is not stranded behind an unbounded FIFO of superseded file-A entries
- **AND** total admitted heavy follow-up work still does not exceed the global quota

#### Scenario: one admitted follow-up owns one scarce slot until the pre-publish disposition decision
- **GIVEN** `didSave` heavy follow-up crossed the dedicated outer admission boundary
- **WHEN** that follow-up performs writer/runtime preparation, blocking semantic execution и затем проходит final pre-publish supersession/quota/disposition decision
- **THEN** the same bounded follow-up slot remains owned through that decision
- **AND** outbound publish/output wait, if any, does not continue monopolizing the scarce didSave-follow-up slot
- **AND** the implementation does not reinterpret the same work through separate writer-vs-CPU lane quotas

#### Scenario: dedicated follow-up-lane telemetry stays separately attributable
- **GIVEN** didSave heavy follow-up uses the isolated lane under contention or execution
- **WHEN** runtime metrics and request-centric traces are exported
- **THEN** queue/exec/saturation facts for that lane stay separately attributable
- **AND** operators do not need to infer the lane only from generic `interactive/background` buckets
- **AND** any compatibility projection into legacy buckets or binary `CpuWorkClass` / `work_class` views does not replace the dedicated lane representation
- **AND** canonical additive telemetry exposes stable lane identity `did_save_followup` through a bounded lane surface or semantically equivalent dedicated runtime-lane family

#### Scenario: nominal background retagging without outer gate is rejected
- **GIVEN** implementation adds a didSave-follow-up marker but still routes queued work through the same generic `Background` scarce admission points
- **WHEN** older queued follow-up or `quota=0` must be re-evaluated before scarce capacity is consumed
- **THEN** such implementation does not satisfy the requirement
- **AND** the dedicated lane contract is considered unmet until an explicit outer admission boundary exists

#### Scenario: residual contention stays explicit after follow-up isolation
- **GIVEN** heavy didSave follow-up всё же сталкивается с residual contention
- **WHEN** diagnostics save trace экспортирует terminal или in-flight состояние
- **THEN** trace сохраняет explicit request-centric blocker facts
- **AND** не подменяет remaining delay на guessed generic `pending`
