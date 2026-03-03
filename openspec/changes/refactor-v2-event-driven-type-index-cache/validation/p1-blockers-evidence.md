# P1 Blockers Evidence (type_index taxonomy + observability)

## Scope
- change_id: `refactor-v2-event-driven-type-index-cache`
- date (UTC): `2026-03-03T11:42:07Z`

## Contract/Test Evidence
- `cargo test -p bsl-analysis-v2 type_index_reason_code_strings_match_contract -- --nocapture` -> `ok`
- `cargo test -p bsl-analysis-v2 apply_change_reports_type_index_invalidation_effects -- --nocapture` -> `ok`
- `cargo test -p bsl-runtime runtime_queue_and_exec_projection_do_not_raise_hint_mismatch -- --nocapture` -> `ok`
- `cargo test -p bsl-runtime type_index_reason_metrics_are_exported_with_bounded_reasons -- --nocapture` -> `ok`
- `cargo test -p bsl-runtime observability_completion_v1_contract_matches_runtime_metric_labels -- --nocapture` -> `ok`
- `cargo test -p bsl-backend p6_type_index_precompute_slot_tracks_latest_version_and_clears_on_did_close -- --nocapture` -> `ok`
- `cargo test -p bsl-backend p7_completion_owner_hint_type_lookup_is_serve_only_even_when_flow_sensitive_enabled -- --nocapture` -> `ok`
- `python3 scripts/check-versioned-contracts.py` -> `Versioned contracts policy check passed.`

## Separate Perf Run Evidence
Generated with:
- `cargo run -p bsl-backend --bin intellisense_perf -- ... --scenario backend/tests/perf/scenarios/intellisense_small.json ...`
- `cargo run -p bsl-backend --bin intellisense_perf -- ... --scenario backend/tests/perf/scenarios/intellisense_large.json ...`
- `cargo run -p bsl-backend --bin intellisense_perf -- ... --scenario backend/tests/perf/scenarios/intellisense_churn.json ...`

Artifacts (ignored by git via `.gitignore`, retained locally):
- `backend/tests/perf/reports/refactor-v2-event-driven-type-index-cache-small.json`
- `backend/tests/perf/reports/refactor-v2-event-driven-type-index-cache-large.json`
- `backend/tests/perf/reports/refactor-v2-event-driven-type-index-cache-churn.json`
- `backend/tests/perf/reports/refactor-v2-event-driven-type-index-cache-gate.json`
- `backend/tests/perf/reports/refactor-v2-event-driven-type-index-cache-gate.md`

Gate summary (`refactor-v2-event-driven-type-index-cache-gate.json`):
- `verdict`: `pass`
- `pass`: `true`
- `reason_codes`: `[]`

## OpenSpec Validate Evidence
- Command: `openspec validate refactor-v2-event-driven-type-index-cache --strict --no-interactive`
- Result: `Change 'refactor-v2-event-driven-type-index-cache' is valid`
- Raw log (ignored by git due `*.log`): `openspec/changes/refactor-v2-event-driven-type-index-cache/validation/refactor-v2-event-driven-type-index-cache-openspec-validate.log`
