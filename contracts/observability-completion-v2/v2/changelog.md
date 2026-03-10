# observability-completion-v2 v2

## 2.0.0

- Narrows the authoritative public baseline to canonical-path metrics and fail-closed completion observability.
- Removes legacy public `type_index_*` reason labels and precompute-specific metric families from the compatibility surface.
- Introduces explicit anti-rescue guard counters that must stay zero on authoritative fail-closed fixtures:
  - `intellisense_v2_interactive_stale_served_total`
  - `intellisense_v2_completion_stale_fallback_total`
- Migration note: dashboards and tooling that depended on `type_index_reason_*` or `type_index_precompute_*` metrics must treat them as legacy/internal diagnostics outside the `v2` compatibility baseline.
