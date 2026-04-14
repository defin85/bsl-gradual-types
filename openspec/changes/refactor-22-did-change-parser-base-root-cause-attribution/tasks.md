## 1. Root-cause contract

- [ ] 1.1 Add low-cardinality root-cause attribution for ranged `didChange` transitions that end in
      `fallback_reason=stale_parser_base`.
- [ ] 1.2 Extend didChange parse-snapshot evidence / incident-bundle payloads with bounded
      shadow-vs-ready base state so the miss class is explainable without raw logs.

## 2. Regressions and evidence

- [ ] 2.1 Add backend regressions that distinguish at least:
      `ready_snapshot_lags_shadow_state`, `no_matching_ready_snapshot_for_shadow_state`, and
      `tree_cache_mismatch_after_prime`.
- [ ] 2.2 Capture representative repo-local evidence proving the new attribution fields stay
      truthful on real `conf_big` churn.

## 3. Validation

- [ ] 3.1 Run targeted backend tests covering the new miss taxonomy and payload export.
- [x] 3.2 Run `openspec validate refactor-22-did-change-parser-base-root-cause-attribution --strict --no-interactive`.

## 4. OpenSpec / Beads Sync

- [x] 4.1 Keep Beads epic `bsl-gradual-types-wikt` and child `bsl-gradual-types-wikt.1`
      aligned with the current plan/status of this change.
- [x] 4.2 Keep `validation/epic-summary.md` aligned with the actual cross-change sequencing and
      Beads dependency graph for this epic.
