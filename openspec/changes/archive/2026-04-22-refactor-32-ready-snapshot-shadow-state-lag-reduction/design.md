## Context

`refactor-31` closed the attribution bug but did not return the representative mixed same-file
profile to exact ready artifacts.

The post-fix incident bundle on `git 8aa12610` still shows:

- `didSave` heavy follow-up publishing through `shadow_state` after `7.0-7.9s`;
- exact timeout still dominated by bounded `program_lowering`;
- ranged `didChange` still falling back through `stale_parser_base` with
  `ready_snapshot_lags_shadow_state`;
- repeated same-file parse workers starting and then being retargeted during `parse_exec`.

This means the remaining problem is no longer “which bucket should observability export?” It is
“how do we keep one exact, parser-base-capable head fresh enough that representative same-file churn
stops defaulting to fallback paths?”

## Goals / Non-Goals

- Goals:
  - return the representative mixed `didChange + didSave` profile to `ready_artifacts` for the
    heavy save follow-up without widening existing budgets;
  - make `ready_snapshot_lags_shadow_state` an exceptional fallback cause rather than the dominant
    steady-state explanation under the target churn profile;
  - preserve exactness, latest-wins supersession, and truthful observability.
- Non-Goals:
  - re-open diagnostics-save coherence work;
  - optimize completion UI/transport for this change;
  - publish stale diagnostics or weaken same-version guarantees.

## Decisions

### 1. Treat the remaining incident as an exact-head freshness problem

The bundle already proves that timeout attribution is coherent and localized. The next change
therefore targets exact-head freshness under same-file churn, not another reporting fix.

### 2. Unify `didSave` follow-up success and ranged `didChange` recovery around one still-current exact head

The representative symptoms are coupled:

- `didSave` heavy follow-up misses because the exact same-version producer still cannot beat the
  fallback window;
- ranged `didChange` misses parser-base reuse because the latest ready head still trails
  `shadow_state`.

The implementation may use producer prioritization, recovery/prime paths, or head publication
changes, but the contract should be framed around keeping one parser-base-capable exact head close
to the live shadow document, not around independently patching each symptom.

### 3. Representative `conf_big` live evidence is mandatory

Synthetic unit coverage is necessary but insufficient here. Acceptance must prove that the
representative mixed same-file profile no longer defaults to `shadow_state` and no longer exports
`ready_snapshot_lags_shadow_state` as the steady-state didChange explanation.

## Risks / Trade-offs

- Favoring one still-current exact head too aggressively can starve newer revisions unless
  latest-wins rechecks remain strict.
- More aggressive parser-base reuse can accidentally smuggle stale state unless version/text
  binding remains explicit.
- A fix that only reduces retarget churn in synthetic tests but not on representative live load
  would not satisfy this change.

## Validation Plan

- Keep targeted regressions for:
  - exact save follow-up publishing through `ready_artifacts` on the representative same-file
    profile;
  - ranged `didChange` no longer defaulting to `stale_parser_base` from
    `ready_snapshot_lags_shadow_state` under the same representative churn family.
- Refresh the representative `conf_big` live report / incident bundle and use it as the primary
  acceptance asset.
