## ADDED Requirements

### Requirement: didSave follow-up outer slot reserves inner scheduler capacity inside the existing bounded budget (MUST)
Для post-fastlane `didSave + idle_heavy` follow-up outer admission lane сама по себе SHALL NOT считаться достаточной реализацией isolation.

Система MUST считать такой follow-up fully admitted only when the same outer-admission-owned slot also owns an opaque inner execution entitlement inside the existing bounded non-interactive scheduler budget.

Этот contract MUST:

- оставлять diagnostics runtime единственным owner outer admission arbiter;
- не вводить третий `CpuWorkClass` и не повышать heavy follow-up до `Interactive`;
- не borrow-ить interactive reserved capacity;
- не увеличивать total runtime/CPU parallelism;
- удерживать тот же outer-owned slot/entitlement от outer admission через writer/runtime preparation, blocking CPU execution и final pre-publish supersession/disposition decision;
- освобождать scarce inner entitlement до outbound publish/output wait;
- влиять на writer/runtime dequeue and blocking CPU admission as real scheduler input, а не только как telemetry label;
- не позволять admitted follow-up заново вставать в generic `Background` scarce wait paths как default primary gate;
- оставлять unrelated auxiliary/background work generic background work, без права потреблять reserved didSave-follow-up entitlement;
- сохранять additive telemetry `lane=did_save_followup` и existing request-centric wait attribution.

#### Scenario: admitted follow-up does not re-enter generic background CPU permit wait
- **GIVEN** didSave heavy follow-up already crossed the outer admission boundary
- **AND** generic background blocking CPU holders are still active
- **WHEN** the admitted follow-up reaches its blocking syntax or semantic stage
- **THEN** it does not wait again behind the generic `Background` CPU permit queue as its default primary gate
- **AND** the admitted slot keeps owning the same inner execution entitlement through that stage

#### Scenario: admitted follow-up prepare work bypasses generic background writer backlog
- **GIVEN** didSave heavy follow-up already owns the outer slot and inner execution entitlement
- **AND** the shared writer/runtime thread has queued generic background commands
- **WHEN** the follow-up needs lane-aware `wait_for_file_version` or `snapshot_with_deps` preparation
- **THEN** that admitted prepare work is dequeued ahead of generic background backlog by default
- **AND** the implementation still uses the existing shared writer/runtime scheduler rather than a second writer thread

#### Scenario: generic background auxiliary work cannot consume reserved follow-up capacity
- **GIVEN** admitted didSave heavy follow-up owns reserved inner scheduler capacity
- **AND** unrelated generic background work such as `bsl.getCurrentContext` starts concurrently
- **WHEN** scheduler arbitration occurs inside the bounded non-interactive budget
- **THEN** the generic background work does not consume the reserved didSave-follow-up entitlement
- **AND** admitted follow-up is not forced back behind that generic competitor by default

#### Scenario: outer lane marker without inner scheduler reservation is non-compliant
- **GIVEN** implementation propagates `AdmissionLane::DidSaveFollowup` only into trace labels or metrics
- **AND** admitted follow-up still enters the same generic background dequeue and permit wait paths after outer admission
- **WHEN** the system is evaluated against this requirement
- **THEN** the requirement is not satisfied
- **AND** the dedicated lane contract remains incomplete until inner scheduler arbitration changes accordingly
