# intellisense-perf-gate v1

Initial contract baseline for dedicated perf-gate evaluator (`Option B`).

## Scope
- Input contract for profiles and required metric keys.
- Baseline contract for absolute latency ceilings.
- Baseline contract for resource budget ceilings.
- Baseline bootstrap policy (`required_profiles`, `sample_size_min>=5`, `aggregation_rule=median`).
- Report contract for deterministic verdict/reason-codes.

## 2026-03-03
- Added optional provenance fields for `v1` reports:
  - `change_id`
  - `generated_at`
  - `profile`
  - `schema_version`
  - `contract_version`
- Added authoritative `change_id` source priority:
  1. `--change-id`
  2. `OPENSPEC_CHANGE_ID`
- Added fail-closed evaluator reason codes:
  - `provenance_missing_for_authoritative_run`
  - `provenance_mismatch_expected_change_id`
  - `provenance_invalid`
  - `provenance_non_authoritative_cutover_evidence`
  - `parity_evidence_insufficient`
  - `parity_drift_threshold_exceeded`
