## Context

`isolate-completion-pre-dispatch-ingress` already moved the transport path to a
`reader -> single-owner scheduler + strict priority lanes` model, and
`refactor-lsp-auxiliary-runtime-isolation` already removed known CPU-heavy auxiliary work from the
interactive async path.

The fresh bundle on `f7bb44cb` shows what is still left after those fixes:

- save-followup is no longer the dominant residual;
- response egress is not the dominant residual;
- extension pre-send is not the dominant residual;
- the remaining outliers are still concentrated before completion dispatch, even when the actual
  handler cost is `1ms`.

That means the next gap is narrower than "general backlog" and narrower than
"auxiliary CPU still runs inline". But the bundle still proves only a coarse backend
pre-dispatch seam. It does not yet prove that same-task `join!` starvation is the exact root
cause.

The strongest current signal is more specific than that coarse label:

- the worst completion outlier spends `35362ms` in `adapter_read -> dispatch` while handler cost is
  `1ms`;
- the same bundle still shows several-second `didChange/current-revision` publish/materialization
  chunks on `examples/conf_big`;
- that outlier overlaps a train of same-file `didChange/current-revision` work rather than an idle
  transport path.

So the best current explanation is backend pre-dispatch under same-file
document-sync/current-revision pressure. Transport runtime topology may still contribute, but the
causal model has to include file-local freshness pressure rather than treating the seam as generic
queue starvation.

The current code exposes several plausible contributors inside that seam:

1. task-level starvation inside the transport runtime itself, because `read_input`,
   `process_scheduler`, `print_output`, `process_server_tasks`, and completion handoff progression
   still share one joined async task;
2. reader-side admission backpressure, because `read_input` can stop reading stdin while staged
   spillover waits for lane space;
3. scheduler-side shared readiness, because the scheduler waits on `poll_ready()` before dequeue;
4. global completion barrier coupling, because completion-supporting document-sync notifications do
   an inline first poll and can activate a global barrier;
5. implicit same-file ingress ordering, where completion-supporting document-sync notifications and
   interactive completion still depend on a shared lane/FIFO path rather than on an explicit
   same-file ingress owner/token published after current-revision handoff registration.

So the change should not reopen the whole pre-dispatch-lane design, but it also should not pretend
that the exact root cause is already known. It should decompose the seam first, then harden the
runtime topology that remains after that design, and finally prove that the dominant bucket is
actually bounded rather than merely re-labeled.

## Goals / Non-Goals

- Goals:
  - separate reader-side wait before `adapter_read` from scheduler-side wait inside
    `adapter_read -> dispatch`;
  - make transport reader, scheduler, and output progression starvation-safe relative to each
    other when the new decomposition proves that runtime topology contributes materially;
  - make same-file document ingress ownership explicit before interactive completion depends on it;
  - decompose `adapter_read -> dispatch` into truthful bounded buckets that operators can read
    directly from the authoritative timeline and incident bundle;
  - prevent future changes from claiming success when the same seconds-scale residual only moves
    into a newly exposed bucket;
  - preserve current fail-closed, latest-wins, and exactly-once terminal semantics.
- Non-Goals:
  - a generic fair scheduler for every LSP method;
  - a rewrite of completion exact/query-body internals;
  - a new UI/client throttling policy;
  - another save-followup or output-egress change.

## Decisions

### 1. Diagnose two windows, not one coarse ingress seam

The incident family now includes two distinct windows:

- `client write -> adapter_read`;
- `adapter_read -> dispatch`.

They are related, but not identical:

- the first window can include local reader backpressure, because `read_input` may stop reading
  while staged requests wait for lane space;
- the second window can include queue residence, shared `poll_ready()` wait, completion barrier
  wait, same-file token wait, or residual post-ready delay before dispatch.

The change therefore treats `client write -> adapter_read` and `adapter_read -> dispatch` as
separate seams in both design and acceptance. A later runtime isolation fix is not considered
complete if it only shrinks one seam while leaving the other unmeasured or if it merely moves the
same seconds-scale residual between buckets.

### 2. Instrument before declaring exact root cause

The existing single-owner `poll_ready()/call()` rule is still correct, but the current telemetry is
too coarse to prove which internal wait dominates.

The new contract therefore remains additive and keeps the old umbrella fields while exposing more
precise bounded buckets.

