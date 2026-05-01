## Context

BSL procedure/function declarations contain parameter names, passing mode, defaults, and export visibility, but not type annotations. 1C development standards describe procedure/function descriptions as leading comments with a `Parameters:` section, and allow one parameter to have one or several type descriptions on separate lines. In Russian codebases the same convention is commonly written as `Параметры:`.

The current repository already has the right semantic path for this feature:

- `syntax/src/tree_sitter_adapter/directives.rs` looks backward from routine declarations to find adjacent compiler directives.
- `syntax/src/tree_sitter_adapter/statement_converter/declarations.rs` lowers declarations, but comments are otherwise skipped.
- `syntax/src/ast.rs` routine declarations currently carry `params: Vec<String>`.
- `shared/src/ir/types.rs` already has `Parameter { name, type_hint, default_value, is_val }`.
- `analysis-v2/src/ast_to_ir/statement_converter.rs` currently sets routine parameter `type_hint` to `None`.

The feature should therefore add doc-comment metadata to the existing shared v2 pipeline instead of adding an adapter-specific source scan.

## Goals / Non-Goals

Goals:

- Extract standard leading routine comments into structured parameter type hints.
- Support multi-line multi-type parameter entries like the BSP `ТекущийОбъект` example.
- Preserve comment-derived hints as typed metadata through AST -> IR.
- Use the hints to seed routine parameter types inside the routine body and exported routine summaries.
- Keep all v2 surfaces consistent by reading the same semantic facts.
- Fail closed on stale, unknown, malformed, or ambiguous comments.

Non-goals:

- Do not introduce type syntax into BSL signatures.
- Do not evaluate arbitrary prose or infer types from free-form descriptions outside the parameter section.
- Do not parse function return documentation in this change, except for leaving the model extensible.
- Do not make doc comments an absolute proof that suppresses all runtime/type uncertainty.

## Comment Association

Add a syntax helper next to the existing directive lookup, for example `find_preceding_doc_comment_block(node, source)`.

Association rules:

1. Only comments immediately adjacent to a routine declaration are eligible.
2. Compiler directives may appear between the comment block and declaration.
3. Blank lines inside the comment block are allowed.
4. A non-comment, non-directive token between the block and declaration stops association.
5. A comment block without a parameter section is ignored for parameter inference.

This keeps stale comments from binding across unrelated statements.

## Parsing Model

Normalize only the comment prefix and line order:

- Strip leading `//` and one optional following space.
- Preserve indentation enough to distinguish parameter starts from continuation lines.
- Recognize section headers case-insensitively: `Параметры:` and `Parameters:`.

Inside the section:

1. A parameter entry starts with a name that matches one declaration parameter, followed by `-`.
2. A continuation type line starts with `-` after indentation and belongs to the current parameter.
3. Each type part is read up to the next description separator, preserving only type tokens/list.
4. Multiple type lines for the same parameter produce a union-like list of raw type names.
5. Lines that do not match the conservative grammar are ignored or recorded as low-severity doc parse notes.

For the motivating example, `ТекущийОбъект` should produce the raw type list:

- `СправочникОбъект`
- `ДокументОбъект`
- `ПланВидовХарактеристикОбъект`
- `ПланСчетовОбъект`
- `ПланВидовРасчетаОбъект`
- `БизнесПроцессОбъект`
- `ЗадачаОбъект`
- `ПланОбменаОбъект`
- `РегистрСведенийМенеджерЗаписи`
- `РегистрСведенийНаборЗаписей`
- `РегистрНакопленияНаборЗаписей`
- `РегистрБухгалтерииНаборЗаписей`
- `РегистрРасчетаНаборЗаписей`

## AST and IR Shape

Prefer structured metadata over a string-only hint:

```rust
pub struct ParameterDocTypeHint {
    pub raw_type_names: Vec<String>,
    pub source: ParameterTypeHintSource,
    pub range: Span,
}

pub enum ParameterTypeHintSource {
    DocComment,
}
```

`Parameter.type_hint: Option<String>` is not enough for durable union/family semantics. If changing the public shape is too large in one implementation slice, keep `type_hint` as a backwards-compatible display/canonical field and add a structured `doc_type_hints` field. The v2 resolver should consume the structured field.

The syntax and IR layers must not resolve the types. They should only preserve raw names, source range, and source kind.

## Type Resolution

`analysis-v2` resolves raw doc type names through the same type-resolution services used elsewhere:

- platform/simple types, for example `ФормаКлиентскогоПриложения`;
- known configuration-specific metadata types when the name includes enough identity;
- broad metadata family types when the doc comment names a family such as `ДокументОбъект` or `СправочникОбъект`.

Broad family types are intentionally not the same as a missing concrete metadata object. They should become coarse semantic facets, not `Unknown`, when the family name is recognized. Multiple doc types become a union-like `TypeResolution` with source/certainty metadata indicating that the evidence came from documentation.

Unknown names fail closed: preserve the raw text for display/diagnostics if useful, but do not invent a concrete owner/member contract.

## Inference and Consumers

Routine body scope creation should seed parameters from doc-derived resolutions before body inference runs. Local variables and explicit assignments inside the body can still widen or override flow-sensitive facts according to existing v2 rules.

Exported routine summaries should expose doc-derived parameter types so signature help, hover, diagnostics, completion, CLI, Web, and `bsl-agent` all see the same contract through v2 semantic facts. Adapter layers must not reparse comments or maintain separate doc-comment inference.

## Diagnostics Policy

Doc comments are useful evidence, not infallible syntax. Diagnostics should distinguish:

- recognized doc-derived type;
- unknown doc type name;
- doc parameter name that does not match the declaration;
- malformed parameter section.

Malformed or partially recognized docs must not stop parsing, AST lowering, IR construction, or semantic analysis. High-confidence missing-member diagnostics should require a resolved owner/type, not raw doc prose.

## Alternatives Considered

- Parse comments directly in `type_inference_v2` from raw source text: rejected because it duplicates syntax trivia rules and would drift across consumers.
- Add a `bsl-agent`-only parser: rejected because it would recreate the previous surface-parity gap.
- Store only a display string in `Parameter.type_hint`: acceptable as a temporary migration bridge, but insufficient as the target model because the BSP example needs union/family semantics.
- Wait for a full 1C documentation parser: rejected because the parameter-section grammar is small and the value is immediate.

## Open Questions

- Should unresolved doc type names produce user-facing informational diagnostics immediately, or only appear in debug/trace output until false-positive risk is measured?
- Should function return documentation be added as a follow-up change using the same comment block association model?
