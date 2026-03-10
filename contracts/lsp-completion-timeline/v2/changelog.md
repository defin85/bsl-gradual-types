# lsp-completion-timeline v2

## 2.0.0

- Aligns timeline terminal outcomes with the fail-closed canonical completion cutover.
- Removes legacy semantic substitute outcomes `degraded_incomplete` and `fallback_unavailable` from the public timeline baseline.
- Keeps runtime-visible terminal outcomes for cancellation, supersession, queue rejection, and missing current-revision artifacts.
- Migration note: consumers must stop treating `degraded_incomplete` and `fallback_unavailable` as stable timeline outcomes in `v2` and rely on fail-closed transport semantics instead.
