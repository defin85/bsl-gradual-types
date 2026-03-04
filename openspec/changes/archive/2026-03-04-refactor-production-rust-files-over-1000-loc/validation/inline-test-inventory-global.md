# Global Inline Test Module Inventory (Production Rust Scope)

Date: 2026-03-04
Change: refactor-production-rust-files-over-1000-loc
Task: bsl-gradual-types-21s.1

## Scope and detection

Production scope rule mirrors scripts/check-rust-file-llm-budget.py exclusions: third_party, target, node_modules, tests, benches, examples, fixtures, mocks; and file-name exclusions tests.rs, *_test.rs.

Detection pattern for inline blocks: ^\s*mod\s+[A-Za-z0-9_]*tests\s*\{

## Summary

- Inline test module blocks found: 87
- Subsystems involved: 14
- Owner source: CODEOWNERS not found; owner is assigned by top-level subsystem prefix.

### Counts by subsystem
- analysis-v2: 3
- backend: 13
- bsl-agent: 2
- bsl-api-dtos: 1
- bsl-repository: 3
- bsl-runtime: 27
- bsl-types: 5
- frontend: 1
- line-index: 1
- mcp-debug-server: 8
- semantic-diagnostics: 1
- shared: 17
- syntax: 3
- type-visualization: 2

## Detailed inventory

| File | Line | Block | Nearby cfg(test) | Proposed test path | Owner (subsystem) | Migration plan |
|---|---:|---|---|---|---|---|
| `analysis-v2/src/ast_to_ir/global_collections.rs` | 141 | `mod tests {` | `140:#[cfg(test)]` | `analysis-v2/src/ast_to_ir/global_collections/tests.rs` | `analysis-v2` | Extract inline module to global_collections/tests.rs and wire via #[path = "global_collections/tests.rs"] mod tests; |
| `analysis-v2/src/derived_artifacts.rs` | 475 | `mod tests {` | `474:#[cfg(test)]` | `analysis-v2/src/derived_artifacts/tests.rs` | `analysis-v2` | Extract inline module to derived_artifacts/tests.rs and wire via #[path = "derived_artifacts/tests.rs"] mod tests; |
| `analysis-v2/src/implicit_bindings.rs` | 181 | `mod tests {` | `180:#[cfg(test)]` | `analysis-v2/src/implicit_bindings/tests.rs` | `analysis-v2` | Extract inline module to implicit_bindings/tests.rs and wire via #[path = "implicit_bindings/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/config.rs` | 96 | `mod tests {` | `95:#[cfg(test)]` | `backend/src/bin/lsp_server/config/tests.rs` | `backend` | Extract inline module to config/tests.rs and wire via #[path = "config/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/converters/diagnostics.rs` | 105 | `mod tests {` | `104:#[cfg(test)]` | `backend/src/bin/lsp_server/converters/diagnostics/tests.rs` | `backend` | Extract inline module to diagnostics/tests.rs and wire via #[path = "diagnostics/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/converters/position.rs` | 48 | `mod tests {` | `47:#[cfg(test)]` | `backend/src/bin/lsp_server/converters/position/tests.rs` | `backend` | Extract inline module to position/tests.rs and wire via #[path = "position/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/handlers/code_actions.rs` | 325 | `mod tests {` | `324:#[cfg(test)]` | `backend/src/bin/lsp_server/handlers/code_actions/tests.rs` | `backend` | Extract inline module to code_actions/tests.rs and wire via #[path = "code_actions/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/handlers/formatting.rs` | 136 | `mod tests {` | `135:#[cfg(test)]` | `backend/src/bin/lsp_server/handlers/formatting/tests.rs` | `backend` | Extract inline module to formatting/tests.rs and wire via #[path = "formatting/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/handlers/hover.rs` | 102 | `mod tests {` | `101:#[cfg(test)]` | `backend/src/bin/lsp_server/handlers/hover/tests.rs` | `backend` | Extract inline module to hover/tests.rs and wire via #[path = "hover/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/handlers/inlay_hints.rs` | 243 | `mod tests {` | `242:#[cfg(test)]` | `backend/src/bin/lsp_server/handlers/inlay_hints/tests.rs` | `backend` | Extract inline module to inlay_hints/tests.rs and wire via #[path = "inlay_hints/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/handlers/signature_help.rs` | 54 | `mod tests {` | `53:#[cfg(test)]` | `backend/src/bin/lsp_server/handlers/signature_help/tests.rs` | `backend` | Extract inline module to signature_help/tests.rs and wire via #[path = "signature_help/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/progress_bridge.rs` | 225 | `mod tests {` | `224:#[cfg(test)]` | `backend/src/bin/lsp_server/progress_bridge/tests.rs` | `backend` | Extract inline module to progress_bridge/tests.rs and wire via #[path = "progress_bridge/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/server/completion_cancellation.rs` | 171 | `mod tests {` | `170:#[cfg(test)]` | `backend/src/bin/lsp_server/server/completion_cancellation/tests.rs` | `backend` | Extract inline module to completion_cancellation/tests.rs and wire via #[path = "completion_cancellation/tests.rs"] mod tests; |
| `backend/src/bin/lsp_server/server/request_context.rs` | 220 | `mod tests {` | `219:#[cfg(test)]` | `backend/src/bin/lsp_server/server/request_context/tests.rs` | `backend` | Extract inline module to request_context/tests.rs and wire via #[path = "request_context/tests.rs"] mod tests; |
| `backend/src/presentation/semantic_html_generator/generator.rs` | 143 | `mod tests {` | `142:#[cfg(test)]` | `backend/src/presentation/semantic_html_generator/generator/tests.rs` | `backend` | Extract inline module to generator/tests.rs and wire via #[path = "generator/tests.rs"] mod tests; |
| `backend/src/presentation/semantic_html_generator/utils.rs` | 13 | `mod tests {` | `12:#[cfg(test)]` | `backend/src/presentation/semantic_html_generator/utils/tests.rs` | `backend` | Extract inline module to utils/tests.rs and wire via #[path = "utils/tests.rs"] mod tests; |
| `bsl-agent/src/jobs/mod.rs` | 595 | `mod tests {` | `594:#[cfg(test)]` | `bsl-agent/src/jobs/tests.rs` | `bsl-agent` | Extract inline module to sibling tests.rs; keep #[cfg(test)] mod tests; in mod.rs |
| `bsl-agent/src/semantic/mod.rs` | 7 | `mod tests {` | `6:#[cfg(test)]` | `bsl-agent/src/semantic/tests.rs` | `bsl-agent` | Extract inline module to sibling tests.rs; keep #[cfg(test)] mod tests; in mod.rs |
| `bsl-api-dtos/src/semantic_dtos.rs` | 442 | `mod tests {` | `441:#[cfg(test)]` | `bsl-api-dtos/src/semantic_dtos/tests.rs` | `bsl-api-dtos` | Extract inline module to semantic_dtos/tests.rs and wire via #[path = "semantic_dtos/tests.rs"] mod tests; |
| `bsl-repository/src/signature_index/index.rs` | 646 | `mod tests {` | `645:#[cfg(test)]` | `bsl-repository/src/signature_index/index/tests.rs` | `bsl-repository` | Extract inline module to index/tests.rs and wire via #[path = "index/tests.rs"] mod tests; |
| `bsl-repository/src/signature_index/method_builder.rs` | 403 | `mod tests {` | `402:#[cfg(test)]` | `bsl-repository/src/signature_index/method_builder/tests.rs` | `bsl-repository` | Extract inline module to method_builder/tests.rs and wire via #[path = "method_builder/tests.rs"] mod tests; |
| `bsl-repository/src/signature_registry.rs` | 283 | `mod tests {` | `282:#[cfg(test)]` | `bsl-repository/src/signature_registry/tests.rs` | `bsl-repository` | Extract inline module to signature_registry/tests.rs and wire via #[path = "signature_registry/tests.rs"] mod tests; |
| `bsl-runtime/src/application/type_system/extractors/symbol_extractor.rs` | 94 | `mod tests {` | `93:#[cfg(test)]` | `bsl-runtime/src/application/type_system/extractors/symbol_extractor/tests.rs` | `bsl-runtime` | Extract inline module to symbol_extractor/tests.rs and wire via #[path = "symbol_extractor/tests.rs"] mod tests; |
| `bsl-runtime/src/application/type_system/extractors/type_extractor.rs` | 107 | `mod tests {` | `106:#[cfg(test)]` | `bsl-runtime/src/application/type_system/extractors/type_extractor/tests.rs` | `bsl-runtime` | Extract inline module to type_extractor/tests.rs and wire via #[path = "type_extractor/tests.rs"] mod tests; |
| `bsl-runtime/src/application/type_system/formatters/hover_formatters.rs` | 309 | `mod tests {` | `308:#[cfg(test)]` | `bsl-runtime/src/application/type_system/formatters/hover_formatters/tests.rs` | `bsl-runtime` | Extract inline module to hover_formatters/tests.rs and wire via #[path = "hover_formatters/tests.rs"] mod tests; |
| `bsl-runtime/src/application/type_system/formatters/type_formatters.rs` | 45 | `mod tests {` | `44:#[cfg(test)]` | `bsl-runtime/src/application/type_system/formatters/type_formatters/tests.rs` | `bsl-runtime` | Extract inline module to type_formatters/tests.rs and wire via #[path = "type_formatters/tests.rs"] mod tests; |
| `bsl-runtime/src/application/type_system/services/completion_target.rs` | 686 | `mod tests {` | `685:#[cfg(test)]` | `bsl-runtime/src/application/type_system/services/completion_target/tests.rs` | `bsl-runtime` | Extract inline module to completion_target/tests.rs and wire via #[path = "completion_target/tests.rs"] mod tests; |
| `bsl-runtime/src/data/adapters/converters.rs` | 378 | `mod tests {` | `377:#[cfg(test)]` | `bsl-runtime/src/data/adapters/converters/tests.rs` | `bsl-runtime` | Extract inline module to converters/tests.rs and wire via #[path = "converters/tests.rs"] mod tests; |
| `bsl-runtime/src/data/loaders/config_metadata_parser/form_parser.rs` | 450 | `mod tests {` | `449:#[cfg(test)]` | `bsl-runtime/src/data/loaders/config_metadata_parser/form_parser/tests.rs` | `bsl-runtime` | Extract inline module to form_parser/tests.rs and wire via #[path = "form_parser/tests.rs"] mod tests; |
| `bsl-runtime/src/data/loaders/config_metadata_parser/parser.rs` | 571 | `mod tests {` | `570:#[cfg(test)]` | `bsl-runtime/src/data/loaders/config_metadata_parser/parser/tests.rs` | `bsl-runtime` | Extract inline module to parser/tests.rs and wire via #[path = "parser/tests.rs"] mod tests; |
| `bsl-runtime/src/data/loaders/hbk_recovery/mod.rs` | 60 | `mod tests {` | `59:#[cfg(test)]` | `bsl-runtime/src/data/loaders/hbk_recovery/tests.rs` | `bsl-runtime` | Extract inline module to sibling tests.rs; keep #[cfg(test)] mod tests; in mod.rs |
| `bsl-runtime/src/data/loaders/platform_types.rs` | 207 | `mod tests {` | `206:#[cfg(test)]` | `bsl-runtime/src/data/loaders/platform_types/tests.rs` | `bsl-runtime` | Extract inline module to platform_types/tests.rs and wire via #[path = "platform_types/tests.rs"] mod tests; |
| `bsl-runtime/src/data/loaders/progress.rs` | 292 | `mod tests {` | `291:#[cfg(test)]` | `bsl-runtime/src/data/loaders/progress/tests.rs` | `bsl-runtime` | Extract inline module to progress/tests.rs and wire via #[path = "progress/tests.rs"] mod tests; |
| `bsl-runtime/src/data/loaders/signature_sources.rs` | 41 | `mod tests {` | `40:#[cfg(test)]` | `bsl-runtime/src/data/loaders/signature_sources/tests.rs` | `bsl-runtime` | Extract inline module to signature_sources/tests.rs and wire via #[path = "signature_sources/tests.rs"] mod tests; |
| `bsl-runtime/src/data/loaders/syntax_helper/loader.rs` | 532 | `mod tests {` | `531:#[cfg(test)]` | `bsl-runtime/src/data/loaders/syntax_helper/loader/tests.rs` | `bsl-runtime` | Extract inline module to loader/tests.rs and wire via #[path = "loader/tests.rs"] mod tests; |
| `bsl-runtime/src/data/loaders/syntax_helper/type_parser.rs` | 371 | `mod tests {` | `370:#[cfg(test)]` | `bsl-runtime/src/data/loaders/syntax_helper/type_parser/tests.rs` | `bsl-runtime` | Extract inline module to type_parser/tests.rs and wire via #[path = "type_parser/tests.rs"] mod tests; |
| `bsl-runtime/src/domain/flow_analyzer_simple.rs` | 146 | `mod tests {` | `145:#[cfg(test)]` | `bsl-runtime/src/domain/flow_analyzer_simple/tests.rs` | `bsl-runtime` | Extract inline module to flow_analyzer_simple/tests.rs and wire via #[path = "flow_analyzer_simple/tests.rs"] mod tests; |
| `bsl-runtime/src/system/ast_cache.rs` | 108 | `mod tests {` | `107:#[cfg(test)]` | `bsl-runtime/src/system/ast_cache/tests.rs` | `bsl-runtime` | Extract inline module to ast_cache/tests.rs and wire via #[path = "ast_cache/tests.rs"] mod tests; |
| `bsl-runtime/src/system/fs_utils.rs` | 25 | `mod tests {` | `24:#[cfg(test)]` | `bsl-runtime/src/system/fs_utils/tests.rs` | `bsl-runtime` | Extract inline module to fs_utils/tests.rs and wire via #[path = "fs_utils/tests.rs"] mod tests; |
| `bsl-runtime/src/system/intellisense_index.rs` | 341 | `mod tests {` | `340:#[cfg(test)]` | `bsl-runtime/src/system/intellisense_index/tests.rs` | `bsl-runtime` | Extract inline module to intellisense_index/tests.rs and wire via #[path = "intellisense_index/tests.rs"] mod tests; |
| `bsl-runtime/src/system/intellisense_index_store.rs` | 401 | `mod tests {` | `400:#[cfg(test)]` | `bsl-runtime/src/system/intellisense_index_store/tests.rs` | `bsl-runtime` | Extract inline module to intellisense_index_store/tests.rs and wire via #[path = "intellisense_index_store/tests.rs"] mod tests; |
| `bsl-runtime/src/system/keyword_index.rs` | 113 | `mod tests {` | `112:#[cfg(test)]` | `bsl-runtime/src/system/keyword_index/tests.rs` | `bsl-runtime` | Extract inline module to keyword_index/tests.rs and wire via #[path = "keyword_index/tests.rs"] mod tests; |
| `bsl-runtime/src/system/parallel_analyzer.rs` | 420 | `mod tests {` | `419:#[cfg(test)]` | `bsl-runtime/src/system/parallel_analyzer/tests.rs` | `bsl-runtime` | Extract inline module to parallel_analyzer/tests.rs and wire via #[path = "parallel_analyzer/tests.rs"] mod tests; |
| `bsl-runtime/src/system/persistent_cache.rs` | 426 | `mod tests {` | `425:#[cfg(test)]` | `bsl-runtime/src/system/persistent_cache/tests.rs` | `bsl-runtime` | Extract inline module to persistent_cache/tests.rs and wire via #[path = "persistent_cache/tests.rs"] mod tests; |
| `bsl-runtime/src/system/platform_version.rs` | 40 | `mod tests {` | `39:#[cfg(test)]` | `bsl-runtime/src/system/platform_version/tests.rs` | `bsl-runtime` | Extract inline module to platform_version/tests.rs and wire via #[path = "platform_version/tests.rs"] mod tests; |
| `bsl-runtime/src/system/positioning.rs` | 81 | `mod tests {` | `80:#[cfg(test)]` | `bsl-runtime/src/system/positioning/tests.rs` | `bsl-runtime` | Extract inline module to positioning/tests.rs and wire via #[path = "positioning/tests.rs"] mod tests; |
| `bsl-runtime/src/system/startup_v2.rs` | 228 | `mod tests {` | `227:#[cfg(test)]` | `bsl-runtime/src/system/startup_v2/tests.rs` | `bsl-runtime` | Extract inline module to startup_v2/tests.rs and wire via #[path = "startup_v2/tests.rs"] mod tests; |
| `bsl-runtime/src/system/system_coordinator/mod.rs` | 26 | `mod tests {` | `25:#[cfg(test)]` | `bsl-runtime/src/system/system_coordinator/tests.rs` | `bsl-runtime` | Extract inline module to sibling tests.rs; keep #[cfg(test)] mod tests; in mod.rs |
| `bsl-runtime/src/system/tree_cache.rs` | 108 | `mod tests {` | `107:#[cfg(test)]` | `bsl-runtime/src/system/tree_cache/tests.rs` | `bsl-runtime` | Extract inline module to tree_cache/tests.rs and wire via #[path = "tree_cache/tests.rs"] mod tests; |
| `bsl-types/src/facet_utils.rs` | 327 | `mod tests {` | `326:#[cfg(test)]` | `bsl-types/src/facet_utils/tests.rs` | `bsl-types` | Extract inline module to facet_utils/tests.rs and wire via #[path = "facet_utils/tests.rs"] mod tests; |
| `bsl-types/src/metadata_patterns.rs` | 178 | `mod tests {` | `177:#[cfg(test)]` | `bsl-types/src/metadata_patterns/tests.rs` | `bsl-types` | Extract inline module to metadata_patterns/tests.rs and wire via #[path = "metadata_patterns/tests.rs"] mod tests; |
| `bsl-types/src/type_definition_location.rs` | 169 | `mod tests {` | `168:#[cfg(test)]` | `bsl-types/src/type_definition_location/tests.rs` | `bsl-types` | Extract inline module to type_definition_location/tests.rs and wire via #[path = "type_definition_location/tests.rs"] mod tests; |
| `bsl-types/src/type_id/core.rs` | 163 | `mod tests {` | `162:#[cfg(test)]` | `bsl-types/src/type_id/core/tests.rs` | `bsl-types` | Extract inline module to core/tests.rs and wire via #[path = "core/tests.rs"] mod tests; |
| `bsl-types/src/type_id/normalization.rs` | 100 | `mod tests {` | `99:#[cfg(test)]` | `bsl-types/src/type_id/normalization/tests.rs` | `bsl-types` | Extract inline module to normalization/tests.rs and wire via #[path = "normalization/tests.rs"] mod tests; |
| `frontend/src/api/extensions.rs` | 256 | `mod tests {` | `255:#[cfg(test)]` | `frontend/src/api/extensions/tests.rs` | `frontend` | Extract inline module to extensions/tests.rs and wire via #[path = "extensions/tests.rs"] mod tests; |
| `line-index/src/lib.rs` | 155 | `mod tests {` | `154:#[cfg(test)]` | `line-index/src/lib/tests.rs` | `line-index` | Extract inline module to lib/tests.rs and wire via #[path = "lib/tests.rs"] mod tests; |
| `mcp-debug-server/src/config/adapters.rs` | 152 | `mod tests {` | `151:#[cfg(test)]` | `mcp-debug-server/src/config/adapters/tests.rs` | `mcp-debug-server` | Extract inline module to adapters/tests.rs and wire via #[path = "adapters/tests.rs"] mod tests; |
| `mcp-debug-server/src/dap/events.rs` | 311 | `mod tests {` | `310:#[cfg(test)]` | `mcp-debug-server/src/dap/events/tests.rs` | `mcp-debug-server` | Extract inline module to events/tests.rs and wire via #[path = "events/tests.rs"] mod tests; |
| `mcp-debug-server/src/server/resources.rs` | 169 | `mod tests {` | `168:#[cfg(test)]` | `mcp-debug-server/src/server/resources/tests.rs` | `mcp-debug-server` | Extract inline module to resources/tests.rs and wire via #[path = "resources/tests.rs"] mod tests; |
| `mcp-debug-server/src/server/tools.rs` | 28 | `mod tests {` | `27:#[cfg(test)]` | `mcp-debug-server/src/server/tools/tests.rs` | `mcp-debug-server` | Extract inline module to tools/tests.rs and wire via #[path = "tools/tests.rs"] mod tests; |
| `mcp-debug-server/src/session/manager.rs` | 262 | `mod tests {` | `261:#[cfg(test)]` | `mcp-debug-server/src/session/manager/tests.rs` | `mcp-debug-server` | Extract inline module to manager/tests.rs and wire via #[path = "manager/tests.rs"] mod tests; |
| `mcp-debug-server/src/session/state.rs` | 112 | `mod tests {` | `111:#[cfg(test)]` | `mcp-debug-server/src/session/state/tests.rs` | `mcp-debug-server` | Extract inline module to state/tests.rs and wire via #[path = "state/tests.rs"] mod tests; |
| `mcp-debug-server/src/types/error.rs` | 149 | `mod tests {` | `148:#[cfg(test)]` | `mcp-debug-server/src/types/error/tests.rs` | `mcp-debug-server` | Extract inline module to error/tests.rs and wire via #[path = "error/tests.rs"] mod tests; |
| `mcp-debug-server/src/types/session_id.rs` | 54 | `mod tests {` | `53:#[cfg(test)]` | `mcp-debug-server/src/types/session_id/tests.rs` | `mcp-debug-server` | Extract inline module to session_id/tests.rs and wire via #[path = "session_id/tests.rs"] mod tests; |
| `semantic-diagnostics/src/helpers.rs` | 87 | `mod tests {` | `86:#[cfg(test)]` | `semantic-diagnostics/src/helpers/tests.rs` | `semantic-diagnostics` | Extract inline module to helpers/tests.rs and wire via #[path = "helpers/tests.rs"] mod tests; |
| `shared/src/analysis/narrowing_engine.rs` | 299 | `mod tests {` | `298:#[cfg(test)]` | `shared/src/analysis/narrowing_engine/tests.rs` | `shared` | Extract inline module to narrowing_engine/tests.rs and wire via #[path = "narrowing_engine/tests.rs"] mod tests; |
| `shared/src/analysis/type_guards.rs` | 376 | `mod tests {` | `375:#[cfg(test)]` | `shared/src/analysis/type_guards/tests.rs` | `shared` | Extract inline module to type_guards/tests.rs and wire via #[path = "type_guards/tests.rs"] mod tests; |
| `shared/src/domain/code_location.rs` | 463 | `mod tests {` | `462:#[cfg(test)]` | `shared/src/domain/code_location/tests.rs` | `shared` | Extract inline module to code_location/tests.rs and wire via #[path = "code_location/tests.rs"] mod tests; |
| `shared/src/domain/flow_analysis.rs` | 254 | `mod tests {` | `253:#[cfg(test)]` | `shared/src/domain/flow_analysis/tests.rs` | `shared` | Extract inline module to flow_analysis/tests.rs and wire via #[path = "flow_analysis/tests.rs"] mod tests; |
| `shared/src/domain/generic_inference.rs` | 255 | `mod tests {` | `254:#[cfg(test)]` | `shared/src/domain/generic_inference/tests.rs` | `shared` | Extract inline module to generic_inference/tests.rs and wire via #[path = "generic_inference/tests.rs"] mod tests; |
| `shared/src/domain/metadata_constants.rs` | 331 | `mod tests {` | `330:#[cfg(test)]` | `shared/src/domain/metadata_constants/tests.rs` | `shared` | Extract inline module to metadata_constants/tests.rs and wire via #[path = "metadata_constants/tests.rs"] mod tests; |
| `shared/src/domain/null_safety.rs` | 340 | `mod tests {` | `339:#[cfg(test)]` | `shared/src/domain/null_safety/tests.rs` | `shared` | Extract inline module to null_safety/tests.rs and wire via #[path = "null_safety/tests.rs"] mod tests; |
| `shared/src/domain/resolver/resolver_generic_tests.rs` | 14 | `mod tests {` | `13:#[cfg(test)]` | `shared/src/domain/resolver/resolver_generic_tests/tests.rs` | `shared` | File name suggests test-focused file in production scope; move under tests/ or rename to *_test.rs and exclude by scope policy |
| `shared/src/domain/resolver/resolver_intersection_tests.rs` | 14 | `mod tests {` | `13:#[cfg(test)]` | `shared/src/domain/resolver/resolver_intersection_tests/tests.rs` | `shared` | File name suggests test-focused file in production scope; move under tests/ or rename to *_test.rs and exclude by scope policy |
| `shared/src/domain/resolver/resolver_nullable_tests.rs` | 14 | `mod tests {` | `13:#[cfg(test)]` | `shared/src/domain/resolver/resolver_nullable_tests/tests.rs` | `shared` | File name suggests test-focused file in production scope; move under tests/ or rename to *_test.rs and exclude by scope policy |
| `shared/src/domain/resolver/resolver_union_tests.rs` | 4 | `mod tests {` | `3:#[cfg(test)]` | `shared/src/domain/resolver/resolver_union_tests/tests.rs` | `shared` | File name suggests test-focused file in production scope; move under tests/ or rename to *_test.rs and exclude by scope policy |
| `shared/src/domain/runtime_context.rs` | 91 | `mod tests {` | `90:#[cfg(test)]` | `shared/src/domain/runtime_context/tests.rs` | `shared` | Extract inline module to runtime_context/tests.rs and wire via #[path = "runtime_context/tests.rs"] mod tests; |
| `shared/src/formatting/mod.rs` | 135 | `mod tests {` | `134:#[cfg(test)]` | `shared/src/formatting/tests.rs` | `shared` | Extract inline module to sibling tests.rs; keep #[cfg(test)] mod tests; in mod.rs |
| `shared/src/ir/cfg.rs` | 249 | `mod tests {` | `248:#[cfg(test)]` | `shared/src/ir/cfg/tests.rs` | `shared` | Extract inline module to cfg/tests.rs and wire via #[path = "cfg/tests.rs"] mod tests; |
| `shared/src/ir/visitor.rs` | 285 | `mod tests {` | `284:#[cfg(test)]` | `shared/src/ir/visitor/tests.rs` | `shared` | Extract inline module to visitor/tests.rs and wire via #[path = "visitor/tests.rs"] mod tests; |
| `shared/src/utils/hash.rs` | 26 | `mod tests {` | `25:#[cfg(test)]` | `shared/src/utils/hash/tests.rs` | `shared` | Extract inline module to hash/tests.rs and wire via #[path = "hash/tests.rs"] mod tests; |
| `shared/src/utils/string_utils.rs` | 130 | `mod tests {` | `129:#[cfg(test)]` | `shared/src/utils/string_utils/tests.rs` | `shared` | Extract inline module to string_utils/tests.rs and wire via #[path = "string_utils/tests.rs"] mod tests; |
| `syntax/src/formatter.rs` | 164 | `mod tests {` | `163:#[cfg(test)]` | `syntax/src/formatter/tests.rs` | `syntax` | Extract inline module to formatter/tests.rs and wire via #[path = "formatter/tests.rs"] mod tests; |
| `syntax/src/lib.rs` | 45 | `mod tests {` | `44:#[cfg(test)]` | `syntax/src/lib/tests.rs` | `syntax` | Extract inline module to lib/tests.rs and wire via #[path = "lib/tests.rs"] mod tests; |
| `syntax/src/tree_sitter_adapter/syntax_error_enhancers.rs` | 676 | `mod tests {` | `675:#[cfg(test)]` | `syntax/src/tree_sitter_adapter/syntax_error_enhancers/tests.rs` | `syntax` | Extract inline module to syntax_error_enhancers/tests.rs and wire via #[path = "syntax_error_enhancers/tests.rs"] mod tests; |
| `type-visualization/src/html_renderer.rs` | 332 | `mod tests {` | `331:#[cfg(test)]` | `type-visualization/src/html_renderer/tests.rs` | `type-visualization` | Extract inline module to html_renderer/tests.rs and wire via #[path = "html_renderer/tests.rs"] mod tests; |
| `type-visualization/src/markdown_renderer.rs` | 142 | `mod tests {` | `141:#[cfg(test)]` | `type-visualization/src/markdown_renderer/tests.rs` | `type-visualization` | Extract inline module to markdown_renderer/tests.rs and wire via #[path = "markdown_renderer/tests.rs"] mod tests; |
