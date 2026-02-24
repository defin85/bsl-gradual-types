# Versioned Contracts

`contracts/**` stores versioned public contracts for external surfaces.

Path convention:

- `contracts/<surface>/vN/contract.json`
- `contracts/<surface>/vN/schema.json`
- `contracts/<surface>/vN/changelog.md`

Rules:

- `vN` is a major version.
- Breaking changes require a new major version directory (`vN -> vN+1`).
- Breaking changes require a migration note in `changelog.md`.
- Existing versions are treated as compatibility baselines.

Current baseline surfaces:

- `lsp-completion-v2`
- `observability-completion-v2`

## Compatibility-Diff Gate (manual)

Use the compatibility-diff checker to compare baseline and candidate contracts:

```bash
python3 scripts/check-contract-compatibility-diff.py \
  --baseline-ref master \
  --candidate-root contracts \
  --report artifacts/contracts-compatibility-diff-report.json
```

The report contains:

- `overall.pass`
- per-surface `diff_classification` (`non_breaking` / `breaking`)
- policy violations (for example, `breaking_without_major_bump`, `missing_migration_note`)
