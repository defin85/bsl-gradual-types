## 1. Contract

- [x] 1.1 Define the detached diagnostics-ready artifact contract for same-version `didSave`
      follow-up, including target identity, diagnostics-only scope, truthful observability, and
      the boundary versus `refactor-43` parser-base / timeout-leaf-fidelity work.
- [x] 1.2 Preserve the existing live exact-readiness and fail-closed contract for interactive
      exact consumers so detached diagnostics artifacts never become silent substitutes.

## 2. Implementation

- [x] 2.1 Add publication of a detached diagnostics-ready artifact on the same-version `didSave`
      path after bounded diagnostics-ready build is complete but before live `ready_install`
      becomes the primary gate.
- [x] 2.2 Teach `didSave` heavy follow-up and its request-centric diagnostics path to consume the
      detached artifact while the target remains still-current, instead of defaulting to terminal
      `shadow_state` solely because live exact install is still pending.
- [x] 2.3 Preserve latest-wins supersession, cancellation, and truthful fallback when a newer
      same-file revision or newer save cycle overtakes the target or detached proof is exhausted.
- [x] 2.4 Preserve or extend operator-facing telemetry / incident evidence so detached
      diagnostics-ready success is distinguishable from canonical live `ready_artifacts` and from
      degraded `shadow_state` fallback.

## 3. Regressions and evidence

- [x] 3.1 Add backend regressions for the representative `ready_install`-dominated same-version
      `didSave` family, proving detached diagnostics-ready artifacts unblock the heavy follow-up.
      Treat `p55` / `p56`-style save-followup traces as the representative acceptance surface;
      legacy `p53` is optional diagnostic coverage only.
- [x] 3.2 Add regressions proving `hover`, `definition`, `signatureHelp`, `type-at-position`, and
      semantically equivalent exact consumers still remain on the canonical live exact gate.
- [x] 3.3 Refresh representative live evidence for the save-followup family and capture the new
      detached-artifact terminal path without regressing initial `didOpen`/same-version readiness
      or re-opening `parser_base_recovery` dominance already owned by `refactor-43`. Do not block
      this change on `p53_real_conf_big_exact_program_lowering_report_live` unless the residual
      clearly regresses back into `program_lowering`.

## 4. Validation

- [x] 4.1 Run targeted backend/runtime regressions for detached diagnostics-ready publication and
      preserved live exact fail-closed behavior.
- [x] 4.2 Run `openspec validate refactor-44-save-followup-detached-ready-artifacts --strict
      --no-interactive`.
