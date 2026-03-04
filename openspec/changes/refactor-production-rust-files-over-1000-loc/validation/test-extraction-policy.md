# Test Extraction Policy For Production Rust Files

Date: 2026-03-04  
Change: `refactor-production-rust-files-over-1000-loc`

## Policy

For this campaign, production Rust files MUST NOT keep inline test modules.

Disallowed pattern in production scope:

- inline test module blocks: `mod tests { ... }`, `mod *_tests { ... }`

Required target state:

- tests are moved to separate test paths (`tests/**`, crate-level dedicated test files, or dedicated sibling test files via `#[path = "..."] mod tests;`)
- production files keep only runtime logic and public/private implementation code

## Detection command

`rg -n '^\\s*mod\\s+[A-Za-z0-9_]*tests\\s*\\{' backend/src/bin bsl-agent/src bsl-runtime/src analysis-v2/src semantic-diagnostics/src bsl-repository/src`

## Baseline in target inventory (current)

Files in current large-file target inventory that still contain inline test module blocks:

- none (all target files in this change are migrated to separate test paths)

Files already migrated to separate test paths in this change:

1. `backend/src/bin/lsp_server/server/core.rs` -> `backend/src/bin/lsp_server/server/core/tests.rs`
2. `backend/src/bin/intellisense_perf.rs` -> `backend/src/bin/intellisense_perf/tests.rs`
3. `backend/src/perf_gate_evaluator.rs` -> `backend/src/perf_gate_evaluator/tests.rs`
4. `backend/src/bin/lsp_server/server/completion_dispatcher.rs` -> `backend/src/bin/lsp_server/server/completion_dispatcher/tests.rs`
5. `backend/src/bin/lsp_server/handlers/completion.rs` -> `backend/src/bin/lsp_server/handlers/completion/tests.rs`
6. `backend/src/bin/lsp_server/server/language_server.rs` -> `backend/src/bin/lsp_server/server/language_server/tests.rs`
7. `bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs` -> `bsl-runtime/src/data/loaders/config_metadata_parser/converter/tests.rs`
8. `bsl-runtime/src/system/disk_cache.rs` -> `bsl-runtime/src/system/disk_cache/tests.rs`
9. `bsl-runtime/src/system/runtime_config.rs` -> `bsl-runtime/src/system/runtime_config/tests.rs`
10. `bsl-runtime/src/system/parser_coordinator.rs` -> `bsl-runtime/src/system/parser_coordinator/tests.rs`
11. `bsl-runtime/src/system/system_coordinator/lifecycle.rs` -> `bsl-runtime/src/system/system_coordinator/lifecycle/tests.rs`
12. `bsl-runtime/src/system/system_coordinator/coordinator.rs` -> `bsl-runtime/src/system/system_coordinator/coordinator/tests.rs`
13. `bsl-runtime/src/system/system_coordinator/config_loader.rs` + `bsl-runtime/src/system/system_coordinator/config_loader/helpers.rs` -> `bsl-runtime/src/system/system_coordinator/config_loader/helpers/tests.rs`
14. `bsl-runtime/src/system/basic_observability.rs` -> `bsl-runtime/src/system/basic_observability/tests.rs` (plus notes in `bsl-runtime/src/system/basic_observability/comparison_notes.rs`)
15. `bsl-runtime/src/application/type_system/services/completion_service.rs` -> `bsl-runtime/src/application/type_system/services/completion_service/tests.rs`
16. `bsl-runtime/src/application/type_system/services/completion_ranking.rs` -> `bsl-runtime/src/application/type_system/services/completion_ranking/tests.rs`
17. `bsl-runtime/src/application/intellisense_v2/facade.rs` -> `bsl-runtime/src/application/intellisense_v2/facade/tests.rs`
18. `bsl-runtime/src/application/intellisense_v2/policy.rs` -> `bsl-runtime/src/application/intellisense_v2/policy/tests.rs`
19. `semantic-diagnostics/src/visitor.rs` -> `semantic-diagnostics/src/visitor/tests.rs`
20. `bsl-repository/src/repository.rs` -> `bsl-repository/src/repository/tests.rs`
21. `analysis-v2/src/lib.rs` -> `analysis-v2/src/lib/tests.rs`
22. `analysis-v2/src/type_inference_v2.rs` -> `analysis-v2/src/type_inference_v2/tests.rs`
23. `bsl-agent/src/server/mod.rs` -> `bsl-agent/src/server/tests.rs`
24. `bsl-agent/src/session/mod.rs` -> `bsl-agent/src/session/tests.rs`

## Migration rule for batches

For each decomposed file in batches A-E:

1. extract inline tests into dedicated test file/path
2. keep/restore equivalent assertions and fixtures
3. pass batch parity checks from `validation/parity-matrix.md`

The task is considered complete only when no inline test module remains in
production files covered by this change.
