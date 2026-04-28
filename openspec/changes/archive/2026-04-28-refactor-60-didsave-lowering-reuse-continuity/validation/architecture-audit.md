# Architecture Audit: refactor-60-didsave-lowering-reuse-continuity

## Audit Verdict

Conditionally approved after tightening seed retention wording.

The change correctly targets the residual proven by
`/home/egor/code/temp/bsl-observability-incident-2026-04-27T11-07-23Z`: a
post-refactor-59 same-file `didSave` sequence where v11 proves fast lowering
reuse but v15 falls back to `full_rebuild` with no reuse-plan source. The main
architecture risk was that "bounded seed" could be implemented as immediate
eviction, turning the new contract back into cache-only behavior. This audit
tightened design, spec, and tasks so seed lifetime, candidate count, and eviction
reason are observable and testable.

## Locked Decisions

- Scope is didSave lowering reuse continuity, not UI/pre-send, completion
  ingress/egress, runtime saturation, or classifier-only work.
- The save-critical path may still use the existing parser AST cache, but cache
  residency is not the sole acceptance path.
- Full rebuild remains legal only with a low-cardinality reason proving why reuse
  was unavailable or unsafe for the same trace.
- Exact interactive readiness semantics remain unchanged.
- Seed retention is bounded, but bounded retention cannot be a normal accepted
  excuse for losing the active representative save-family seed.

## Audit Matrix

| Area | Verdict | Evidence | Follow-up |
| --- | --- | --- | --- |
| Requirement coverage | Pass after patch | Spec now requires seed source, candidate count, eviction reason, and full-rebuild reason. | Keep scenarios request-centric. |
| Performance | Pass | v15 target is `program_lowering=4125ms`, `0/2088` reuse. | Gate against seconds-scale unproved full rebuilds. |
| Reliability | Pass after patch | Seed validation remains fail-closed by text hash/ranges/parser compatibility. | Add unsafe-seed negative tests. |
| Operability | Pass after patch | Bundle must export source/reason fields through backend, custom request, raw JSON, and summary. | Add projection tests. |
| Scalability/memory | Watch | Seed store is intentionally bounded. | Add cleanup and eviction tests. |
| Compatibility | Pass | Exact interactive consumers are explicitly preserved. | Verify completion/hover/definition paths are untouched. |
| Rollback | Watch | Design allows cache-only fallback with explicit reason. | Ensure rollback mode is visible in validation. |
| Test strategy | Pass after patch | Tasks now include seed miss, eviction, cleanup, timeline, and projection coverage. | Keep representative live gate. |

## External References

- Rust compiler dev guide incremental compilation:
  https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html
- Rust compiler dev guide incremental compilation in detail:
  https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html
- Salsa red-green algorithm:
  https://salsa-rs.github.io/salsa/reference/algorithm.html
- rust-analyzer architecture:
  https://rust-analyzer.github.io/book/contributing/architecture.html

## Exact Wording Fixes Applied

- `design.md`: added seed lifetime as a correctness contract, explicit
  `seed_evicted`, seed candidate count, eviction reason, cleanup, and rollback
  boundaries.
- `specs/bsl-intellisense-v2/spec.md`: added retention/eviction requirements
  and scenarios so bounded retention cannot masquerade as accepted full rebuild.
- `tasks.md`: added retention cleanup, seed candidate/eviction observability,
  and negative eviction coverage.

## Execution Plan

1. Implement the seed model and validation in the runtime.
2. Wire seed source, candidate count, eviction reason, and failure reason into
   diagnostics-save timeline attribution.
3. Project new fields through custom requests and incident bundles.
4. Add parser-coordinator, diagnostics-save timeline, and VS Code projection
   tests.
5. Capture a fresh representative bundle and compare against the `11:07` source
   evidence.
6. Run strict OpenSpec validation and the repo verification gates listed in
   `tasks.md`.

## Assumptions and Open Questions

- The first implementation should prefer a small per-file/save-family bounded
  store over global AST retention.
- Capacity limits are still open; the implementation may choose constants or a
  runtime config, but must expose capacity-pressure eviction as evidence.
- If a later bundle proves current-context contention directly removes the seed,
  that should be treated as supporting evidence for the same continuity change,
  not as a separate UI/transport investigation.
