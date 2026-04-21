# Change: decompose completion pre-dispatch residual under same-file current-revision pressure

## Why

The incident bundle captured at `2026-04-21T09:41:02.078Z` on `0.4.159` /
`git f7bb44cb` changed the diagnosis again.

The recent `didSave` follow-up line is now healthy:

- all four authoritative diagnostics-save traces publish through
  `followup_semantic_path=detached_ready_artifacts`;
- all four traces expose
  `followup_ready_snapshot_zero_probe=not_ready`,
  `followup_ready_snapshot_wait_probe=not_ready`, and
  `followup_ready_snapshot_bounded_wait_winner=detached_ready_artifacts`;
- bounded wait stays small (`p95=116ms`), so save-followup is no longer the dominant seam.

The remaining representative pain moved back to completion pre-dispatch:

- one completion trace shows about `2737ms` from client write to `adapter_read`, then
  `adapter_to_dispatch_wait_ms=35362` while `server_handler_exec_ms=1`;
- another trace shows `adapter_to_dispatch_wait_ms=3161` with equally trivial handler cost;
- `response_output_handoff_send_wait_ms` stays around `0-1ms`, so response egress is not the
  dominant residual;
- local extension pre-send remains `1-3ms`, so this is not a UI-first issue either.

Archived changes already addressed earlier bottlenecks:

- `isolate-completion-pre-dispatch-ingress` introduced strict priority lanes and a single scheduler
  owner for `poll_ready()/call()`;
- `refactor-lsp-auxiliary-runtime-isolation` moved known CPU-heavy auxiliary work off the async
  runtime path;
- `refactor-44` / `refactor-46` made the didSave diagnostics follow-up stop waiting on the old
  canonical exact publication path.

So the new gap is narrower, but the exact root cause is still under-specified. The bundle proves a
backend pre-dispatch seam, not yet the exact sub-bucket inside that seam.

Current code leaves several plausible contributors:

- `read_input`, `process_scheduler`, `print_output`, and completion handoff still live under one
  joined async runtime task;
- `read_input` can stop reading stdin while staged spillover waits for lane space;
- the scheduler waits on shared `poll_ready()` before dequeueing the next request;
- completion-supporting document-sync notifications do an inline first poll and can activate a
  global completion barrier;
- same-file freshness is still encoded through shared queue/barrier semantics instead of an
  explicit token published after current-revision handoff registration.

The strongest current signal is no longer generic backlog in the abstract. The worst `35362ms`
outlier overlaps a same-file `didChange/current-revision` train, while the current bundle still
shows several-second `didChange` ready-snapshot publish/materialization chunks in the same
`examples/conf_big` workload. So the primary risk is backend pre-dispatch under same-file
document-sync/current-revision pressure, not just transport starvation as a standalone label.

This means a same-file completion can still sit behind backend reader backpressure, scheduler
progress, shared readiness, barrier handling, or same-priority ingress work that is unrelated to
the requested file's latest revision token.

## What Changes

- Extend the authoritative completion timeline / incident bundle contract so it decomposes both:
  - reader-side wait before `adapter_read`;
  - scheduler-side wait inside `adapter_read -> dispatch`.
- Require explicit same-file document ingress ownership/token publication for raw
  `didOpen`/`didChange`/`didSave`/`didClose` handoff so completion observes the latest same-file
  revision only after the corresponding current-revision handoff has actually been registered.
- Require the LSP transport runtime to isolate reader, scheduler, and output/handoff progression
  on independent async tasks or an equivalent starvation-safe execution model, but only count this
  as a latency remedy if the new buckets show real bounded waits rather than mere re-attribution.
- Extend the authoritative completion timeline / incident bundle contract with additive
  decomposition that can separate:
  - reader wait caused by local spillover/backpressure;
  - queue residence after admission;
  - shared `poll_ready()` wait;
  - completion barrier / same-file token wait;
  - residual post-ready wait before dispatch.
- Require correlation fields that can tie a completion outlier to active same-file freshness
  pressure when present:
  barrier owner, first-poll doc-sync identity, required token version, and current published token
  version/source.
- Tighten human-readable completion verdicts and mixed-load acceptance so they distinguish
  backend reader wait, queue-residence backlog, shared-readiness backlog, and same-file token
  wait, and so they stop attributing these cases to the client path.
- Tighten acceptance so the change is not considered a latency fix if the dominant seconds-scale
  wait merely moves from `adapter_to_dispatch_wait_ms` into a newly exposed bucket.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/transport_adapter.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - completion timeline / incident-bundle projection surfaces
  - transport/runtime/perf regression suites
- Follow-up relationship:
  - builds on `isolate-completion-pre-dispatch-ingress`
  - builds on `refactor-lsp-auxiliary-runtime-isolation`
  - does not reopen the `didSave` follow-up scope from `refactor-44` / `refactor-46`

## Non-Goals

- Do not rewrite all LSP scheduling or introduce a full weighted-fair scheduler for every method.
- Do not widen interactive budgets as the primary remedy.
- Do not move the problem into VS Code throttling or other client-side heuristics.
- Do not claim that same-task `join!` starvation is already proven as the sole root cause before the
  new decomposition exists.
- Do not change completion exact semantics, fail-closed guarantees, or response egress contracts
  beyond the additive observability split needed for this diagnosis.
