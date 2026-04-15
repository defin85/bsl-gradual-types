## Context

The current end-to-end materialization metric tells us that exact ready-snapshot work is slow, but
not why it is slow. That is enough for alarm, not enough for optimization.

For the current incident the key unknown is whether the `~3720ms` is dominated by:

- blocking parse execution;
- late cancellation / retarget checks;
- a post-parse delay before ready install;
- or side-work such as document symbols that should be separated from exact readiness.

## Goals / Non-Goals

- Goals:
  - break exact ready-snapshot latency into a small bounded set of actionable phases;
  - let `didSave` timeout bundles say which producer phase lost to the wait budget;
  - keep symbol/outline side-work measurable without blaming it on readiness.
- Non-Goals:
  - no change to wait budgets;
  - no parser-base policy rewrite;
  - no new high-cardinality per-request tracing contract.

## Decisions

### Decision: phase timers stop at ready install

The exact readiness path should end when the ready snapshot is installed and queryable for the
exact target revision.

Anything that happens after that point, including documentSymbol / outline building, must be
reported separately so operators can tell whether exact readiness was late or whether secondary
side-work was late.

### Decision: timeout attribution must reuse runtime phase state

The producer already has a coarse phase machine (`Waiting`, `Parsing`, `Materializing`). This
change should extend phase accounting around that machine instead of introducing a second unrelated
state model.

Bundles should be able to say not just "timeout", but "timeout while exact worker was still
parsing" or "timeout while exact worker had already finished parse and was in post-parse install
window".

### Decision: phase list stays intentionally small

The bundle needs only the phases that can drive engineering decisions:

- parse execution;
- post-parse/pre-materialization;
- ready install/materialization;
- symbol/outline side-work after ready install.

More detail would add noise faster than value.

## Alternatives Considered

### Keep a single end-to-end materialization histogram

Rejected. It cannot explain which next optimization matters.

### Add full tracing spans for every internal checkpoint

Rejected. That is too heavy for the default observability path and too hard to keep stable.

## Risks / Trade-offs

- Phase timing can become misleading if timers overlap or if symbol side-work is not clearly
  detached from exact readiness.
- Operators may overfit to one incident if the bundle summary does not also expose counts across
  repeated samples.

## Migration Plan

1. Add phase timing and timeout-attribution fields to ready-snapshot producer lifecycle.
2. Surface them in incident-bundle export and diagnostics-save timeline payloads.
3. Add repo-local evidence that identifies the dominant phase on a real large module.
