{
  "pass": false,
  "tokenizer": "o200k_base",
  "limits": {
    "max_production_loc": 1000,
    "max_target_loc": 800,
    "max_target_bytes": 81920,
    "max_target_tokens": 12000
  },
  "counts": {
    "production_files_scanned": 441,
    "target_files_expected": 28,
    "target_files_missing": 0,
    "hard_loc_violations": 0,
    "target_budget_violations": 0,
    "inline_test_module_violations": 87
  },
  "violations": {
    "hard_loc": [],
    "target_missing": [],
    "target_budget": [],
    "inline_test_modules": [
      {
        "path": "analysis-v2/src/ast_to_ir/global_collections.rs",
        "line": 141,
        "snippet": "mod tests {"
      },
      {
        "path": "analysis-v2/src/derived_artifacts.rs",
        "line": 475,
        "snippet": "mod tests {"
      },
      {
        "path": "analysis-v2/src/implicit_bindings.rs",
        "line": 181,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/config.rs",
        "line": 96,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/converters/diagnostics.rs",
        "line": 105,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/converters/position.rs",
        "line": 48,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/handlers/code_actions.rs",
        "line": 325,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/handlers/formatting.rs",
        "line": 136,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/handlers/hover.rs",
        "line": 102,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/handlers/inlay_hints.rs",
        "line": 243,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/handlers/signature_help.rs",
        "line": 54,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/progress_bridge.rs",
        "line": 225,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/server/completion_cancellation.rs",
        "line": 171,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/bin/lsp_server/server/request_context.rs",
        "line": 220,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/presentation/semantic_html_generator/generator.rs",
        "line": 143,
        "snippet": "mod tests {"
      },
      {
        "path": "backend/src/presentation/semantic_html_generator/utils.rs",
        "line": 13,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-agent/src/jobs/mod.rs",
        "line": 595,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-agent/src/semantic/mod.rs",
        "line": 7,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-api-dtos/src/semantic_dtos.rs",
        "line": 442,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-repository/src/signature_index/index.rs",
        "line": 646,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-repository/src/signature_index/method_builder.rs",
        "line": 403,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-repository/src/signature_registry.rs",
        "line": 283,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/application/type_system/extractors/symbol_extractor.rs",
        "line": 94,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/application/type_system/extractors/type_extractor.rs",
        "line": 107,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/application/type_system/formatters/hover_formatters.rs",
        "line": 309,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/application/type_system/formatters/type_formatters.rs",
        "line": 45,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/application/type_system/services/completion_target.rs",
        "line": 686,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/data/adapters/converters.rs",
        "line": 378,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/data/loaders/config_metadata_parser/form_parser.rs",
        "line": 450,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/data/loaders/config_metadata_parser/parser.rs",
        "line": 571,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/data/loaders/hbk_recovery/mod.rs",
        "line": 60,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/data/loaders/platform_types.rs",
        "line": 207,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/data/loaders/progress.rs",
        "line": 292,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/data/loaders/signature_sources.rs",
        "line": 41,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/data/loaders/syntax_helper/loader.rs",
        "line": 532,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/data/loaders/syntax_helper/type_parser.rs",
        "line": 371,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/domain/flow_analyzer_simple.rs",
        "line": 146,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/ast_cache.rs",
        "line": 108,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/fs_utils.rs",
        "line": 25,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/intellisense_index.rs",
        "line": 341,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/intellisense_index_store.rs",
        "line": 401,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/keyword_index.rs",
        "line": 113,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/parallel_analyzer.rs",
        "line": 420,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/persistent_cache.rs",
        "line": 426,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/platform_version.rs",
        "line": 40,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/positioning.rs",
        "line": 81,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/startup_v2.rs",
        "line": 228,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/system_coordinator/mod.rs",
        "line": 26,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-runtime/src/system/tree_cache.rs",
        "line": 108,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-types/src/facet_utils.rs",
        "line": 327,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-types/src/metadata_patterns.rs",
        "line": 178,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-types/src/type_definition_location.rs",
        "line": 169,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-types/src/type_id/core.rs",
        "line": 163,
        "snippet": "mod tests {"
      },
      {
        "path": "bsl-types/src/type_id/normalization.rs",
        "line": 100,
        "snippet": "mod tests {"
      },
      {
        "path": "frontend/src/api/extensions.rs",
        "line": 256,
        "snippet": "mod tests {"
      },
      {
        "path": "line-index/src/lib.rs",
        "line": 155,
        "snippet": "mod tests {"
      },
      {
        "path": "mcp-debug-server/src/config/adapters.rs",
        "line": 152,
        "snippet": "mod tests {"
      },
      {
        "path": "mcp-debug-server/src/dap/events.rs",
        "line": 311,
        "snippet": "mod tests {"
      },
      {
        "path": "mcp-debug-server/src/server/resources.rs",
        "line": 169,
        "snippet": "mod tests {"
      },
      {
        "path": "mcp-debug-server/src/server/tools.rs",
        "line": 28,
        "snippet": "mod tests {"
      },
      {
        "path": "mcp-debug-server/src/session/manager.rs",
        "line": 262,
        "snippet": "mod tests {"
      },
      {
        "path": "mcp-debug-server/src/session/state.rs",
        "line": 112,
        "snippet": "mod tests {"
      },
      {
        "path": "mcp-debug-server/src/types/error.rs",
        "line": 149,
        "snippet": "mod tests {"
      },
      {
        "path": "mcp-debug-server/src/types/session_id.rs",
        "line": 54,
        "snippet": "mod tests {"
      },
      {
        "path": "semantic-diagnostics/src/helpers.rs",
        "line": 87,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/analysis/narrowing_engine.rs",
        "line": 299,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/analysis/type_guards.rs",
        "line": 376,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/code_location.rs",
        "line": 463,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/flow_analysis.rs",
        "line": 254,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/generic_inference.rs",
        "line": 255,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/metadata_constants.rs",
        "line": 331,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/null_safety.rs",
        "line": 340,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/resolver/resolver_generic_tests.rs",
        "line": 14,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/resolver/resolver_intersection_tests.rs",
        "line": 14,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/resolver/resolver_nullable_tests.rs",
        "line": 14,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/resolver/resolver_union_tests.rs",
        "line": 4,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/domain/runtime_context.rs",
        "line": 91,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/formatting/mod.rs",
        "line": 135,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/ir/cfg.rs",
        "line": 249,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/ir/visitor.rs",
        "line": 285,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/utils/hash.rs",
        "line": 26,
        "snippet": "mod tests {"
      },
      {
        "path": "shared/src/utils/string_utils.rs",
        "line": 130,
        "snippet": "mod tests {"
      },
      {
        "path": "syntax/src/formatter.rs",
        "line": 164,
        "snippet": "mod tests {"
      },
      {
        "path": "syntax/src/lib.rs",
        "line": 45,
        "snippet": "mod tests {"
      },
      {
        "path": "syntax/src/tree_sitter_adapter/syntax_error_enhancers.rs",
        "line": 676,
        "snippet": "mod tests {"
      },
      {
        "path": "type-visualization/src/html_renderer.rs",
        "line": 332,
        "snippet": "mod tests {"
      },
      {
        "path": "type-visualization/src/markdown_renderer.rs",
        "line": 142,
        "snippet": "mod tests {"
      }
    ]
  }
}
