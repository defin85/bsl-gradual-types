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
    "inline_test_module_violations": 45
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
