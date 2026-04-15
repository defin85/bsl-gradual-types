## 1. Implementation
- [x] 1.1 Introduce a canonical producer-side replay plan for ranged `didChange` requests before text reconstruction and parser-edit derivation.
- [x] 1.2 Use the same normalized ordered replay plan for both `updated_text` construction and `parser_edits`, applying multi-range changes in reverse document order.
- [x] 1.3 Preserve the existing fail-safe full fallback contract and canonical observability output while eliminating replay-order-induced `edits_do_not_match_new_content` mismatches.

## 2. Validation
- [x] 2.1 Add regressions covering valid multi-range `didChange` input, including UTF-16/Cyrillic text, where incremental replay remains version-consistent.
- [x] 2.2 Add an integration regression proving valid ranged `didChange` no longer reports `fallbackReason=edits_do_not_match_new_content` solely because producer replay order diverged.
- [x] 2.3 Run `openspec validate refactor-18-did-change-ranged-edit-normalization --strict --no-interactive`.