Recommended additive fields:

- `adapter_read_started_at_ms`;
- `adapter_parse_completed_at_ms`;
- `read_loop_wait_reason`;
- `read_loop_wait_ms`;
- `pending_completion_spillover_depth`;
- `pending_general_request_staged`;
- `admission_try_enqueue_at_ms`;
- `admission_lane`;
- `admission_lane_depth_before`;
- `admission_lane_depth_after`;
- `admission_enqueue_outcome`;
- `admission_spillover_outcome`;
- `admission_enqueued_at_ms`;
- `admission_queue_wait_ms`;
- `scheduler_woke_at_ms`;
- `scheduler_poll_ready_entered_at_ms`;
- `scheduler_poll_ready_resolved_at_ms`;
- `scheduler_poll_ready_wait_ms`;
- `scheduler_dequeued_at_ms`;
- `completion_barrier_active_at_dequeue`;
- `completion_barrier_generation`;
- `completion_barrier_owner_method`;
- `completion_barrier_owner_uri`;
- `completion_barrier_owner_version`;
- `completion_barrier_wait_ms`;
- `scheduler_service_call_started_at_ms`;
- `scheduler_service_call_returned_at_ms`;
- `scheduler_service_call_sync_exec_ms`;
- `doc_sync_first_poll_exec_ms`;
- `doc_sync_first_poll_outcome`;
- `doc_sync_first_poll_method`;
- `doc_sync_first_poll_uri`;
- `doc_sync_first_poll_version`;
- `same_file_ingress_token_required_version`;
- `same_file_ingress_token_published_at_ms`;
- `same_file_ingress_token_source`;
- `same_file_ingress_token_wait_ms`;
- `scheduler_ready_to_dispatch_wait_ms`.

`adapter_to_dispatch_wait_ms` remains the compatibility umbrella for the full server-side interval
between `adapter_read_at_ms` and the earliest dispatch boundary.

With these fields, a bundle can distinguish:

- local backend wait before the adapter records the request;
- queue residence after admission;
- shared `poll_ready()` wait;
- barrier or same-file freshness wait;
- which doc-sync/barrier owner created that freshness wait when it exists;
- residual post-ready wait before dispatch.

### 3. Keep a single scheduler owner, but stop running the whole transport runtime on one joined task

The existing single-owner `poll_ready()/call()` rule is still correct. The problem is not that
multiple actors call `poll_ready()`. The problem is that reader, scheduler, output progression, and
handoff work still share one async task boundary.

This change therefore keeps the single scheduler owner but requires separate async tasks for:

- transport input/read/decode/classify;
- scheduler/readiness/call ownership;
- output writer / flush progression;
- completion handoff workers.

This follows the general guidance in:

- Tokio `join!`: same-task multiplexing can starve sibling branches if one branch keeps the task
  busy before yielding;
- Tokio `spawn` / `JoinSet`: independent tasks are the normal way to isolate unrelated progression;
- Tower `Service`: readiness/backpressure should still be owned by one caller.

Consequence: the change is not an instrumentation-only patch. Runtime topology remains part of the
normative remedy, but it must now be justified by the newly decomposed latency buckets rather than
by the coarse umbrella seam alone.

### 4. Admission ordering must depend on explicit same-file ingress ownership, not only on shared FIFO

The current architecture relies on the fact that completion-supporting document-sync notifications
share a priority lane with completion. That was enough to prevent general backlog from hiding
same-file `didChange`, but it is still too implicit:

- unrelated same-priority work can still occupy scheduler progress;
- completion does not have a first-class "the relevant file token is ready" signal before dispatch;
- the bundle cannot separate "queue residence before my file token exists" from "queue residence
  even though my file token was already published".

So the new architecture should introduce explicit same-file document ingress ownership/token
publication:

- raw `didOpen`/`didChange`/`didSave`/`didClose` for one file stay serialized by a per-file owner;
- this owner publishes a bounded latest-ingress token for the file only after the required current
  revision handoff is registered, not at dispatcher-event emission time;
- later completion for that file depends on that token, not on unrelated same-priority FIFO
  residence.

This keeps raw document ordering correct without requiring a global FIFO interpretation for all
interactive traffic.

### 5. Human-readable verdicts must distinguish reader wait, queue residence, shared-readiness backlog, and same-file token wait

