## 1. Implementation
- [x] 1.1 Replace opaque incremental-parse fallback reporting with a bounded canonical reason taxonomy.
- [x] 1.2 Record version-bound parse-snapshot build context on `didChange`, including base-text source and change-shape classification.
- [x] 1.3 Extend the incident-bundle/export path with a compact version-bound parse-snapshot evidence surface that can be correlated with later didSave cycles.

## 2. Validation
- [x] 2.1 Add regressions proving stale-base-text, missing-previous-tree, and parser-rejected incremental paths map to distinct canonical fallback reasons.
- [x] 2.2 Add bundle/projection tests proving operators can see base-text source, change shape, and fallback reason without raw text payloads.
- [x] 2.3 Run `openspec validate refactor-16-did-change-incremental-parse-fallback-attribution --strict --no-interactive`.

## 3. OpenSpec / Beads Sync
- [x] 3.1 Keep Beads epic `bsl-gradual-types-1rkq` and child `bsl-gradual-types-1rkq.2` aligned with the real implementation/validation status of this change.
