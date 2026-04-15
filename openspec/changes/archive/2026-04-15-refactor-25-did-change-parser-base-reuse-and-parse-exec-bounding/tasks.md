## 1. Parser-base reuse recovery

- [x] 1.1 Add a bounded recovery/prime path for ranged `didChange` when
      `stale_parser_base` is caused by `ready_snapshot_lags_shadow_state`, so the runtime does not
      immediately pay for a full parse from `shadow_state` in that miss class.
- [x] 1.2 Preserve truthful fallback attribution when recovery still cannot produce a matching
      parser base, without widening same-version exactness rules.

## 2. Parse-exec waste bounding

- [x] 2.1 Add additional retarget/cancel observation points inside the expensive exact
      parse/build path so same-file obsolete work can terminate during `parse_exec`.
- [x] 2.2 Distinguish parse-exec aborts from late post-parse/materialization losses in lifecycle
      attribution and cumulative metrics.

## 3. Regressions and live evidence

- [x] 3.1 Add backend regressions for:
      bounded parser-base recovery success,
      truthful fallback when recovery still fails,
      and retarget-during-parse abort on same-file churn.
- [x] 3.2 Capture representative repo-local live evidence on `examples/conf_big` showing whether
      the mixed `didChange + didSave` incident path returns to `ready_artifacts` or, if not, what
      bounded root cause remains.

## 4. Validation

- [x] 4.1 Run targeted backend tests covering parser-base recovery, parse-exec abort attribution,
      and the relevant `didSave` follow-up path.
- [x] 4.2 Run `openspec validate refactor-25-did-change-parser-base-reuse-and-parse-exec-bounding --strict --no-interactive`.

## 5. OpenSpec / Beads Sync

- [x] 5.1 Keep Beads epic `bsl-gradual-types-qtm3` and children
      `bsl-gradual-types-qtm3.1` through `bsl-gradual-types-qtm3.4` aligned with the actual
      implementation status and dependency graph of this change.