Today the best derived verdict available for this family is effectively
`adapter_before_dispatch_dominant`, which is too coarse now that the diagnosis moved deeper.

The new projection should therefore distinguish at least:

- `reader_backpressure_dominant`;
- `admission_queue_dominant`;
- `scheduler_poll_ready_dominant`;
- `completion_barrier_dominant`;
- `same_file_ingress_token_dominant`;
- `adapter_before_dispatch_dominant` as a backward-compatible umbrella when the finer split is not
  present.

Client-side ingress verdicts must remain fail-closed: if the server-side additive split already
explains the wait, the extension must not attribute the same request to client ingress.

### 6. Acceptance must prove runtime progress, same-file visibility, and no bucket shifting

The new change is not complete if only synthetic queue tests pass. It must prove:

- reader progress continues while scheduler work is stalled;
- ready output flush can progress while other scheduler work is still blocked;
- same-file `didChange`/`didSave` ingress ownership reaches a later completion without depending on
  unrelated queued traffic;
- representative evidence distinguishes reader-side wait, queue residence, shared readiness,
  barrier/token wait, and residual post-ready dispatch delay;
- representative evidence can correlate the worst completion outlier with active same-file
  document-sync/current-revision work, barrier ownership, and token publication state instead of
  leaving the causal chain implicit;
- the dominant seconds-scale outlier no longer appears in any of those buckets for the accepted
  representative profile, rather than merely moving from `adapter_to_dispatch_wait_ms` into a new
  sub-field;
- representative mixed-load evidence on `examples/conf_big` keeps the new admission buckets within
  the existing interactive budget family.

## Alternatives Considered

### 1. Raise queue capacities or widen budgets

Rejected.

That only hides queue residence under a longer clock and weakens the operator signal.

### 2. Reopen the old "strict priority lanes" change as the same diagnosis

Rejected.

The current runtime already implements that design. The bundle proves the remaining residual is what
survived after that design, not a restatement of the original problem.

### 3. Fix it only with more observability fields

Rejected as a final state.

Observability decomposition is necessary, but it does not remove the same-task starvation risk by
itself.

### 4. Split tasks immediately and accept the coarse diagnosis as sufficient

Rejected.

Task isolation is plausible, but the current bundle still does not prove whether the dominant wait
is local reader backpressure, queue residence, shared readiness, completion barrier/token wait, or
another residual inside the same umbrella seam. Shipping task isolation without the new
decomposition risks turning a latency bug into a measurement shift.

### 5. Move more logic into extension-side throttling

Rejected.

The bundle already shows the dominant wait inside backend transport/admission. Client-side
throttling would hide the root cause instead of fixing it.

## Validation Strategy

- Add transport regressions where a stalled scheduler branch cannot stop later adapter reads,
  cancel classification, or already-ready output progression.
- Add reader-backpressure regressions where staged spillover can no longer create unbounded local
  wait before `adapter_read` without being explicitly attributed.
- Add same-file ingress regressions where a file-local `didChange`/`didSave` token becomes
  available only after current-revision handoff registration and a later completion for that file
  proceeds without waiting behind unrelated queued work.
- Extend incident-bundle and completion timeline regression coverage so the new additive buckets
  appear truthfully in projections, including reader wait, shared readiness, barrier/token wait,
  and residual post-ready delay.
- Require the representative evidence to preserve a correlation slice for the worst outlier:
  active barrier owner, same-file token version/source, and active same-file `didChange` freshness
  pressure when present.
- Refresh representative `examples/conf_big` mixed-load evidence and keep it tied to the existing
  interactive budget family rather than inventing a new looser budget.
- Treat the change as incomplete if the accepted representative trace still shows seconds-scale wait
  in any newly exposed bucket even when the old umbrella `adapter_to_dispatch_wait_ms` looks
  better.

## External References

- Tokio `join!`: https://docs.rs/tokio/latest/tokio/macro.join.html
- Tokio `spawn`: https://docs.rs/tokio/latest/tokio/task/fn.spawn.html
- Tokio `JoinSet`: https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html
- Tokio `mpsc::Sender`: https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Sender.html
- Tokio `watch`: https://docs.rs/tokio/latest/tokio/sync/watch/
- Tower `Service`: https://docs.rs/tower/latest/tower/trait.Service.html
