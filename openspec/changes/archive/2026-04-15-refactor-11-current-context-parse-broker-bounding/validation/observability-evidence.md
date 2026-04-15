# Observability Evidence

## Goal

Close task `2.4` with concrete evidence that `bsl.getCurrentContext` requests now prefer:

- `ready_snapshot` when a same-revision parse artifact already exists;
- one brokered leader parse plus follower reuse for same-key bursts;
- bounded empty or superseded outcomes instead of extra independent parse holders.

## Evidence

- Ready snapshot smoke:
  [refactor-11-current-context-parse-broker-bounding-ready-snapshot-smoke.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-11-current-context-parse-broker-bounding-ready-snapshot-smoke.json)
- Broker burst smoke:
  [refactor-11-current-context-parse-broker-bounding-burst-smoke.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-11-current-context-parse-broker-bounding-burst-smoke.json)

## Read Snapshot

- Source test:
  `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_uses_parse_snapshot_without_warming_exact_type_index -- --nocapture`
- Expected signal:
  `parse_attempts = 0`
  `ready_snapshot_role_total = 1`
  `ready_snapshot_source_total = 1`
  `broker_leader_total = 0`
  `broker_follower_total = 0`

This proves that a same-revision request can resolve from the ready parse snapshot without launching any auxiliary blocking parse holder.

## Broker Burst

- Source test:
  `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_same_revision_burst_shares_one_broker_leader_before_blocking -- --nocapture`
- Expected signal:
  `parse_attempts = 1`
  `broker_leader_total = 1`
  `broker_follower_total = 1`
  `resolved_total = 2`
  `parser_coordinator_source_total = 2`

Interpretation:

- `parse_attempts` counts actual blocking parse holders.
- `parser_coordinator_source_total` counts served requests that observed parser-coordinator as their source.
- Therefore `parser_coordinator_source_total = 2` with `parse_attempts = 1` is the intended brokered shape: two requests consumed one shared parse result instead of spawning two independent parse holders.

## Conclusion

Together these reports show the expected shift away from repeated parser-coordinator parse holders:

- hot same-revision path resolves through `ready_snapshot` with zero auxiliary parse attempts;
- same-key burst path resolves through one broker leader and follower reuse rather than N blocking parse holders.
