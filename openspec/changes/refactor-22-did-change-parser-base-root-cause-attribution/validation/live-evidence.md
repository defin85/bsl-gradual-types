# Live Evidence

## Commands

- `cargo test -p bsl-backend --bin bsl-lsp-server p22_did_change_ -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p22_get_observability_metrics_exposes_did_change_parse_snapshot_evidence -- --nocapture`
- `CHANGE_ID=refactor-22-did-change-parser-base-root-cause-attribution BSL_V2_REAL_CONF_BIG_STALE_PARSER_BASE_ROOT_CAUSE_REPORT=/home/egor/code/bsl-gradual-types/openspec/changes/refactor-22-did-change-parser-base-root-cause-attribution/validation/refactor-22-real-conf-big-stale-parser-base-root-cause-live.json cargo test -p bsl-backend --bin bsl-lsp-server p49_real_conf_big_stale_parser_base_root_cause_report_live -- --nocapture`

## Evidence

- Synthetic regressions now separate the three stale parser-base miss classes without raw logs:
  - `ready_snapshot_lags_shadow_state`
  - `no_matching_ready_snapshot_for_shadow_state`
  - `tree_cache_mismatch_after_prime`
- Real `conf_big` churn evidence is checked in here:
  [refactor-22-real-conf-big-stale-parser-base-root-cause-live.json](/home/egor/code/bsl-gradual-types/openspec/changes/refactor-22-did-change-parser-base-root-cause-attribution/validation/refactor-22-real-conf-big-stale-parser-base-root-cause-live.json)
  - `fallback_reason=stale_parser_base`
  - `parser_base_root_cause=ready_snapshot_lags_shadow_state`
  - `shadow_document_version=3`
  - `latest_ready_document_version=1`
  - `matching_ready_snapshot_for_shadow_state=false`
  - `ready_snapshot_prime_attempted=false`

## Interpretation

- The checked-in `conf_big` artifact confirms the real incident shape this change was meant to
  explain: same-file churn advanced `shadow_state` beyond the latest ready snapshot, so the ranged
  `didChange` exact path fell back to `stale_parser_base` full parse for a bounded, explicit
  reason.
- Operators no longer need raw backend logs to tell whether the miss was caused by ready lag, by
  complete absence of a matching ready base, or by a post-prime tree-cache mismatch.
