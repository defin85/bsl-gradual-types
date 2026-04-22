## Context

Representative `didSave` heavy follow-up no longer defaults to `shadow_state`, but the current
canonical semantic path still burns too much CPU inside local-function-summary inference.

Fresh live evidence on the current workspace shows:

- `followup_publish_semantic_path=ready_artifacts`;
- `semantic_diagnostics_query_ms=5251`;
- `semantic_diagnostics_ir_ms=3954`;
- `semantic_facts_materialize_ms=3494`;
- `local_function_summaries_ms=2343`;
- `local_function_summaries_fixed_point_ms=2335`;
- `local_function_summaries_function_count=311`;
- `local_function_summaries_scc_count=311`;
- `local_function_summaries_fixed_point_iteration_count=622`.

This is strong evidence that the current solver is spending most of its time in the fixed-point
framework itself rather than in unavoidable recursive convergence:

- `prep_ms` is effectively negligible;
- `scc_count == function_count`, so the representative workload is not dominated by large mutually
  recursive SCCs;
- the solver still performs about two iterations per function and rebuilds a file-wide summary
  snapshot on each iteration.

## Contract

- Inputs/outputs:
  - input remains the parsed program plus base environment;
  - output remains `HashMap<String, LocalFunctionSummary>` with the same semantic meaning.
- Correctness invariants:
  - singleton non-recursive SCCs MUST produce the same summary as the legacy solver;
  - self-recursive and mutually recursive SCCs MUST still converge to the same summary as the
    legacy solver;
  - `infer_global_function_call` and `semantic_identifier_call_target` MUST observe the same
    effective summaries they would have observed under the legacy full snapshot.
- Performance invariants:
  - singleton non-recursive SCCs MUST NOT enter the generic fixed-point loop;
  - recursive SCC iterations MUST NOT rebuild a full-file local-summary snapshot on each iteration;
  - the `base + overlay` lookup model MUST NOT clone or rebuild unrelated out-of-SCC summaries per
    SCC or per iteration under a different helper name.
- Out of scope:
  - broad type-inference rewrites outside the local-function-summary hotspot;
  - routing, wait-policy, or fallback-behavior changes.

## Goals / Non-Goals

- Goals:
  - remove avoidable fixed-point work for singleton non-recursive SCCs;
  - stop rebuilding file-wide local-summary snapshots on each recursive SCC iteration;
  - preserve exact semantic parity and deterministic behavior;
  - prove the improvement with representative evidence.
- Non-Goals:
  - relax exactness or latest-wins guarantees;
  - introduce a stale summary cache that outlives the current semantic-materialization target;
  - perform a full dataflow-solver rewrite for all semantic-facts stages.

## Decisions

### 1. Singleton non-recursive SCCs get a dedicated one-pass fast path

If an SCC contains exactly one function and has no self-edge, the solver can evaluate it once after
its callees are already stabilized by reverse-topological order.

That case does not need fixed-point convergence because the function does not depend on its own
current summary.

The fast path therefore:

- keeps reverse-topological ordering so callee summaries outside the SCC are already stable;
- runs one bounded body-inference pass for that function;
- publishes the final summary directly into stable state;
- skips generic fixed-point iteration accounting for that SCC.

Self-recursive singletons MUST stay on the recursive path because they do depend on their own
previous summary.

### 2. Recursive SCCs switch from full-file snapshot rebuild to `base + overlay`

The current implementation rebuilds one full `HashMap<String, LocalFunctionSummary>` snapshot for
all local routines on every fixed-point iteration.

The optimized design replaces that with a logical lookup split:

- `base`: already stabilized summaries for functions outside the current SCC;
- `overlay`: current-iteration summaries for functions inside the current SCC.

Lookup rules:

- first consult the overlay for names inside the active SCC;
- fall back to the stable base for all other names.

This preserves the semantic contract of “current SCC sees the latest in-SCC summaries and stable
out-of-SCC summaries” without rebuilding unrelated file-wide entries each iteration.

The contract here is semantic and costed, not merely nominal:

- it is acceptable to materialize the active SCC overlay for the current iteration;
- it is not acceptable to clone, rebuild, or remap unrelated out-of-SCC summaries per SCC or per
  iteration as part of serving the base view.

### 3. Keep the lookup refactor narrow

The current read surface for local function summaries is already narrow: local summaries are read
through the environment in the global-call and local-call-target paths.

The implementation should therefore introduce one narrow lookup abstraction instead of rewriting
the broader environment model.

That keeps the blast radius small and makes parity testing tractable.

### 4. Acceptance is evidence-gated, not only test-gated

Synthetic regressions alone are insufficient for this change.

The representative `conf_big` save-follow-up must show that:

- `local_function_summaries_fixed_point_ms` is no longer the dominant steady-state cost for a
  singleton-SCC-heavy workload;
- `fixed_point_iteration_count` drops materially versus the current baseline when the workload is
  mostly singleton and non-recursive;
- attribution remains truthful for recursive SCCs that still require convergence.

## Alternatives Considered

### 1. Only add singleton fast path, keep full snapshot rebuild for recursive SCCs

Rejected as the final design.

It would remove one obvious waste, but the current representative evidence already shows non-trivial
time in snapshot rebuild and recursive framework overhead. Keeping the full-file rebuild would leave
an avoidable `O(total_functions)` cost inside each remaining recursive iteration.

### 2. Replace the whole local summary solver with a new global worklist engine

Rejected for now.

That is a much larger algorithmic rewrite with a wider semantic risk surface. The current evidence
already points to two narrower sources of waste that can be removed without changing the broader
type-inference architecture.

### 3. Cache full-file local summary snapshots across semantic-diagnostics requests

Rejected.

The change target is current same-version exact semantic materialization. Cross-request caching
would enlarge invalidation complexity and risks hiding stale-summary bugs instead of removing the
hot-path waste currently visible inside one canonical build.

## Risks / Trade-offs

- Lookup abstraction bugs could silently change which summary wins for names inside or outside the
  active SCC.
- Self-edge detection must remain exact; incorrectly classifying a self-recursive singleton as
  non-recursive would be unsound.
- Overlay-based lookup must stay deterministic and allocation-conscious; otherwise the change could
  trade one kind of overhead for another.

## Validation Strategy

- Add parity regressions for:
  - singleton non-recursive local routines;
  - self-recursive singleton routines;
  - mutually recursive SCCs;
  - files with many unrelated locals plus one recursive SCC.
- Keep diagnostics-save observability exporting:
  - `local_function_summaries_ms`;
  - `local_function_summaries_prep_ms`;
  - `local_function_summaries_fixed_point_ms`;
  - `local_function_summaries_snapshot_build_ms`;
  - `local_function_summaries_body_infer_ms`;
  - `function_count`;
  - `scc_count`;
  - `fixed_point_iteration_count`;
  - `singleton_fast_path_count`;
  - `recursive_scc_count`.
- Refresh representative live evidence on `conf_big`.

## Quality Gates

- Representative `didSave` heavy follow-up still publishes through `ready_artifacts`.
- Representative evidence shows material reduction in `local_function_summaries_ms` versus the
  current baseline.
- On singleton-heavy representative load, `fixed_point_iteration_count` is no longer approximately
  `2 * function_count`.
- Representative evidence distinguishes singleton fast-path wins from recursive residual through
  explicit bounded counters rather than inference from one aggregate iteration count.
- Recursive SCC parity tests stay green.
