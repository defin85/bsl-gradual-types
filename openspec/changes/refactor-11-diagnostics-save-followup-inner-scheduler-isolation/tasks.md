## 1. Implementation
- [ ] 1.1 Introduce an opaque inner execution entitlement for admitted `did_save_followup` work, carved from the existing bounded non-interactive budget without borrowing interactive reserved capacity and without increasing total runtime/CPU parallelism.
- [ ] 1.2 Make diagnostics-runtime outer admission own that entitlement end to end, from outer admission through writer/runtime preparation, blocking CPU execution, and the final pre-publish supersession/disposition decision, still releasing it before outbound publish/output wait.
- [ ] 1.3 Teach the existing writer/runtime scheduler to prioritize admitted `did_save_followup` prepare work ahead of generic background backlog without forking a second writer scheduler or losing the current interactive priority contract.
- [ ] 1.4 Ensure admitted `did_save_followup` blocking stages consume the reserved entitlement instead of re-entering the generic `Background` CPU permit wait path, while generic background work continues to use the generic background path.
- [ ] 1.5 Preserve current additive telemetry, binary `CpuWorkClass`, runtime-config quota semantics, `disabled_by_config`, and request-centric save timeline contracts.

## 2. Validation
- [ ] 2.1 Add deterministic regressions proving an admitted `did_save_followup` no longer waits behind unrelated generic background blocking CPU holders as its default primary gate.
- [ ] 2.2 Add deterministic regressions proving admitted didSave follow-up prepare work does not sit behind generic background writer backlog once the outer lane has already admitted it.
- [ ] 2.3 Add deterministic regressions covering a real generic competitor such as `bsl.getCurrentContext`, proving it cannot consume the reserved didSave-follow-up inner entitlement by default.
- [ ] 2.4 Re-run existing `refactor-10` regressions to confirm no regression in first publish, `quota=0`, telemetry shape, or latest-wins semantics.
- [ ] 2.5 Capture representative `conf_big` live evidence showing bounded first publish and improved follow-up `runtime_queue_wait` after inner-scheduler isolation under comparable mixed load.
- [ ] 2.6 Run `openspec validate refactor-11-diagnostics-save-followup-inner-scheduler-isolation --strict --no-interactive`.
