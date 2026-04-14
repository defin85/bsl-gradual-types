# Change: isolate `didSave` exact follow-up publish from post-parse apply lag

## Why

`refactor-25` закрыл parser-base recovery и during-parse abort bounding, но live `conf_big`
evidence показал следующий residual:

- mixed `didChange + didSave` path всё ещё может завершаться через `shadow_state`;
- bounded wait по exact producer всё ещё истекает на `parse_exec`;
- после этого timeline уже показывает `apply_lag` как residual blocker, а relief valve truthfully
  уходит в `skipped_apply_lag`;
- checked-in live report не получает same-version `didChange` evidence для final revision в окне
  наблюдения, хотя первичный parser-base root cause уже снят.

Иными словами, следующий bottleneck сместился в exact post-parse / publish path: нужно отделить
случай "writer/apply ещё отстаёт" от случая "exact ready artifacts уже доказаны и могут быть
использованы", не ослабляя exactness semantics.

## What Changes

- Require `didSave` heavy follow-up to prefer exact same-version ready artifacts even when
  writer-owned apply still lags, если runtime уже может доказать matching current
  `(file_id, requested_version, text_hash)`.
- Require bounded attribution that distinguishes:
  - `apply_lag` до появления usable exact ready artifacts;
  - post-parse / post-ready publish gating, когда exact artifacts уже есть, но follow-up publish
    ещё не состоялся.
- Require regressions and repo-local live evidence that show whether the mixed `conf_big`
  incident path returns to `ready_artifacts`, or, if not, what new bounded blocker remains after
  parser-base fixes.

## Sequence

This change intentionally follows:

- `refactor-24-diagnostics-save-followup-budget-valve`
- `refactor-25-did-change-parser-base-reuse-and-parse-exec-bounding`

`refactor-24` added the temporary valve and truthful skip reasons.
`refactor-25` removed the stale parser-base root cause and bounded during-parse waste.
This change targets the next residual: apply-lag / publish gating after exact parse work.

## Epic

This change is tracked by Beads epic `bsl-gradual-types-lyr2`
(`OpenSpec refactor-26: didSave exact publish apply-lag isolation`).

Execution children for this step:

- `bsl-gradual-types-lyr2.1` - distinguish post-ready publish gating from `apply_lag`
- `bsl-gradual-types-lyr2.2` - publish exact follow-up from ready artifacts despite delayed apply
- `bsl-gradual-types-lyr2.3` - regressions and `conf_big` live evidence
- `bsl-gradual-types-lyr2.4` - targeted validation and strict OpenSpec validation

Dependency graph:

- `bsl-gradual-types-lyr2.1` starts first;
- `bsl-gradual-types-lyr2.2` depends on `.1`;
- `bsl-gradual-types-lyr2.3` depends on `.2`;
- `bsl-gradual-types-lyr2.4` depends on `.3`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - ready-snapshot apply / publish wiring around same-version diagnostics follow-up
  - diagnostics save timeline / observability export / `conf_big` live evidence

## Non-Goals

- Do not widen the base `didSave` bounded wait or the temporary relief valve as the primary fix.
- Do not relax exact same-version semantics or permit older-version diagnostics to publish.
- Do not revisit parser-base recovery or during-parse abort logic from `refactor-25` unless a new
  blocker proves those paths were misattributed.
