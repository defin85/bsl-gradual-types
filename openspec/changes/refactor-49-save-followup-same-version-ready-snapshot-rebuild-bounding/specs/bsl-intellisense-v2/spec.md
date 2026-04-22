## ADDED Requirements

### Requirement: Same-version didSave heavy follow-up avoids cold exact rebuild when the saved revision is still current (MUST)

После successful same-version `save_fastlane` first publish система MUST NOT по умолчанию
тратить bounded heavy-follow-up window на seconds-scale cold `parse_exec ->
exact_ready_snapshot_assembly -> program_lowering`, если:

- exact save target для `(file_id, requested_version, text_hash, save_cycle_sequence)` всё ещё
  остаётся current;
- сервер уже имеет matching current-revision state для этого target, достаточный чтобы безопасно
  seed-ить более быстрый exact rebuild path;
- newer same-file revision, explicit queue backlog или writer/apply lag не являются primary
  blocker для данного follow-up.

Для этого система MUST:

- предпочитать reuse-aware same-version exact rebuild path или semantically equivalent exact-safe
  fast path до cold full rebuild;
- сохранять canonical live exact install как источник истины для interactive exact consumers;
- оставаться keyed к exact save target identity и сохранять latest-wins semantics;
- truthfully fall back, если safe reuse proof отсутствует, mismatched, cancelled или superseded;
- сохранять request-centric evidence, из которой видно, остался ли blocker в rebuild-stage
  `parse_exec/program_lowering`, а не в queue/apply attribution.

#### Scenario: Same-version saved revision no longer burns the whole wait inside program_lowering

- **GIVEN** `didSave` уже завершил same-version `save_fastlane` first publish для revision `V`
- **AND** heavy follow-up всё ещё targeting тот же current save target
- **AND** matching current-revision state для этого target уже существует
- **AND** newer revision, explicit queue wait и writer/apply lag не являются primary blocker
- **WHEN** сервер rebuild-ит exact ready snapshot для richer heavy follow-up
- **THEN** он использует reuse-aware same-version fast path до cold full
  `exact_ready_snapshot_assembly/program_lowering`
- **AND** representative same-version path MUST NOT падать в `shadow_state` только потому, что
  exact rebuild остался seconds-scale внутри `parse_exec`

#### Scenario: Missing or stale reuse proof preserves truthful fallback

- **GIVEN** same-version fast rebuild path не может быть доказан safe для current save target
- **OR** target уже superseded, mismatched или cancelled
- **WHEN** heavy follow-up выбирает exact rebuild branch
- **THEN** система сохраняет canonical truthful fallback / supersession behavior
- **AND** не выдаёт `shadow_state` или partial rebuild за canonical exact readiness

### Requirement: Representative save-followup validation fails on rebuild-dominated shadow-state fallback (MUST)

Representative live/perf validation для same-file `didSave` follow-up на `examples/conf_big` MUST
завершаться ошибкой, если same-version saved revision всё ещё приходит к
`followup_semantic_path=shadow_state` только потому, что exact ready-snapshot rebuild был
доминирован rebuild-stage `parse_exec`, включая
`exact_ready_snapshot_assembly/program_lowering`, а не newer revision supersession или отдельно
атрибутированный queue/apply blocker.

Checked-in evidence для этого gate MUST сохранять хотя бы один correlation slice, который
показывает:

- `requested_version` и `save_cycle_sequence` affected follow-up;
- terminal semantic path (`ready_artifacts`, `detached_ready_artifacts` или `shadow_state`);
- bounded-wait probe outcome и ready-snapshot task state;
- dominant phase/checkpoint, включая `program_lowering` attribution, когда он присутствует.

#### Scenario: Live gate fails when same-version rebuild still times out inside exact assembly

- **GIVEN** representative same-file `didSave` profile на крупном модуле
- **AND** `save_fastlane` already published same-version first refresh
- **AND** same-version save target остаётся current
- **WHEN** measured follow-up sample всё ещё приходит к `shadow_state`
- **AND** exported evidence показывает rebuild-dominated `parse_exec` / `program_lowering`
- **THEN** representative gate завершается ошибкой
- **AND** regression не маскируется под old didChange handoff lag, generic client ingress или
  unrelated output wait
