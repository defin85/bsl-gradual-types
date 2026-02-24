# Validation Log

## Completed checks
- `openspec validate add-versioned-contract-compatibility-diff-gate --strict --no-interactive` ✅
- `python3 scripts/test-contract-compatibility-diff.py` ✅
- `python3 scripts/check-contract-compatibility-diff.py --baseline-ref master --candidate-root contracts --report /tmp/contracts-compat-master-report.json` ✅

## Attached sample reports
- Non-breaking sample:
  - `openspec/changes/add-versioned-contract-compatibility-diff-gate/validation/non-breaking-sample-report.json`
- Breaking sample:
  - `openspec/changes/add-versioned-contract-compatibility-diff-gate/validation/breaking-sample-report.json`
