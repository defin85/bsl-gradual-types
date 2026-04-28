# Change: bound didChange ready-install type-index wait after detached diagnostics-ready publication

## Why

`refactor-54-didsave-exact-materialization-latency-bounding` closed the fresh
diagnostics-save latency contour for the installed-runtime incident captured at
`2026-04-24T14:22:42Z`: the representative current-source run no longer shows slow
`save_fastlane` first publish, no longer accepts `program_lowering` full rebuild before detached
diagnostics-ready publication, and the same-version save follow-up reaches
`detached_ready_artifacts` inside the bounded wait window.

That same current-source report still records a separate residual:

```text
did_change_ready_snapshot_materialization_ms p50=42597 p95=43758 count=4
```

The report also shows this residual is not the accepted refactor-54 save-followup path:

```text
followup_semantic_path_detached_ready_artifacts=4
max_followup_ready_snapshot_bounded_wait_elapsed_ms=47
max_followup_ready_snapshot_parse_exec_ms=163
max_followup_publish_elapsed_ms=2261
```

Per-cycle probes show the canonical ready snapshot and exact type-index state are lagging after the
fast detached diagnostics-ready path:

- `observed_version_after_timeout` is the newer stage2 revision;
- `ready_snapshot_state_after_timeout.file_version` is still the previous stage1 revision;
- `exact_ready_after_timeout=false`;
- `type_index_task_state_after_timeout.phase=computing`;
- `background_parse_task_state_after_timeout.phase=Some(Materializing)`;
- `type_index_parse_snapshot_meta_after_timeout=null`.

Code inspection explains why the histogram can be high without contradicting refactor-54:
`record_detached_diagnostics_ready_artifact_v2` runs before
`wait_for_exact_type_index_before_ready_install_v2`, while
`did_change_ready_snapshot_materialization_ms` is recorded only after the canonical ready snapshot
install path finishes waiting for exact type-index readiness. The residual therefore belongs to
canonical ready-install/type-index readiness after detached diagnostics-ready publication.

There is also a source-attribution risk in the same worker. The worker captures `source_label`
before debounce and before same-version `didSave` promotion can mutate a running `didChange`
target to `DidSave`. Later metrics can still be attributed to the initial source even when the
effective target was promoted. This change must make original source, effective source, and
promotion/retarget evidence explicit before treating `did_change_*` materialization histograms as
authoritative.

## What Changes

- Add `bsl-intellisense-v2` requirements that canonical ready snapshot install after detached
  diagnostics-ready publication must either remain bounded by an explicit checked-in readiness
  envelope or export a truthful blocker for exact type-index readiness.
- Require a clear distinction between detached diagnostics-ready publication and canonical live
  ready install. Detached artifacts can satisfy diagnostics follow-up, but they are not proof that
  the live ready snapshot and exact type-index are installed.
- Require ready-install/type-index wait observability: elapsed wait, explicit ceiling/deadline
  class, outcome, task phase, active requested version, exact readiness, parse snapshot metadata,
  serve-only blocked state when available, and the current canonical ready snapshot version.
- Require background parse snapshot materialization metrics, phase metrics, and lifecycle labels to
  use effective source attribution after same-version promotion/retarget, while preserving the
  original source as evidence.
- Extend representative p56 validation so high canonical materialization latency fails unless the
  report proves a truthful reason such as supersession, cancellation, latest-version mismatch,
  continuity loss, type-index invalidation, or serve-only blocked readiness.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - type-index precompute scheduling/promotion and exact-ready install wait
  - ready-parse-snapshot materialization metrics and source attribution
  - `backend/src/bin/lsp_server/server/core/tests/live_reports/representative_bundle_live.rs`
  - representative incident-bundle and live-report projections
- Follow-up relationship:
  - follows `refactor-54-didsave-exact-materialization-latency-bounding`;
  - does not reopen refactor-54 save-fastlane or detached diagnostics-ready acceptance;
  - keeps canonical exact gates for interactive consumers intact;
  - keeps VS Code UI, completion transport, and response egress out of scope unless a newer bundle
    shows direct evidence that those layers are material again.

## Non-Goals

- Do not widen ready-install, bounded-wait, or relief-valve budgets as the primary remedy.
- Do not publish canonical live ready snapshots before exact type-index readiness is proven.
- Do not treat detached diagnostics-ready artifacts as canonical live exact readiness for
  completion, hover, definition, signatureHelp, type-at-position, or equivalent interactive
  consumers.
- Do not fold this residual into refactor-54 acceptance. The accepted save-followup path is fast in
  the current-source report; this change targets the later canonical materialization/type-index
  wait.
- Do not start in `vscode-extension/` dispatch or UI rendering for this latency class without fresh
  contradictory bundle evidence.
