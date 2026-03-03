# Perf-gate architecture boundary

Date: March 2, 2026

## Goal

Confirm that perf verdict logic is not duplicated inline in `lsp_server` or scripts,
and that dedicated evaluator module is the single source.

## Check

```bash
python3 scripts/check-perf-gate-architecture.py
```

Result:

- `Perf gate architecture check passed.`

## Boundary summary

- Dedicated evaluator module: `backend/src/perf_gate_evaluator.rs`
- Runtime/LSP test path consumes evaluator API (no inline `evaluate_scale_aware_gate` in `core.rs`)
- Harness path (`intellisense_perf`) consumes `evaluate_intellisense_perf_profile`
