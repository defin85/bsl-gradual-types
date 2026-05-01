## Context

The current snapshot readiness contract is intentionally file-scoped and authoritative:

- request: `bsl/getSnapshotStatus`
- notification: `bsl/snapshotStatus`
- state vocabulary: `idle | building | ready | stale | shadow_only | failed`

The VS Code extension already renders a dedicated right-side status bar item and an Observability `Snapshot Readiness` section. Both surfaces are currently limited by the coarse DTO: they can show state, revisions, exactness, task state, phase, trigger, and a bounded fallback reason, but not a concrete explanation of why the active file is stuck in `shadow_only` or `failed`.

The existing cache dashboard is workspace/cache-scoped. Snapshot readiness is active-file-scoped and live-runtime-scoped, so merging them would blur ownership and invite incorrect readiness reconstruction from cache metrics.

## Goals / Non-Goals

Goals:

- Make `shadow_only`, `failed`, `stale`, and long-running `building` states explainable from the live status surface.
- Preserve `bsl/getSnapshotStatus` as the source of truth for file-scoped readiness.
- Add enough structured detail for normal diagnosis without requiring an incident bundle.
- Keep the status bar compact and move details into the existing Observability Tree View.
- Keep fields bounded and low-cardinality where they can affect notifications, logs, or UI grouping.

Non-goals:

- No change to snapshot build semantics, exact readiness gates, or fallback behavior.
- No new mutating recovery command beyond existing refresh/prime/export style actions.
- No Webview-first dashboard.
- No inference of readiness from cache dashboard, metrics, diagnostics save timeline, or completion timeline.

## Decisions

### 1. Extend the DTO with optional structured diagnostics

The base v1 fields remain valid. The next schema version adds optional sections:

- `reason`: bounded reason code plus short operator-facing message;
- `artifacts`: readiness of relevant artifacts such as `shadowState`, `readyParseSnapshot`, `exactTypeIndex`, and completion/current head where the server can observe them;
- `worker`: current target, phase, trigger, start/age, and supersession/cancellation facts;
- `lastFailure`: last explicit failure for the same subject/revision, including stage, reason, message, and timestamp;
- `recommendation`: bounded next step hint such as refresh, wait, prime exact index, or export bundle.

The schema must define these sections as optional so older or narrower servers can omit facts they do not own. For servers that advertise the new schema version, each section must be populated when authoritative data is available and omitted only when the fact is genuinely unknown or unsupported.

The server should omit unknown sections rather than guessing. Clients render missing sections as unavailable/unknown, not as healthy.

The payload must not include source text, full file contents, raw stack traces, or unbounded diagnostic blobs. Failure messages are detail text only and should be length-capped before they reach status-bar tooltips, tree items, logs, or history.

### 2. Keep notifications coalesced; fetch details on demand

Live notifications should continue to avoid unbounded micro-step noise. They may carry the same structured fields when already cheap to compute, but the canonical path for fresh detail is still `bsl/getSnapshotStatus`.

The client should keep a bounded per-URI transition history derived from accepted monotonic updates. History is client-side diagnostic context, not a replacement for server truth.

The history bound should be explicit in code and tests. The first implementation should only keep recent transitions for active or recently observed URIs and should drop history on reset/dispose so the extension cannot accumulate unbounded per-file state across long editing sessions.

### 3. Use existing Observability Tree View as the primary detail surface

The status bar remains a compact signal:

- `BSL Snap: ready vN`
- `BSL Snap: building vN`
- `BSL Snap: shadow-only vN`
- `BSL Snap: failed vN`

Its tooltip should show the most useful summary fields, including reason, last failure, worker age, and artifact summary when available.

Clicking the status bar should focus/reveal `BSL Analyzer: Observability -> Snapshot Readiness`. The Tree View should render sections for:

- Summary
- Why
- Artifacts
- Worker
- Last Failure
- Recent Transitions
- Actions

This fits the existing VS Code extension structure and keeps the first implementation lighter than a Webview dashboard.

The command should use the existing `bslAnalyzer.observability` Tree View surface and reveal or focus the Snapshot Readiness node when possible. If direct `TreeView.reveal` cannot be used for a particular VS Code state, the fallback is to focus the Observability view and refresh it, not to open a new Webview.

### 4. Keep cache dashboard separate

The cache dashboard may include a link/action to open Snapshot Readiness, but it must not own snapshot status or display synthetic readiness. Cache metrics can explain cache health; they cannot explain whether the active file has an exact current snapshot.

## Risks / Trade-offs

- More DTO fields can become noisy if they use unbounded messages or high-cardinality internals.
  - Mitigation: bounded reason/stage/status codes, optional free text only for failure detail and tooltips.
- UI can accidentally imply `shadow_only` is acceptable exact readiness.
  - Mitigation: render `shadow_only` as degraded and explicitly show which exact artifact is missing.
- Notification payloads can grow or flicker.
  - Mitigation: keep coalescing and let manual/hydrate request provide fresh detail.
- Backend may not know every artifact state immediately.
  - Mitigation: omit unknown fields and render them explicitly as unknown.

## Rollout

1. Add DTO/schema v2 fields and backend computation for reason, artifacts, worker, and last failure.
2. Preserve existing v1-compatible fields for old clients and tests.
3. Update VS Code TypeScript DTOs and snapshot status cache/history.
4. Expand status bar tooltip and Observability `Snapshot Readiness` tree.
5. Add targeted backend and VS Code extension tests.
6. Validate the OpenSpec change and run the smallest relevant test set during implementation.
