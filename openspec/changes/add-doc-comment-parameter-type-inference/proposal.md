# Change: Infer routine parameter types from standard BSL doc comments

## Why

Real BSP-style modules often document procedure/function parameter types in the standard leading comment block rather than in BSL syntax, because BSL routine signatures do not carry type annotations. v2 currently discards those comments before IR/type inference, so exported helpers such as `ПриЧтенииНаСервере(Форма, ТекущийОбъект)` lose useful parameter type evidence and can surface inconsistent or overly-unknown diagnostics, hover, completion, and agent results.

The recent BSP common-module factory work resolves the target exported routine. The remaining gap is that the resolved routine's documented parameter contract is still not available to the shared v2 semantic pipeline.

## What Changes

- Parse adjacent standard BSL doc-comment blocks before procedure/function declarations.
- Extract `Параметры:` / `Parameters:` entries into structured parameter type hints, including multi-line multi-type parameter descriptions.
- Preserve the hints through AST -> IR without doing inference in the syntax/IR layer.
- Seed routine parameter symbols in `analysis-v2` from doc-derived hints so body inference and exported routine summaries can consume them.
- Represent broad 1C metadata family types such as `СправочникОбъект`, `ДокументОбъект`, and register record-set/record-manager families without requiring a concrete metadata object name.
- Keep unknown, malformed, stale, or non-adjacent comments fail-closed: they must not break parsing and must not create high-confidence diagnostics from guessed types.
- Expose the same doc-derived type facts through all v2 consumers that already read v2 semantic facts, including LSP/VS Code, CLI, Web, and `bsl-agent`.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `syntax/src/tree_sitter_adapter/` comment/directive/declaration conversion
  - `syntax/src/ast.rs` routine declaration metadata
  - `shared/src/ir/types.rs` parameter type-hint representation
  - `analysis-v2/src/ast_to_ir/` AST -> IR lowering
  - `analysis-v2/src/type_inference_v2/` parameter scope seeding and local/exported routine summaries
  - semantic diagnostics, hover/type-at-position, completion, signature help, CLI/Web/`bsl-agent` consumers that use v2 semantic facts
