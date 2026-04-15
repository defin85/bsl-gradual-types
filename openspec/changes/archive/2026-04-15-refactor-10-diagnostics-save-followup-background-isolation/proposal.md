# Change: isolate didSave heavy follow-up from generic background runtime backlog

## Why
Свежий incident bundle `2026-04-08T09-34-09Z` показал, что `refactor-09` закрыл observability gap, но не устранил сам latency tail. На representative `conf_big` save-flow:

- `save_fastlane` first publish уже bounded (`47ms`);
- `idle_heavy` follow-up still publishes only after `75848ms`;
- request-centric trace уже прямо показывает доминирующий blocker: `runtime_queue_wait_ms=69712`, при вторичных `apply_lag_ms=3006` и `semantic_diagnostics_query_ms=6126`.

Кодовое расследование подтвердило причину: post-fastlane didSave follow-up по-прежнему исполняется в общем `CpuWorkClass::Background` и тем самым наследует generic background saturation от unrelated auxiliary/background jobs (`bsl.getCurrentContext`, parse-snapshot enrichment, type-index precompute, other background diagnostics work). Это конфликтует с intent, что после bounded same-version first refresh richer follow-up не должен по умолчанию висеть за чужим background backlog.

Дополнительный read-only разбор показал, что проблема не сводится к одному флагу маршрутизации:

- facade/runtime contract и writer queue по-прежнему бинарны (`Interactive` / `Background`);
- applied-state fast path currently bypasses generic lane-aware prepare/admission hooks;
- runtime observability and legacy projections are also binary, so dedicated follow-up visibility is currently lost unless a first-class lane surface is added.

## What Changes
- Зафиксировать в `bsl-intellisense-v2`, что весь post-fastlane `didSave + idle_heavy` follow-up, включая fallback через writer/runtime queue, MUST быть изолирован от generic background runtime backlog как default primary gate.
- Спроектировать remediation вокруг explicit dedicated bounded follow-up admission lane, оформленной как first-class facade/runtime admission contract (например, `AdmissionLane::DidSaveFollowup` или семантически эквивалентный тип) с canonical additive telemetry/raw-label value `did_save_followup`; этот contract одинаково обслуживает writer-owned applied-state, shadow-state, ready-artifacts и didSave fallback path, оставаясь отдельным от бинарного `CpuWorkClass` contract (`Interactive` / `Background`), не повышая follow-up до `Interactive`, не меняя contract `save_fastlane` и не создавая net-new total runtime/CPU parallelism.
- Явно закрепить owner outer-admission arbiter в diagnostics runtime orchestration layer до branch fan-out applied/shadow/ready/fallback: backend/LSP владеет latest-wins queue, supersession facts, per-file coalescing/fair rotation и выдачей opaque lane-slot/admission token, а facade/runtime helper paths обязаны потреблять этот admission contract и MUST NOT независимо дублировать branch-local arbitration.
- Зафиксировать operator-visible quota model как count end-to-end `didSave_followup` slots, которые охватывают outer admission boundary, writer/runtime preparation, blocking CPU execution и final pre-publish supersession/quota/disposition decision одного heavy follow-up, но MUST NOT включать outbound publish/output wait; remediation MUST NOT раскалывать этот contract на independently configurable writer-vs-CPU quotas.
- Зафиксировать fairness scope этого quota contract: quota считается глобально на процесс для lane `did_save_followup`, но queued work MUST храниться как latest-only per-file entries с fair rotation между distinct files, чтобы same-file save storm не превращался в raw FIFO head-of-line blocker для другого файла и при этом total admitted follow-up work не превышал global quota.
- Зафиксировать default effective quota этого lane равной `1`, чтобы remediation по умолчанию оставалась bounded и не вводила save-storm fan-out без явного operator override.
- Зафиксировать, что простого lane-tag/enum на существующих `Background` queue/permit paths недостаточно: remediation MUST ввести explicit outer admission boundary before existing scarce writer FIFO and blocking CPU permit acquisition, чтобы supersession и `quota=0` могли re-check-иться до потребления scarce resources.
- Потребовать explicit outer admission arbiter/latest-wins queue для didSave follow-up и один end-to-end slot guard, который удерживается от outer admission через writer/runtime preparation, blocking CPU execution и final pre-publish supersession/quota/disposition decision, но освобождается до outbound publish/output wait; raw semaphore/permit без outer arbiter для этого change недостаточен.
- Потребовать, чтобы writer-owned applied-state branch не обходил новый lane contract через direct `snapshot_with_deps` / inline query execution вне lane-aware prepare/admission hooks.
- Зафиксировать explicit latest-wins / stale-shedding contract для этой lane на admission boundary до scarce writer/runtime resources, чтобы older queued follow-up не мог стать новым head-of-line blocker для более нового same-file save cycle.
- Добавить stable runtime-config knob для quota/permits этого lane и зафиксировать, что `0` выключает новые `didSave + idle_heavy` admissions вместо clamp-to-one или silent fallback в generic background, queued-but-not-started work обязана re-check-ить effective quota на admission boundary, а runtime updates apply to future outer-admission decisions instead of retroactively revoking already admitted work.
- Потребовать operator-visible canonical semantics для `quota=0`: heavy follow-up должен завершаться explicit non-cancellation disposition/outcome `disabled_by_config`, а не пропадать молча, и этот outcome должен быть выражен shared terminal disposition/outcome contract, а не trace-only string.
- Потребовать dedicated lane telemetry и saturation metrics сразу, причём в explicit additive schema: canonical lane visibility для этого change оформляется через новую bounded `lane` dimension или semantically equivalent dedicated runtime-lane metric family для queue/exec/saturation signals, где stable lane value `did_save_followup` сохраняется отдельно от legacy `interactive/background` / `work_class`; shared diagnostics terminal taxonomy одновременно расширяется explicit non-cancellation value `disabled_by_config`, и оба additive contracts вводятся в текущем change без ожидания `rewrite-v2-observability-perf-pipeline`.
- Потребовать representative saturation regressions, forced-fallback coverage и save-storm coverage, которые проверяют именно request-centric `followup_runtime_queue_wait_ms`, а не только cumulative process histograms.

