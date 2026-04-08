## 1. Implementation
- [ ] 1.1 Rework post-`save_fastlane` `didSave + idle_heavy` scheduling so shared
      interactive/runtime contention is not its default primary gate.
- [ ] 1.2 Extend diagnostics save timeline with explicit follow-up runtime contention facts and keep
      server/client DTOs in sync.
- [ ] 1.3 Update incident bundle projection so request-centric follow-up blocker breakdown is shown
      directly in summary output.

## 2. Validation
- [ ] 2.1 Add regressions for queued follow-up contention, apply-change contention, and truthful
      in-flight/terminal follow-up attribution.
- [ ] 2.2 Capture and check in live `conf_big` evidence as
      `p46_real_conf_big_did_save_diagnostics_followup_runtime_report_live`.
- [ ] 2.3 Run `openspec validate refactor-09-diagnostics-save-followup-runtime-contention-bounding --strict --no-interactive`.
