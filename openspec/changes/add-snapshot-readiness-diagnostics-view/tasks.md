## 1. Specification
- [x] 1.1 Extend the `bsl-intellisense-v2` snapshot readiness contract with bounded diagnostic detail requirements.
- [x] 1.2 Define VS Code status bar and Observability Tree View behavior for `shadow_only`, `failed`, `stale`, and long-running `building`.
- [x] 1.3 Document that cache dashboard remains workspace/cache-scoped and must not synthesize file snapshot readiness.

## 2. Backend DTO and LSP status
- [x] 2.1 Add schema-v2 optional diagnostic fields to shared snapshot readiness DTOs while preserving existing v1-compatible fields.
- [x] 2.2 Define bounded status/reason/stage vocabularies for diagnostic sections and length-cap free-text failure detail.
- [x] 2.3 Populate bounded reason/artifact/worker/last-failure details from authoritative LSP runtime state.
- [x] 2.4 Preserve monotonic `updatedAtMs`, coalesced notifications, v1/v2 compatibility, and fail-closed behavior for unknown or unsupported details.
- [x] 2.5 Add backend tests for `shadow_only`, `failed`, stale, in-flight, unknown-detail, and legacy v1-compatible states.

## 3. VS Code extension UX
- [x] 3.1 Update TypeScript DTOs and snapshot status cache to accept schema-v2 diagnostic fields.
- [x] 3.2 Add bounded per-URI transition history from accepted snapshot status updates, including eviction/reset behavior.
- [x] 3.3 Expand the status bar tooltip with reason, artifact summary, worker age, and last failure when available.
- [x] 3.4 Change the status bar command to focus/reveal the Snapshot Readiness detail surface with a safe Observability-view fallback.
- [x] 3.5 Expand Observability `Snapshot Readiness` into Summary, Why, Artifacts, Worker, Last Failure, Recent Transitions, and Actions sections.
- [x] 3.6 Ensure status bar/tooltips/tree labels do not include raw source text, full stack traces, or unbounded message text.
- [x] 3.7 Add or update VS Code extension tests for tooltip rendering, Tree View rendering, stale-update rejection, transition history bounds, v1 fallback, and unsupported-server behavior.

## 4. Validation
- [x] 4.1 Run `openspec validate add-snapshot-readiness-diagnostics-view --strict --no-interactive`.
- [x] 4.2 Run targeted backend snapshot-status tests.
- [x] 4.3 Run targeted VS Code extension tests for snapshot status and observability providers.
- [x] 4.4 Run formatting/checks required by touched code paths.
