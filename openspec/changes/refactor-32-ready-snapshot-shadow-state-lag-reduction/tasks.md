## 1. Exact head freshness for `didSave`

- [ ] 1.1 Rework same-file exact ready-snapshot producer policy so a still-current `didSave`
      heavy follow-up can publish through `ready_artifacts` on the representative bounded
      `program_lowering` profile without widening the existing wait budgets.
- [ ] 1.2 Preserve truthful latest-wins supersession, cancellation, and fallback semantics when a
      newer same-file revision or save cycle overtakes the current target.

## 2. Parser-base lag reduction for ranged `didChange`

- [ ] 2.1 Keep a parser-base-capable exact head close enough to `shadow_state` during ranged
      same-file churn so `ready_snapshot_lags_shadow_state` stops being the dominant steady-state
      reason for `stale_parser_base` on representative large-module profiles.
- [ ] 2.2 Bound retarget/restart waste so the newest same-file revision prefers advancing one
      still-current exact head or bounded recovery path over repeatedly starting obsolete parse
      workers that will be retargeted during `parse_exec`.

## 3. Evidence and regressions

- [ ] 3.1 Add targeted backend/runtime regressions covering representative same-file
      `didChange + didSave` churn, exact follow-up publish through `ready_artifacts`, and the
      reduced incidence of `stale_parser_base / ready_snapshot_lags_shadow_state`.
- [ ] 3.2 Refresh representative `conf_big` live evidence / incident-bundle proof showing that the
      target profile no longer defaults to `shadow_state` for the heavy `didSave` follow-up and no
      longer treats `ready_snapshot_lags_shadow_state` as the steady-state `didChange` explanation.

## 4. Validation

- [ ] 4.1 Run targeted backend tests and representative live repros for the new exact-head
      freshness contract.
- [ ] 4.2 Run `openspec validate refactor-32-ready-snapshot-shadow-state-lag-reduction --strict
      --no-interactive`.
