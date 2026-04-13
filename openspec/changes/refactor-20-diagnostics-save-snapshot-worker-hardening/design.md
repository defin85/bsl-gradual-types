## Context

`refactor-17` already changed `didSave` branch ordering for the `in_flight_same_version` case, but
the latest incident bundle still shows the same save cycle timing out on exact snapshot reuse.

The new lifecycle metrics reveal why:

- `didSave` is waiting on an exact same-version worker that was originally scheduled by
  `didChange`;
- that worker usually never materializes a ready snapshot before the bounded wait expires;
- obsolete workers are classified as `aborted`, which is consistent with outer-task cancellation
  but not with a guaranteed stop of already-started blocking parse work;
- `bsl.getCurrentContext` still frequently enters `parser_coordinator` instead of reusing the same
  in-flight exact work.

So the missing piece is not another branch reorder. It is a worker control model that lets:

- superseded exact workers stop before they keep burning parser capacity;
- `didSave` promote the one exact worker it already needs;
- `bsl.getCurrentContext` reuse exact work instead of racing it.

## Goals / Non-Goals

- Goals:
  - let the same save cycle benefit from an already-started exact same-version snapshot worker
  - stop obsolete snapshot workers from monopolizing parser/blocking capacity after supersession
  - reduce duplicate parse pressure from `bsl.getCurrentContext`
  - preserve no-stale-publish and current completion isolation guarantees
- Non-goals:
  - no larger or unbounded wait before `shadow_state` fallback
  - no duplicate `didSave` parse worker for identical text/version
  - no move of snapshot-backed install to the interactive writer queue
  - no promise that every mixed-load parser-contention profile will materialize exact same-version
    artifacts before the existing bounded wait deadline
  - no redesign of unrelated completion transport or `didChange` replay semantics

## Design

### 1. Background ready-snapshot workers need a shared control plane

The current task registry tracks only `requested_version`, `text_hash`, `source`, and the outer
`JoinHandle`. That is enough to observe an in-flight task, but not enough to manage it.

The worker entry should be extended with shared state such as:

- cooperative cancellation flag
- promotion request flag / requested lane bias
- materialization notification primitive
- worker start timestamp / task-local identity for observability and tests

This makes supersession, promotion, and materialization first-class state transitions instead of
side-effects of `handle.abort()`.

### 2. Supersession must be cooperative, not abort-only

Outer `tokio::task::JoinHandle::abort()` is not sufficient once blocking parse work has already
started. The snapshot worker therefore needs cooperative stop points:

- before or during debounce
- before entering parse build
- inside incremental/full parse where cancellation can still be observed

The parser-coordinator snapshot path should gain a cancellation-aware variant so obsolete exact
workers stop before they keep holding blocking threads or the parser mutex after a newer revision
already won.

### 3. didSave should promote the exact worker it already depends on

When `didSave` heavy follow-up observes `in_flight_same_version`, it should not merely wait and
hope that the worker finishes under generic background scheduling. Instead, it should request
promotion of that exact worker into `did_save_followup` admission/CPU priority for the
materialization stage and then spend the existing bounded wait on that promoted worker.

Key constraints:

- promotion applies only to an existing exact same-version worker with matching text/version
- no duplicate `didSave` worker is started for the same `(file_id, requested_version, text_hash)`
- if the promoted worker is superseded, mismatched, cancelled, or still not ready at deadline,
  fallback stays truthful (`shadow_state` or generic pipeline)

### 4. bsl.getCurrentContext should reuse exact in-flight snapshot work

`bsl.getCurrentContext` already has latest-only generation semantics, but it still tends to start a
fresh parser-coordinator parse when a same-file exact snapshot worker is already doing equivalent
work for the same text/version.

The current-context path should:

- consume a ready exact snapshot immediately when available
- otherwise spend a short bounded reuse budget on the exact same-version worker's materialization
- only after that fall back to the existing broker/leader parse path

This keeps newest-generation-wins semantics intact while reducing duplicate parser contention in
the very window that matters to `didSave`.

### 5. Keep snapshot-backed install on the background writer path

The install step for `SetFileWithSnapshot` is explicitly kept off the interactive writer queue
today so completion handoff is not blocked by slow snapshot installs. That invariant should remain.

This change is about:

- worker materialization
- cooperative supersession
- exact-task promotion/reuse

It is not about turning snapshot install into interactive writer work.

## Alternatives Considered

### 1. Increase the bounded didSave wait budget

Rejected. The latest bundle already shows that timeout alone is not the primary defect: the exact
worker usually never materializes because it is superseded or starved first. A larger wait would
mainly stretch the stall window.

### 2. Start a dedicated didSave snapshot worker for the same version

Rejected. That would duplicate parse work for identical text/version and increase parser
contention exactly in the hot path we are trying to calm down.

### 3. Move snapshot-backed install onto the interactive writer queue

Rejected for this change. The code already documents that slow snapshot installs must not block
interactive completion handoff. Promotion should target worker materialization, not writer apply.

## Risks / Trade-offs

- Cancellation-aware parse plumbing touches a sensitive low-level path in `parser_coordinator`.
- Promotion could starve ordinary background work if applied too broadly.
- A short current-context reuse wait that is too generous could regress cursor responsiveness.

## Mitigations

- Gate promotion strictly on exact same-version task identity and matching text/version.
- Keep both `didSave` and current-context waits bounded and reuse the existing `didSave` budget.
- Add regression coverage for cooperative supersession, no-duplicate-worker semantics, and
  newest-generation current-context behavior.