## Impact
- Affected specs:
  - `bsl-intellisense`
  - `bsl-intellisense-v2`
  - `bsl-runtime-config`
- Affected code (implementation follow-up):
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `bsl-runtime/src/system/runtime_config.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `bsl-runtime/src/system/basic_observability/core_metrics.rs`
  - `bsl-runtime/src/system/basic_observability/labels.rs`
  - `bsl-runtime/src/system/basic_observability/runtime_metrics.rs`
  - `bsl-runtime/src/system/basic_observability/tests.rs`
  - `bsl-runtime/src/system/system_coordinator/coordinator/observability.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleDiagnosticsSave.ts`
  - `vscode-extension/src/test/suite/customRequests.test.ts`
  - `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`
  - representative perf/live evidence under `backend/tests/perf/reports/`

## Non-Goals
- Не перепроектировать весь writer/runtime scheduler или общую fairness policy для всех background jobs beyond the didSave follow-up path.
- Не переоткрывать UI / VS Code extension investigation без нового прямого контрдоказательства.
- Не оптимизировать generic semantic query cost в heavy follow-up.
- Не лечить все process-wide outlier'ы `apply_change_set_file_exec_ms` в этом change; это отдельный follow-up hardening scope.

## Resolved Assumptions and Open Questions
- Default effective quota для dedicated follow-up lane принимается равной `1`. Это safe-by-default значение: remediation включена из коробки, но остаётся bounded и не добавляет net-new parallel fan-out без явного override.
- Outer admission arbiter закрепляется за `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`, потому что именно там уже живут branch selection, save-cycle identity, supersession facts и wait-state publication; `bsl-runtime` получает downstream lane identity plus opaque admission token/facts и не владеет отдельной branch-local queue policy.
- Quota semantics фиксируются как global process-wide slot count для lane `did_save_followup` с latest-only per-file queue entries и fair rotation между distinct files. Это сохраняет one-slot mental model при default `1`, не даёт одному noisy файлу забить очередь stale work и не вводит per-file multiplicative fan-out.
- Additive observability schema фиксируется уже сейчас: runtime queue/exec/saturation для dedicated lane идут через first-class bounded `lane` surface с canonical value `did_save_followup`, а shared diagnostics terminal taxonomy расширяется значением `disabled_by_config`, которое не считается cancellation reason.
- `rewrite-v2-observability-perf-pipeline` не является blocker для этого change. `refactor-10` вводит минимальный canonical additive lane/outcome contract уже сейчас; будущий observability v3 rewrite должен его сохранить или перенести, а не откладывать remediation.
