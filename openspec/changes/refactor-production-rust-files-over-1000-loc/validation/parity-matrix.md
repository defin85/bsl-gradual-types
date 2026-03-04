# Parity Validation Matrix (behavior-preserving)

Date: 2026-03-04  
Change: `refactor-production-rust-files-over-1000-loc`

## Purpose

This matrix defines the minimum reproducible validation set per batch to ensure
behavior parity (no intentional contract/semantic changes) during large-file
decomposition.

## Global gates (all batches)

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `python3 scripts/check-versioned-contracts.py`
4. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py`

Pass criteria:

- all commands exit with `0`
- no contract policy violations
- no file budget violations introduced outside approved target trajectory

## Batch matrix

### Batch A (LSP/Web + perf-gate runtime binary files)

Scope:

- `backend/src/bin/lsp_server/**`
- `backend/src/presentation/web/handlers.rs`
- `backend/src/bin/intellisense_perf.rs`
- `backend/src/perf_gate_evaluator.rs`

Checks:

1. `cargo test -p bsl-backend --locked`
2. `./scripts/run-intellisense-tests.sh smoke`

Pass criteria:

- backend tests pass
- smoke IntelliSense tests pass

### Batch B (Runtime services/observability)

Scope:

- `bsl-runtime/src/system/basic_observability.rs`
- `bsl-runtime/src/application/type_system/services/completion_service.rs`
- `bsl-runtime/src/application/type_system/services/completion_ranking.rs`
- `bsl-runtime/src/application/intellisense_v2/{facade.rs,policy.rs}`

Checks:

1. `cargo test -p bsl-runtime --locked`
2. `cargo test -p bsl-backend --locked` (consumer parity)

Pass criteria:

- runtime crate tests pass
- backend consumer tests pass

### Batch C (Runtime coordinator/loaders)

Scope:

- `bsl-runtime/src/system/system_coordinator/{config_loader.rs,lifecycle.rs,coordinator.rs}`
- `bsl-runtime/src/system/{disk_cache.rs,runtime_config.rs,parser_coordinator.rs}`
- `bsl-runtime/src/data/loaders/config_metadata_parser/{discovery.rs,converter.rs}`

Checks:

1. `cargo test -p bsl-runtime --locked`
2. `cargo test -p bsl-backend --locked`

Pass criteria:

- runtime coordinator/loader behavior unchanged for consumers

### Batch D (Analysis/Semantic/Repository)

Scope:

- `analysis-v2/src/{lib.rs,type_inference_v2.rs}`
- `semantic-diagnostics/src/visitor.rs`
- `bsl-repository/src/repository.rs`

Checks:

1. `cargo test -p bsl-analysis-v2 --locked`
2. `cargo test -p bsl-diagnostics --locked`
3. `cargo test -p bsl-repository --locked`
4. `cargo test -p bsl-backend --locked` (integration consumer)

Pass criteria:

- analysis/diagnostics/repository behavior unchanged for backend integrations

### Batch E (Agent)

Scope:

- `bsl-agent/src/{session/mod.rs,server/mod.rs}`

Checks:

1. `cargo test -p bsl-agent --locked`
2. `cargo test -p bsl-backend --locked` (cross-surface safety)

Pass criteria:

- agent tests pass
- backend parity unaffected

## Final validation

1. `cargo test --workspace --locked`
2. `openspec validate refactor-production-rust-files-over-1000-loc --strict --no-interactive`

Pass criteria:

- workspace and OpenSpec validation pass
- no unresolved deltas against contracts/policies

## Progress evidence (Batch A partial)

Executed on 2026-03-04 for tasks `2.1` and `2.6`:

1. `cargo test -p bsl-backend --locked --bin intellisense_perf` -> passed
2. `cargo test -p bsl-backend --locked perf_gate_evaluator` -> passed
3. `cargo test -p bsl-backend --locked --bin bsl-lsp-server --no-run` -> passed
4. `cargo test -p bsl-backend --locked --bin bsl-lsp-server diagnostics_debounce_floor_prevents_zero_ms_tight_loops` -> passed
5. `cargo test -p bsl-backend --locked --bin bsl-lsp-server p9a_formatting_disabled_does_not_advertise_capability_and_returns_null` -> passed
6. `cargo test -p bsl-backend --locked --bin bsl-lsp-server queue_coalesces_did_change_to_latest_revision` -> passed
7. `cargo test -p bsl-backend --locked --bin bsl-lsp-server queue_capacity_update_applies_to_existing_dispatchers` -> passed
8. `cargo test -p bsl-backend --locked --bin bsl-lsp-server --no-run` -> passed
9. `cargo test -p bsl-backend --locked --bin bsl-web-server --no-run` -> passed
10. `cargo test -p bsl-backend --locked --bin bsl-lsp-server completion::tests:: -- --list` -> passed
11. `cargo test -p bsl-backend --locked --bin bsl-lsp-server metadata_completion_kinds_have_unique_lsp_kinds` -> passed
12. `cargo test -p bsl-backend --locked --bin bsl-lsp-server --no-run` -> passed (after `language_server.rs` split)
13. `cargo test -p bsl-backend --locked --bin bsl-lsp-server idle_heavy_runs_for_save_trigger_even_when_flow_sensitive_disabled` -> passed
14. `cargo test -p bsl-backend --locked --bin bsl-lsp-server completion_routing_plan_follows_mode_contract` -> passed

## Progress evidence (Batch A parity execution for task `2.7`)

Executed on 2026-03-04:

1. `cargo test -p bsl-backend --locked` -> failed (known baseline fail set).
2. `./scripts/run-intellisense-tests.sh smoke` -> failed (known baseline fail set).

Baseline comparison performed in clean detached worktree at `HEAD` (`bd446f7`):

1. `cargo test -p bsl-backend --locked --bin bsl-lsp-server handlers::hover::tests::m5_hover_v2_is_deterministic -- --nocapture` -> failed in baseline and in refactor branch.
2. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p24_real_scenario_observability_stage_parity_lsp_vs_mcp -- --nocapture` -> failed in baseline and in refactor branch.
3. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools -- --nocapture` -> failed in baseline and in refactor branch.
4. `cargo test -p bsl-backend --locked --test m8_completion_matrix_golden_v2_test -- --nocapture` -> failed in baseline and in refactor branch.

Conclusion for Batch A parity in this change:

- no new failures introduced by Batch A decomposition in validated fail set;
- failing tests are pre-existing and tracked as baseline drift outside this refactor scope.

## Progress evidence (Batch C task `4.3` decomposition)

Executed on 2026-03-04:

1. `cargo test -p bsl-runtime --locked config_metadata_parser::converter::tests:: -- --list` -> passed (test module migration compile/list check).
2. `cargo test -p bsl-runtime --locked config_metadata_parser::converter::tests::test_convert_catalog_to_raw_type` -> passed.
3. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report openspec/changes/refactor-production-rust-files-over-1000-loc/validation/rust-llm-budget-progress-batch-a-2026-03-04.json --json` -> executed; hard LOC violations reduced `20 -> 18` after decomposing:
   - `bsl-runtime/src/data/loaders/config_metadata_parser/discovery.rs` (`1016 -> 845`)
   - `bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs` (`1006 -> 608`)

## Progress evidence (Batch C task `4.2` decomposition)

Executed on 2026-03-04:

1. `cargo test -p bsl-runtime --locked --no-run` -> passed after refactor.
2. `cargo test -p bsl-runtime --locked system::runtime_config::tests::registry_has_unique_names` -> passed.
3. `cargo test -p bsl-runtime --locked system::disk_cache::tests::test_disk_cache_roundtrip` -> passed.
4. `cargo test -p bsl-runtime --locked system::parser_coordinator::tests::symbol_index_tests::collect_symbol_items_from_program` -> passed.
5. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report openspec/changes/refactor-production-rust-files-over-1000-loc/validation/rust-llm-budget-progress-batch-a-2026-03-04.json --json` -> executed; hard LOC violations reduced `18 -> 15` after decomposing:
   - `bsl-runtime/src/system/disk_cache.rs` (`1508 -> 995`)
   - `bsl-runtime/src/system/runtime_config.rs` (`1392 -> 990`)
   - `bsl-runtime/src/system/parser_coordinator.rs` (`1329 -> 961`)

## Progress evidence (Batch C task `4.1` decomposition)

Executed on 2026-03-04:

1. `cargo test -p bsl-runtime --locked --no-run` -> passed after splitting `config_loader/lifecycle/coordinator`.
2. `cargo test -p bsl-runtime --locked system::system_coordinator::config_loader::helpers::tests::` -> passed (`15 passed`) after moving tests to `config_loader/helpers/tests.rs`.
3. `cargo test -p bsl-runtime --locked system::system_coordinator::lifecycle::tests::` -> passed (`8 passed`) after moving tests to `lifecycle/tests.rs`.
4. `cargo test -p bsl-runtime --locked system::system_coordinator::coordinator::tests::` -> passed (`2 passed`) after moving tests to `coordinator/tests.rs`.
5. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report openspec/changes/refactor-production-rust-files-over-1000-loc/validation/rust-llm-budget-progress-batch-a-2026-03-04.json --json` -> executed; hard LOC violations reduced `15 -> 12` after decomposing:
   - `bsl-runtime/src/system/system_coordinator/config_loader.rs` (`2262 -> 857`)
   - `bsl-runtime/src/system/system_coordinator/lifecycle.rs` (`1285 -> 973`)
   - `bsl-runtime/src/system/system_coordinator/coordinator.rs` (`1190 -> 347`)

## Progress evidence (Batch C task `4.4` parity execution)

Executed on 2026-03-04:

1. `cargo test -p bsl-runtime --locked` -> passed (`380 passed`, `0 failed`; doc-tests `10 passed`).
2. `cargo test -p bsl-backend --locked --no-run` -> passed (all backend test binaries compiled).
3. `cargo test -p bsl-backend --locked --bin bsl-lsp-server handlers::hover::tests::m5_hover_v2_is_deterministic -- --nocapture` -> failed in current workspace.
4. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p24_real_scenario_observability_stage_parity_lsp_vs_mcp -- --nocapture` -> failed in current workspace.
5. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools -- --nocapture` -> failed in current workspace.
6. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p7_large_churn_budget_timeout_serves_cached_stale_fastpath -- --nocapture` -> passed in current workspace.
7. Baseline comparison in clean detached worktree at `HEAD` (`bd446f7`, path `/tmp/bsl-gradual-types-baseline-c`) with identical command set:
   - `m5_hover_v2_is_deterministic` -> failed
   - `p24_real_scenario_observability_stage_parity_lsp_vs_mcp` -> failed
   - `p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools` -> failed
   - `p7_large_churn_budget_timeout_serves_cached_stale_fastpath` -> passed

Conclusion for Batch C parity in this change:

- fail-set for targeted backend parity checks matches baseline exactly;
- no new regressions introduced by Batch C decomposition in validated parity set;
- remaining failing backend tests are baseline drift outside current refactor scope.

## Progress evidence (Batch B task `3.1` decomposition)

Executed on 2026-03-04:

1. `cargo test -p bsl-runtime --locked --no-run` -> passed after splitting `basic_observability.rs`.
2. `cargo test -p bsl-runtime --locked system::basic_observability::observability_contract_tests:: -- --list` -> passed (module wiring check after test extraction).
3. `cargo test -p bsl-runtime --locked system::basic_observability::observability_contract_tests::` -> passed (`23 passed`).
4. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report openspec/changes/refactor-production-rust-files-over-1000-loc/validation/rust-llm-budget-progress-batch-a-2026-03-04.json --json` -> executed; hard LOC violations reduced `12 -> 11` after decomposing:
   - `bsl-runtime/src/system/basic_observability.rs` (`5616 -> 980`)

## Progress evidence (Batch B task `3.2` decomposition)

Executed on 2026-03-04:

1. `cargo test -p bsl-runtime --locked --no-run` -> passed after splitting `completion_service.rs` and `completion_ranking.rs`.
2. `cargo test -p bsl-runtime --locked application::type_system::services::completion_ranking::tests::` -> passed (`25 passed`).
3. `cargo test -p bsl-runtime --locked application::type_system::services::completion_service::tests::` -> passed (`41 passed`).
4. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report openspec/changes/refactor-production-rust-files-over-1000-loc/validation/rust-llm-budget-progress-batch-a-2026-03-04.json --json` -> executed; hard LOC violations reduced `11 -> 9` after decomposing:
   - `bsl-runtime/src/application/type_system/services/completion_service.rs` (`4888 -> 872`)
   - `bsl-runtime/src/application/type_system/services/completion_ranking.rs` (`1149 -> 434`)

## Progress evidence (Batch B task `3.3` decomposition)

Executed on 2026-03-04:

1. `cargo test -p bsl-runtime --locked --no-run` -> passed after splitting `facade.rs` and `policy.rs`.
2. `cargo test -p bsl-runtime --locked application::intellisense_v2::facade::tests::` -> passed (`29 passed`).
3. `cargo test -p bsl-runtime --locked application::intellisense_v2::policy::tests::` -> passed (`21 passed`).
4. `rg -n '^\\s*mod\\s+[A-Za-z0-9_]*tests\\s*\\{' bsl-runtime/src/application/intellisense_v2/facade.rs bsl-runtime/src/application/intellisense_v2/policy.rs` -> no matches (inline tests removed from production files).
5. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report openspec/changes/refactor-production-rust-files-over-1000-loc/validation/inventory.md --json` -> executed; hard LOC violations reduced `9 -> 7` after decomposing:
   - `bsl-runtime/src/application/intellisense_v2/facade.rs` (`3279 -> 429`)
   - `bsl-runtime/src/application/intellisense_v2/policy.rs` (`1476 -> 785`)

## Progress evidence (Batch B task `3.4` parity execution)

Executed on 2026-03-04:

1. `cargo test -p bsl-runtime --locked` -> passed (`380 passed`, `0 failed`; doc-tests `10 passed`).
2. `cargo test -p bsl-backend --locked --no-run` -> passed (all backend test binaries compiled).
3. `cargo test -p bsl-backend --locked --bin bsl-lsp-server handlers::hover::tests::m5_hover_v2_is_deterministic -- --nocapture` -> failed in current workspace.
4. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p24_real_scenario_observability_stage_parity_lsp_vs_mcp -- --nocapture` -> failed in current workspace.
5. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools -- --nocapture` -> failed in current workspace.
6. `cargo test -p bsl-backend --locked --test m8_completion_matrix_golden_v2_test -- --nocapture` -> failed in current workspace.
7. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p7_large_churn_budget_timeout_serves_cached_stale_fastpath -- --nocapture` -> passed in current workspace.
8. Baseline comparison in clean detached worktree at `HEAD` (`bd446f7`, path `/tmp/bsl-gradual-types-baseline-c`) with identical command set:
   - `m5_hover_v2_is_deterministic` -> failed
   - `p24_real_scenario_observability_stage_parity_lsp_vs_mcp` -> failed
   - `p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools` -> failed
   - `m8_completion_matrix_golden_v2_test` -> failed
   - `p7_large_churn_budget_timeout_serves_cached_stale_fastpath` -> passed

Conclusion for Batch B parity in this change:

- fail-set for validated backend parity checks matches baseline exactly;
- no new regressions introduced by Batch B decomposition in validated parity set;
- remaining failing backend tests are baseline drift outside current refactor scope.

## Progress evidence (Batch D task `5.2` decomposition)

Executed on 2026-03-04:

1. `cargo test -p bsl-diagnostics --locked --no-run` -> passed after splitting `semantic-diagnostics/src/visitor.rs`.
2. `cargo test -p bsl-diagnostics --locked visitor::tests::` -> passed (`14 passed`).
3. `cargo test -p bsl-repository --locked --no-run` -> passed after splitting `bsl-repository/src/repository.rs`.
4. `cargo test -p bsl-repository --locked repository::tests::` -> passed (`8 passed`).
5. `rg -n '^\\s*mod\\s+[A-Za-z0-9_]*tests\\s*\\{' semantic-diagnostics/src/visitor.rs bsl-repository/src/repository.rs` -> no matches (inline tests removed from production files).
6. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report openspec/changes/refactor-production-rust-files-over-1000-loc/validation/inventory.md --json` -> executed; hard LOC violations reduced `7 -> 5` after decomposing:
   - `semantic-diagnostics/src/visitor.rs` (`1272 -> 655`)
   - `bsl-repository/src/repository.rs` (`1262 -> 886`)

## Progress evidence (Batch D task `5.1` decomposition)

Executed on 2026-03-04:

1. `cargo test -p bsl-analysis-v2 --locked --no-run` -> passed after splitting `analysis-v2/src/lib.rs` and `analysis-v2/src/type_inference_v2.rs`.
2. `cargo test -p bsl-analysis-v2 --locked type_inference_v2::tests::` -> passed (`28 passed`).
3. `cargo test -p bsl-analysis-v2 --locked tests::` -> passed (`103 passed`).
4. `rg -n '^\\s*mod\\s+[A-Za-z0-9_]*tests\\s*\\{' analysis-v2/src/lib.rs analysis-v2/src/type_inference_v2.rs` -> no matches (inline tests removed from production files).
5. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report openspec/changes/refactor-production-rust-files-over-1000-loc/validation/inventory.md --json` -> executed; hard LOC violations reduced `5 -> 3` after decomposing:
   - `analysis-v2/src/lib.rs` (`4432 -> 341`)
   - `analysis-v2/src/type_inference_v2.rs` (`2803 -> 881`)

## Progress evidence (Batch D task `5.3` parity execution)

Executed on 2026-03-04:

1. `cargo test -p bsl-analysis-v2 --locked` -> passed (`103 passed`, `0 failed`; doc-tests `1 passed`).
2. `cargo test -p bsl-diagnostics --locked` -> passed (`16 passed`, `0 failed`; doc-tests `0`).
3. `cargo test -p bsl-repository --locked` -> passed (`70 passed`, `0 failed`; doc-tests `23 passed`).
4. `cargo test -p bsl-backend --locked --no-run` -> passed (all backend test binaries compiled).
5. `cargo test -p bsl-backend --locked --bin bsl-lsp-server handlers::hover::tests::m5_hover_v2_is_deterministic -- --nocapture` -> failed in current workspace.
6. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p24_real_scenario_observability_stage_parity_lsp_vs_mcp -- --nocapture` -> failed in current workspace.
7. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools -- --nocapture` -> failed in current workspace.
8. `cargo test -p bsl-backend --locked --test m8_completion_matrix_golden_v2_test -- --nocapture` -> failed in current workspace.
9. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p7_large_churn_budget_timeout_serves_cached_stale_fastpath -- --nocapture` -> passed in current workspace.
10. Baseline comparison in clean detached worktree at `HEAD` (`bd446f7`, path `/tmp/bsl-gradual-types-baseline-c`) with identical command set:
   - `m5_hover_v2_is_deterministic` -> failed
   - `p24_real_scenario_observability_stage_parity_lsp_vs_mcp` -> failed
   - `p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools` -> failed
   - `m8_completion_matrix_golden_v2_test` -> failed
   - `p7_large_churn_budget_timeout_serves_cached_stale_fastpath` -> passed

Conclusion for Batch D parity in this change:

- fail-set for validated backend parity checks matches baseline exactly;
- no new regressions introduced by Batch D decomposition in validated parity set;
- remaining failing backend tests are baseline drift outside current refactor scope.

## Progress evidence (Batch E task `6.1` decomposition)

Executed on 2026-03-04:

1. `cargo fmt --all` -> passed after splitting `bsl-agent/src/server/mod.rs` and `bsl-agent/src/session/mod.rs`.
2. `cargo test -p bsl-agent --locked --no-run` -> passed after extracting include-driven modules and test paths.
3. `cargo test -p bsl-agent --locked server::tests::` -> passed (`4 passed`).
4. `cargo test -p bsl-agent --locked session::tests::` -> passed (`13 passed`).
5. `rg -n '^\\s*mod\\s+[A-Za-z0-9_]*tests\\s*\\{' bsl-agent/src/server/mod.rs bsl-agent/src/session/mod.rs` -> no matches (inline test modules removed from production files).
6. `uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report openspec/changes/refactor-production-rust-files-over-1000-loc/validation/inventory.md --json` -> executed; hard LOC violations reduced `3 -> 1` after decomposing:
   - `bsl-agent/src/server/mod.rs` (`1427 -> 925`)
   - `bsl-agent/src/session/mod.rs` (`4946 -> 161`)

## Progress evidence (Batch E task `6.2` parity execution)

Executed on 2026-03-04:

1. `cargo test -p bsl-agent --locked` -> passed (`54 passed`, `0 failed`; doc-tests `0`).
2. `cargo test -p bsl-backend --locked --no-run` -> passed (all backend test binaries compiled).
3. `cargo test -p bsl-backend --locked --bin bsl-lsp-server handlers::hover::tests::m5_hover_v2_is_deterministic -- --nocapture` -> failed in current workspace.
4. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p24_real_scenario_observability_stage_parity_lsp_vs_mcp -- --nocapture` -> failed in current workspace.
5. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools -- --nocapture` -> failed in current workspace.
6. `cargo test -p bsl-backend --locked --test m8_completion_matrix_golden_v2_test -- --nocapture` -> failed in current workspace.
7. `cargo test -p bsl-backend --locked --bin bsl-lsp-server server::core::tests::p7_large_churn_budget_timeout_serves_cached_stale_fastpath -- --nocapture` -> passed in current workspace.
8. Baseline comparison in clean detached worktree at `HEAD` (`bd446f7`, path `/tmp/bsl-gradual-types-baseline-c`) with identical command set:
   - `m5_hover_v2_is_deterministic` -> failed
   - `p24_real_scenario_observability_stage_parity_lsp_vs_mcp` -> failed
   - `p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools` -> failed
   - `m8_completion_matrix_golden_v2_test` -> failed
   - `p7_large_churn_budget_timeout_serves_cached_stale_fastpath` -> passed

Conclusion for Batch E parity in this change:

- fail-set for validated backend parity checks matches baseline exactly;
- no new regressions introduced by Batch E decomposition in validated parity set;
- remaining failing backend tests are baseline drift outside current refactor scope.
