## Context

The runtime now knows much more about snapshot truth than the product surfaces show:

- whether an exact same-version worker is still in flight;
- whether a ready snapshot already exists for the requested revision;
- whether the system fell back to `shadow_state` instead of exact snapshot artifacts.

That state is already useful during incident analysis, but it is too late and too hidden for normal
editing workflows. Users need a visible answer while they type or save. `bsl-agent` operators need
the same answer in the read-only MCP UI for tracked documents.

## Goals / Non-Goals

- Goals:
  - expose live snapshot readiness truthfully for active VS Code documents;
  - reuse the same bounded vocabulary in `bsl-agent` read-only HTTP UI;
  - avoid reconstructing readiness from retrospective observability surfaces;
  - keep the UI low-noise and fail-closed when unsupported.
- Non-goals:
  - no change to actual snapshot generation semantics or budgets;
  - no new mutating controls or operator actions;
  - no requirement to show a progress widget for every workspace file;
  - no dependence of `bsl-agent` on `bsl-backend`.

## Decisions

### 1. Introduce one bounded snapshot-readiness vocabulary

The same conceptual states must mean the same thing everywhere. The change introduces a shared
bounded vocabulary centered on:

- `state`: `idle | building | ready | stale | shadow_only | failed`
- `exact`: whether the currently reported ready state matches the requested revision exactly
- `task_state`: whether there is a matching in-flight or ready worker
- optional coarse `phase`: `waiting | parsing | materializing`
- `schemaVersion`: contract/schema version for the DTO itself, distinct from document or session
  revision fields

The state names are locked to the following meanings:

- `idle`: the subject is known, but there is no exact-ready snapshot, no degraded current answer,
  and no matching in-flight rebuild to surface yet;
- `building`: a matching rebuild is in flight and exact readiness is not available yet;
- `ready`: exact-ready artifacts exist for the requested revision;
- `stale`: a ready artifact exists, but it is older than the requested revision and the surface is
  not claiming current `shadow_only` truth;
- `shadow_only`: the current answer is based on `shadow_state`/overlay truth rather than exact
  ready artifacts;
- `failed`: the last attempted rebuild for the subject ended in an explicit error and no truthful
  `ready` or `shadow_only` answer is currently available.

This keeps UI surfaces truthful without exposing unbounded internals.

### 2. LSP gets request + notification, not timeline polling

`diagnostics save timeline` and `observability metrics` are retrospective and the wrong transport
for live UX. The LSP side should instead expose:

- `bsl.getSnapshotStatus` for hydrate/manual fetch
- `bsl/snapshotStatus` for live deltas

The notification path should be coalesced by URI and emit only meaningful state transitions so the
extension does not have to poll aggressively during typing.

The contract must also be robust to out-of-order delivery:

- the server must keep `updatedAtMs` monotonic per URI;
- the extension must ignore an older update for the same URI once it has already observed a newer
  `updatedAtMs`;
- the extension should keep one in-memory snapshot-status store and let both the status bar and the
  observability detail read from that same cache instead of independently polling the server.

### 3. VS Code uses a dedicated contextual status bar item for the short answer and Observability for detail

The extension already has an existing observability container and a workspace/global status bar
item. The least
surprising UI is:

- a dedicated right-side status bar item for the active BSL editor;
- the existing left-side status/progress item remains responsible for workspace/global concerns
  such as startup, server connectivity, and indexing;
- detailed snapshot-readiness section inside existing observability UI.

The status bar answers "ready or not, and why"; observability answers "which revision, which phase,
which fallback".

### 4. `bsl-agent` uses a read-only parity endpoint plus the same UI vocabulary

`bsl-agent` has no "active editor", so it needs a session-oriented read-only list of tracked
documents. The parity API should expose a read-only snapshot-status endpoint under the existing
ready-session selection rules, and the MCP UI should render those entries next to existing
sessions/jobs diagnostics.

For this change, "tracked documents" is locked to the deterministic union of:

- documents with an overlay in the current session;
- documents present in the current hot set.

The endpoint should return entries in deterministic path order and should return an empty list,
not an error, when the selected ready session currently tracks no documents.

This keeps `bsl-agent` within its read-only contract and reuses the same operator vocabulary.

### 5. Fail closed on unsupported or partial data

If the connected server does not support snapshot status, the extension must not invent readiness
from timelines, diagnostics events, or stale caches. The same applies to MCP UI: missing readiness
data should stay explicitly unavailable rather than guessed.

When support is absent, the extension should detect that through the hydrate request path
(`Method not found` / equivalent unsupported result), cache that capability result, and avoid
showing a misleading pseudo-status.

## Alternatives Considered

### 1. Poll diagnostics-save timeline or observability metrics

Rejected. Those surfaces are retrospective, higher-noise, and semantically wrong for live status.

### 2. Reuse the existing left-side global BSL status bar item

Rejected. The current extension already uses that item for startup/progress/server state and
enriches its tooltip with unrelated sections. Reusing it for file-scoped snapshot readiness would
create ownership races and blur the global-vs-contextual split.

### 3. Show only a status bar item with no detail view

Rejected. Users need a concise answer, but operators still need version/phase/fallback context
without exporting an incident bundle.

### 4. Do VS Code first and ignore `bsl-agent`

Rejected. The runtime truth and vocabulary should be shared now; otherwise the product will
reintroduce conflicting semantics between extension UI and MCP UI.

## Risks / Trade-offs

- Frequent transitions could create status-bar flicker.
- If `shadow_only` or `stale` is labeled as generic "ready", the UI will lie.
- `bsl-agent` and LSP use different revision models, so overfitting on LSP document versions would
  make the shared surface awkward.
- Out-of-order notifications could regress truthfulness if the client does not gate updates.

## Mitigations

- Coalesce notification updates and only emit meaningful state changes.
- Keep `exact` explicit and forbid collapsing `shadow_only` into `ready`.
- Use a shared bounded DTO/vocabulary in `bsl-api-dtos`, with transport-specific revision fields
  where necessary.
- Require monotonic `updatedAtMs` per subject and stale-update dropping on the client.
