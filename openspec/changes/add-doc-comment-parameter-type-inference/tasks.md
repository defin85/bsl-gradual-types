## 1. Syntax and IR Plumbing

- [ ] 1.1 Add routine leading doc-comment block association near the existing compiler-directive lookup.
- [ ] 1.2 Implement a conservative parser for `Параметры:` / `Parameters:` routine parameter entries, including continuation type lines.
- [ ] 1.3 Add unit tests for adjacent comments, directive-separated comments, non-adjacent stale comments, malformed lines, and multi-type continuation lines.
- [ ] 1.4 Extend AST routine declarations to carry structured doc-comment parameter hints without performing type resolution.
- [ ] 1.5 Extend IR `Parameter` metadata to preserve doc-derived type hints, source range, and source kind.

## 2. Type Resolution and Inference

- [ ] 2.1 Resolve doc type names in `analysis-v2` through the existing type-resolution path.
- [ ] 2.2 Add coarse/family representations for recognized 1C metadata family types such as `СправочникОбъект`, `ДокументОбъект`, and register record-set/record-manager families when no concrete metadata object name is present.
- [ ] 2.3 Seed routine body parameter symbols from resolved doc-derived type hints before body inference.
- [ ] 2.4 Preserve local scope precedence: parameters with doc-derived hints still shadow globals/common modules/owner members.
- [ ] 2.5 Expose doc-derived parameter facts through exported routine summaries/signature facts consumed by hover, signature help, diagnostics, completion, CLI, Web, and `bsl-agent`.

## 3. Diagnostics and Fail-Closed Behavior

- [ ] 3.1 Add low-severity or traceable handling for unknown doc type names without failing parse/lowering/inference.
- [ ] 3.2 Ignore doc entries whose parameter name does not match the declaration, with optional diagnostic/trace evidence.
- [ ] 3.3 Ensure malformed comments do not generate high-confidence missing-member diagnostics from raw prose.

## 4. Regression Coverage

- [ ] 4.1 Add a regression fixture based on the BSP `ПриЧтенииНаСервере(Форма, ТекущийОбъект)` comment shape.
- [ ] 4.2 Assert that `Форма` resolves as `ФормаКлиентскогоПриложения` inside the routine body.
- [ ] 4.3 Assert that `ТекущийОбъект` resolves as a union/family type across the documented object/register families.
- [ ] 4.4 Assert that v2 diagnostics/hover/type-at-position consume the same doc-derived parameter facts.
- [ ] 4.5 Add `bsl-agent`/MCP smoke or integration coverage proving the agent surface reads the shared v2 result rather than reparsing or dropping the hints.

## 5. Validation

- [ ] 5.1 Run `cargo fmt`.
- [ ] 5.2 Run targeted Rust tests for syntax doc-comment parsing and analysis-v2 inference.
- [ ] 5.3 Run `cargo check --workspace`.
- [ ] 5.4 Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] 5.5 Run `openspec validate add-doc-comment-parameter-type-inference --strict --no-interactive`.
