## ADDED Requirements

### Requirement: didSave heavy follow-up isolation exposes runtime-configurable permit quota with explicit zero semantics (MUST)
Система SHALL описывать в runtime-config registry stable key для quota/permits dedicated non-interactive lane, который обслуживает post-fastlane `didSave + idle_heavy` follow-up.

Этот key MUST:

- иметь machine-readable metadata в registry snapshot;
- быть runtime-mutable без рестарта процесса;
- влиять на последующие admission decisions follow-up lane;
- при отсутствии override иметь default effective value `1`;
- управлять dedicated admission lane, отдельной от бинарной taxonomy `CpuWorkClass`, а не переопределять `Interactive` / `Background` в третий work class;
- обозначать global process-wide count end-to-end didSave-follow-up slots, охватывающих outer admission boundary, writer/runtime preparation, blocking CPU execution и final pre-publish supersession/quota/disposition decision одного heavy follow-up, но MUST NOT включающих outbound publish/output wait, а не набор independently configurable writer-vs-CPU quotas или per-file multiplicative capacity;
- регулировать долю dedicated didSave-follow-up lane внутри существующего bounded runtime/CPU budget и MUST NOT создавать net-new total process-wide parallelism;
- трактовать effective value `0` как explicit disable новых `didSave + idle_heavy` admissions;
- не clamp-ить `0` к `1` и не возвращать didSave heavy follow-up в generic background lane молча;
- применяться на outer admission boundary для future admissions; already admitted work MAY finish under already acquired slot и MUST NOT подвергаться retroactive revocation/reclassification mid-flight;
- не менять contract `save_fastlane`;
- не менять contract interactive lane.

#### Scenario: Operator changes follow-up permit quota without restart
- **GIVEN** сервер уже работает и didSave follow-up isolation lane включён
- **WHEN** runtime override меняет permit quota этого lane
- **THEN** новое effective значение видно в runtime-config snapshot
- **AND** последующие didSave follow-up admissions используют новую quota без рестарта

#### Scenario: Default follow-up permit quota is one bounded slot
- **GIVEN** сервер запущен без operator override для dedicated didSave follow-up lane
- **WHEN** runtime-config snapshot строится из default registry values
- **THEN** effective permit quota этого lane equals `1`
- **AND** default behavior remains bounded without introducing net-new save-storm parallelism

#### Scenario: Positive quota change affects subsequent admissions only
- **GIVEN** один heavy follow-up уже прошёл outer admission boundary dedicated lane
- **AND** оператор runtime override меняет positive permit quota этого lane во время выполнения уже admitted work
- **WHEN** сервер принимает последующие didSave heavy follow-up admissions
- **THEN** новое effective значение governs only those subsequent outer-admission decisions
- **AND** already admitted work does not require retroactive revocation or reclassification

#### Scenario: Admitted slot lifetime ends before outbound publish wait
- **GIVEN** didSave heavy follow-up уже владеет одним admitted slot dedicated lane
- **AND** heavy branch дошёл до final pre-publish supersession/quota/disposition decision
- **WHEN** дальнейший progress упирается только в outbound publish/output wait
- **THEN** quota contract больше не считает этот follow-up владельцем scarce slot
- **AND** slot lifetime не продолжается через publish/output wait

#### Scenario: Operator sets follow-up permit quota to zero
- **GIVEN** сервер уже работает и `save_fastlane` semantics остаются доступными
- **WHEN** runtime override устанавливает permit quota didSave follow-up lane в `0`
- **THEN** runtime-config snapshot явно показывает effective value `0`
- **AND** новые `didSave + idle_heavy` follow-up admissions отключаются без silent fallback в generic background lane

#### Scenario: Zero quota also disables queued-but-not-started follow-up at admission time
- **GIVEN** didSave heavy follow-up was queued before the operator changed the dedicated lane quota
- **AND** effective permit quota becomes `0` before that queued work acquires scarce lane capacity
- **WHEN** the admission boundary is reached
- **THEN** the queued work re-checks the effective runtime-config value instead of relying on stale pre-disable assumptions
- **AND** the server does not silently let that work enter the lane
