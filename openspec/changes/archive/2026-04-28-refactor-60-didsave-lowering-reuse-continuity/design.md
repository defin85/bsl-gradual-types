## Analysis

`refactor-59` changed the observability shape correctly: the new bundle from
runtime git `5691e618` reports the second save as `program_lowering_tail`
instead of hiding it behind generic `snapshot_with_deps`. The bundle also shows
the same runtime can perform the desired lowering reuse:

```text
v11: first_publish=65ms, followup=2258ms
v11: program_lowering=1ms
v11: reuse_outcome=top_level_reuse
v11: reuse_plan_build_source=borrowed
v11: reused/rebuilt lowering units=2088/0

v15: first_publish=62ms, followup=4649ms
v15: blocker=program_lowering_tail
v15: timeout_leaf=program_lowering
v15: program_lowering=4125ms
v15: reuse_outcome=full_rebuild
v15: reuse_plan_build_source=null
v15: take_if_unique_hit=false, borrowed_cache_hit=false
v15: reused/rebuilt lowering units=0/2088
```

The architecture driver is therefore reuse continuity, not more latency
attribution. The code path in `parser_coordinator.rs` currently builds exact
lowering reuse plans by looking up the previous source text in `ast_cache`:

- first `take_if_unique(ast_cache_key(old_source))`;
- then `get(ast_cache_key(old_source))`;
- otherwise return `None`, which later becomes full rebuild.

This is a reasonable opportunistic cache path, but it is too weak as the only
save-critical continuity source. In an IDE server, concurrent same-file work can
observe, promote, replace, or miss cache entries across didChange, didSave,
current-context, and diagnostics follow-up tasks. A cache miss by itself does not
prove the user edit requires rebuilding all lowering units.

Prior art supports this separation. The rustc incremental model relies on
explicit dependency/reuse keys and stable fingerprints rather than assuming that
an arbitrary cache entry will still be resident when needed. Salsa similarly
tracks input revisions and dependencies, and can "backdate" results when a
changed input yields the same derived value. Applied here, the save-critical
lowering path needs a scoped, explicit reuse seed with a reasoned fallback.

## Recommendations

### Recommended option: save-family lowering reuse seed

Introduce a bounded save-family lowering reuse seed for exact ready-snapshot
assembly. The seed should be separate from the opportunistic `ast_cache` and
keyed by the active file/save contour:

```text
file_id
requested_version
text_hash
save_cycle_sequence
base_version or base_text_hash
changed_ranges fingerprint
seed_source
```

Candidate seed sources:

- previous same-file ready parse snapshot when text/hash and version relation
  are safe;
- didChange incremental parse snapshot evidence for the same edit family;
- same-content ready snapshot used by current-context after the p33 fix;
- existing parser AST cache as a fast source, but not the only source.

The exact producer should select the strongest valid seed and then build a
lowering reuse plan from it. If no seed is valid, the full rebuild path remains
allowed, but it must emit a low-cardinality reason.

### Alternatives considered

1. Instrumentation-only

This would add more fields around `full_rebuild`, but v15 already proves the
dominant cost and the missing cache source. It would improve triage but not
reduce the tail.

2. Global AST cache retention

Keeping more AST entries could reduce misses, but it risks memory growth and
still couples correctness of a save-critical path to cache residency. It is a
useful fallback optimization, not the primary contract.

3. Current-context throttling

The bundle has long current-context broker parses, but completions remain
healthy and didSave v15's dominant leaf is inside exact program lowering.
Current-context work may be a secondary pressure source, but changing it first
would not directly explain `borrowed_cache_hit=false`.

## Implementation Considerations

1. Model the seed explicitly

Add a small internal structure for a lowering reuse seed. It should carry the
base parse result or a reusable lowering-plan source, plus enough identity to
validate it against the target exact producer. Keep it scoped to recent
same-file/save-family work and evict it boundedly.

Seed lifetime is part of the correctness contract, not only an implementation
detail. A still-current save family should retain at least one compatible seed
until that family reaches a terminal outcome, is superseded/cancelled/failed, or
the implementation can prove an explicit bounded-retention limit forced eviction.
The latter case is allowed only when the trace records the eviction reason and
the representative validation shows this is not the common steady-state path for
the target large-module scenario.

2. Define seed selection order

Prefer the most request-local and semantically specific source:

- exact same save-family/didChange parse snapshot;
- same-text ready snapshot for the target file;
- previous ready snapshot with validated ranged edits;
- borrowed/owned parser AST cache.

When multiple candidates exist, choose deterministically and expose the source.

3. Validate before reuse

Reuse must remain fail-closed. The seed must match the target text hash or have
validated changed ranges that can derive a safe lowering plan. Unsafe shapes
must emit reasons such as:

- `missing_seed`;
- `text_hash_mismatch`;
- `changed_ranges_missing`;
- `changed_ranges_unsafe`;
- `syntax_tree_incompatible`;
- `seed_superseded`;
- `seed_cancelled`;
- `seed_failed`;
- `seed_evicted`;
- `continuity_lost`;
- `cache_disabled`.

4. Preserve seed lifecycle observability end to end

Add/export seed lifecycle fields near the existing program-lowering reuse
fields, for example:

```text
followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_failure_reason
followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_source
followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_candidate_count
followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_eviction_reason
```

Names can be shortened if the local telemetry naming style has an existing
better convention, but the exported bundle must answer why v15 did not use v11's
kind of seed, whether any candidate seed existed, and whether a bounded-retention
decision removed it before exact assembly could use it.

5. Add cleanup and rollback boundaries

The seed store must have explicit cleanup when a save family terminates,
supersedes, cancels, or fails. Rollback must be straightforward: disabling the
new seed source should fall back to the existing cache-only behavior while still
exporting the failure reason (`cache_only_fallback` or equivalent), so validation
can distinguish rollback mode from a normal accepted full rebuild.

6. Acceptance envelope

Representative validation should not require every edit to reuse everything.
It should fail only on the residual shape proven by the bundle:

- fast first publish;
- clean completion/observability integrity;
- same-file didSave exact producer reaches `program_lowering_tail`;
- `full_rebuild` rebuilds nearly all lowering units;
- no seed source and no required-full-rebuild reason;
- or the only reason is bounded-retention eviction that occurs for the normal
  large-module same-file save sequence without supersession, cancellation,
  failure, or documented capacity pressure.

## Risks

- **Incorrect reuse across incompatible text.** Mitigate with text-hash and
  changed-range validation before deriving the plan.
- **Memory growth from retained seeds.** Mitigate with bounded per-file storage
  and lifecycle cleanup when save cycles terminate or are superseded. Also expose
  eviction reasons so memory protection cannot silently become a functional
  full-rebuild fallback.
- **False confidence from aggregate metrics.** Keep the gate request-centric:
  `requested_version`, `save_cycle_sequence`, reuse source, unit counts, and
  failure reason must all come from the same trace.
- **Masking legitimate full rebuilds.** Full rebuild remains legal, but only
  with a reason that explains why reuse is unsafe or unavailable for that save
  family.
- **Overfitting to one incident bundle.** The requirement is shape-based, not
  tied to exact cursor positions or absolute line numbers.

## External References

- Rust compiler dev guide, incremental compilation in detail:
  https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html
- Salsa overview:
  https://salsa-rs.github.io/salsa/overview.html
- Salsa red-green algorithm:
  https://salsa-rs.github.io/salsa/reference/algorithm.html
